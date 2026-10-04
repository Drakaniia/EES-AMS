import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
	chooseRestoreBackup,
	createBackupNow,
	exportAll,
	exportCsvWithFolder,
	exportDatabase,
	exportJsonWithFolder,
	getBackupStatus,
	importAll,
	listBackups,
	restoreBackup,
	wipeAll
} from '$lib/api/backup';
import { addEvent } from '$lib/api/events';
import { getFileSystem } from '$lib/platform/fs';
import {
	BACKUPS_DIR,
	CLASS_ID,
	EXPORTS_DIR,
	ROOT,
	db,
	seedRoster,
	seedSettings,
	useApiFixture
} from './fixture';

/** The shell-only picker, reduced to the one function `pickers.ts` calls. */
const dialog = vi.hoisted(() => ({ open: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: dialog.open }));

useApiFixture();

const STAGING_DIR = `${ROOT}/restore-staging`;
/** Somewhere the fs capability in `default.json` does not reach. */
const USB_ARCHIVE = '/media/E (EES-AMS)/EES-AMS-20260928-142530.zip';

/**
 * A real archive, written outside the app's own folder.
 *
 * The point is that the bytes come out of `createBackupNow()` rather than a
 * hand-built zip, so the restore under test is the production one — only the
 * path the teacher picked it from is different.
 */
async function backupOnUsbStick(): Promise<void> {
	await seedAbsence();
	await createBackupNow();
	const created = (await listBackups())[0];
	const fs = getFileSystem();
	await fs.writeFileAtomic(USB_ARCHIVE, await fs.readFile(created?.path as string));
	await db().execute('DELETE FROM events');
}

/** How many absences the live database holds. */
async function absenceCount(): Promise<number> {
	const rows = await db().query<{ n: number }>(
		"SELECT COUNT(*) AS n FROM events WHERE event_type = 'absent'"
	);
	return Number(rows[0]?.n ?? 0);
}

async function seedAbsence(): Promise<void> {
	await seedRoster();
	await seedSettings();
	await addEvent({ studentId: 's1', classId: 'class-1', type: 'absent' });
	await db().execute(`UPDATE events SET timestamp = ? WHERE id = (SELECT id FROM events)`, [
		Math.floor(new Date(2026, 9, 2, 8, 55).getTime() / 1000)
	]);
}

describe('exportAll', () => {
	it('snapshots the rows, absences included', async () => {
		await seedAbsence();

		const snapshot = await exportAll();

		expect(snapshot.students.map((student) => student.name)).toEqual([
			'Bautista, Ana',
			'Dela Cruz, Juan',
			'Reyes, Maria'
		]);
		expect(snapshot.classes.map((cls) => cls.id)).toEqual(['class-1']);
		expect(snapshot.events).toHaveLength(1);
		expect(snapshot.events[0]?.type).toBe('absent');
		expect(snapshot.settings[0]?.id).toBe('app');
		expect(snapshot.exportedAt).toBeGreaterThan(0);
	});
});

describe('importAll', () => {
	it('round-trips a snapshot: wipe, import, and the absence is still an absence', async () => {
		await seedAbsence();
		const snapshot = await exportAll();

		await db().execute('DELETE FROM events');
		await importAll(snapshot);

		const rows = await db().query<{ event_type: string }>('SELECT event_type FROM events');
		expect(rows.map((row) => row.event_type)).toEqual(['absent']);
	});
});

describe('exportJsonWithFolder', () => {
	it('writes a readable snapshot under the exports folder and returns its path', async () => {
		await seedAbsence();

		const path = await exportJsonWithFolder();

		expect(path.startsWith(`${EXPORTS_DIR}/ees-ams-`)).toBe(true);
		expect(path.endsWith('.json')).toBe(true);

		const written = JSON.parse(await getFileSystem().readTextFile(path)) as {
			students: { name: string }[];
		};
		expect(written.students).toHaveLength(3);
	});
});

describe('exportCsvWithFolder', () => {
	it('writes one row per check-in with the late verdict against the class threshold', async () => {
		await seedAbsence();
		await addEvent({ studentId: 's2', classId: 'class-1', type: 'in' });
		await db().execute(`UPDATE events SET timestamp = ? WHERE student_id = 's2'`, [
			Math.floor(new Date(2026, 9, 2, 8, 55).getTime() / 1000)
		]);
		const snapshot = await exportAll();

		const path = await exportCsvWithFolder(
			snapshot.events,
			snapshot.students,
			snapshot.classes,
			'08:45'
		);

		expect(path.startsWith(`${EXPORTS_DIR}/attendance-`)).toBe(true);
		const csv = await getFileSystem().readTextFile(path);
		expect(csv.split('\n')[0]).toBe('Date,Class,Room,Name,IN,Late');
		// The absent mark is not a check-in, so it must not claim a row.
		expect(csv).toContain('2026-10-02,Grade 3 - Matapat,Room 1,"Reyes, Maria",08:55,Yes');
	});
});

describe('exportDatabase', () => {
	it('writes a SQL dump that carries the schema stamp and one insert per row', async () => {
		await seedAbsence();

		const path = await exportDatabase();
		const dump = await getFileSystem().readTextFile(path);

		expect(path.startsWith(`${EXPORTS_DIR}/attendance-`)).toBe(true);
		expect(path.endsWith('.sql')).toBe(true);
		expect(dump).toMatch(/^PRAGMA user_version = 25;$/m);
		expect(dump).toContain(`INSERT INTO "students" VALUES`);
		expect(dump).toContain(`INSERT INTO "classes" VALUES`);
	});
});

describe('createBackupNow', () => {
	it('leaves one archive behind and reports it in the status', async () => {
		await seedAbsence();

		const status = await createBackupNow();

		expect(status.backupCount).toBe(1);
		expect(status.localBackupDir).toBe(BACKUPS_DIR);
		const backups = await listBackups();
		expect(backups).toHaveLength(1);
		expect(backups[0]).toMatchObject({ kind: 'manual', includesDatabase: true });
		expect(backups[0]?.fileName.endsWith('.zip')).toBe(true);
	});

	it('getBackupStatus on its own reports an empty backup folder', async () => {
		expect(await getBackupStatus()).toMatchObject({ backupCount: 0, localBackupDir: BACKUPS_DIR });
	});
});

describe('chooseRestoreBackup', () => {
	beforeEach(() => {
		dialog.open.mockReset();
	});

	it('stages a backup picked from outside the app folder and restores it from there', async () => {
		await backupOnUsbStick();
		dialog.open.mockResolvedValue(USB_ARCHIVE);

		const preview = await chooseRestoreBackup();

		// The path handed downstream is inside the capability's allow-list, not the
		// one the teacher picked, and it is a copy rather than the original file.
		expect(preview?.sourcePath.startsWith(`${STAGING_DIR}/`)).toBe(true);
		expect(preview?.sourcePath.endsWith('EES-AMS-20260928-142530.zip')).toBe(true);
		expect(preview?.sourcePath).not.toBe(USB_ARCHIVE);
		expect(preview).toMatchObject({ studentCount: 3, classCount: 1, absentCount: 1 });
		expect(await absenceCount()).toBe(0);

		const result = await restoreBackup(preview?.sourcePath as string);

		expect(result.restoredPath).toBe(preview?.sourcePath);
		expect(result.preRestoreBackupPath.startsWith(`${BACKUPS_DIR}/`)).toBe(true);
		expect(await absenceCount()).toBe(1);
		// The staged copy has done its job; leaving it would put an invisible
		// duplicate of the teacher's backup in their Documents folder forever.
		expect(await getFileSystem().exists(preview?.sourcePath as string)).toBe(false);
		// The archive they picked from is untouched.
		expect(await getFileSystem().exists(USB_ARCHIVE)).toBe(true);
	});

	it('returns null and stages nothing when the picker is dismissed', async () => {
		dialog.open.mockResolvedValue(null);

		expect(await chooseRestoreBackup()).toBeNull();
		expect(await getFileSystem().exists(STAGING_DIR)).toBe(false);
	});

	it('overwrites the staged copy when a second pick has the same name', async () => {
		const fs = getFileSystem();
		await seedAbsence();
		await createBackupNow();
		const firstPath = (await listBackups())[0]?.path as string;
		const firstBytes = await fs.readFile(firstPath);
		// Removed so the second `createBackupNow` gets its own file name; what the
		// test needs is two *valid* archives that agree on a name.
		await fs.remove(firstPath);
		await addEvent({ studentId: 's2', classId: CLASS_ID, type: 'absent' });
		await createBackupNow();
		const secondBytes = await fs.readFile((await listBackups())[0]?.path as string);

		// Same name, different folder, different contents: one absence vs two.
		const otherFolder = '/media/D (backup)/EES-AMS-20260928-142530.zip';
		await fs.writeFileAtomic(USB_ARCHIVE, firstBytes);
		await fs.writeFileAtomic(otherFolder, secondBytes);

		dialog.open.mockResolvedValueOnce(USB_ARCHIVE);
		const first = await chooseRestoreBackup();
		dialog.open.mockResolvedValueOnce(otherFolder);
		const second = await chooseRestoreBackup();

		// One staging slot, so both picks land on the same path — and the slot holds
		// the second archive whole, never a blend of the two.
		expect(first?.sourcePath).toBe(second?.sourcePath);
		expect(await fs.readFile(second?.sourcePath as string)).toEqual(secondBytes);
		expect(await fs.readFile(second?.sourcePath as string)).not.toEqual(firstBytes);

		await restoreBackup(second?.sourcePath as string);
		expect(await absenceCount()).toBe(2);
	});

	it('sweeps a staged file left behind by a crash, which nothing can refer to', async () => {
		const fs = getFileSystem();
		const orphan = `${STAGING_DIR}/restore-deadbeef-EES-AMS-20260928-142530.zip`;
		await fs.writeFileAtomic(orphan, 'crashed-mid-restore');
		await backupOnUsbStick();
		dialog.open.mockResolvedValue(USB_ARCHIVE);

		await chooseRestoreBackup();

		expect(await fs.exists(orphan)).toBe(false);
	});
});

describe('restoreBackup', () => {
	it('leaves an archive that was not staged where it is', async () => {
		await seedAbsence();
		await createBackupNow();
		const created = (await listBackups())[0];

		await restoreBackup(created?.path as string);

		// The sweep and the cleanup both key off the staging folder, so a real
		// backup in `backups\` can never be mistaken for one of ours.
		expect(await getFileSystem().exists(created?.path as string)).toBe(true);
	});
});

describe('wipeAll', () => {
	it('empties the tables it claims and says where the safety copy landed', async () => {
		await seedAbsence();

		const outcome = await wipeAll();

		expect(outcome).toMatchObject({ deletedStudents: 3, deletedClasses: 1, deletedEvents: 1 });
		expect(outcome.preWipeBackupPath?.startsWith(`${BACKUPS_DIR}/`)).toBe(true);
		expect(await db().query('SELECT * FROM students')).toEqual([]);
		expect(await db().query('SELECT * FROM events')).toEqual([]);
		expect(await db().query('SELECT * FROM classes')).toEqual([]);
		// The audit trail deliberately survives a wipe.
		expect(await listBackups()).toHaveLength(1);
	});
});
