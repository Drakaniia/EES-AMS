import { describe, expect, it } from 'vitest';
import { getDriver } from '$lib/db';
import { wipeAll } from '$lib/db/repos/transfer';
import { createBackup } from '../create';
import { listBackups, previewBackup } from '../list';
import { restoreBackup } from '../restore';
import { useBackupFixture, seedAttendance, writeWorkbook, WORKBOOKS_DIR } from './fixture';

/**
 * The round trip the whole subsystem exists for: backup → destroy everything →
 * restore → identical state.
 *
 * D16 says the Rust suite is not ported and parity is not a goal, so this is the
 * one test that has to earn the port. It is also the only test that touches every
 * layer at once — the zip, the manifest, the database image, the migration runner,
 * the workbook tree and the safety backup — which is why it is written as one story
 * rather than as four.
 */

const NOW = Date.parse('2026-09-28T14:25:30Z');
const WORKBOOK = `${WORKBOOKS_DIR}/SF2-GRADE-3-MATAPAT.xlsx`;

describe('backup → wipe → restore', () => {
	const fs = useBackupFixture();

	it('returns the app to the state the backup recorded', async () => {
		await seedAttendance();
		await writeWorkbook(WORKBOOK, ['X', '', 'X']);
		const before = await snapshot();
		const workbookBytes = await fs.readFile(WORKBOOK);

		const backup = await createBackup('manual', NOW);
		expect(backup).toMatchObject({ includesDatabase: true, workbookCount: 1, totalXCount: 2 });

		// Destroy both halves: the rows, and the workbook on disk.
		await wipeAll();
		await fs.writeFileAtomic(WORKBOOK, new Uint8Array([1, 2, 3]));
		expect(await snapshot()).toEqual({ students: 0, classes: 0, events: 0, absences: 0 });

		const result = await restoreBackup(backup.path);

		expect(result.workbooksRestored).toBe(true);
		expect(result.preRestoreBackupPath).not.toBe(backup.path);
		expect(await snapshot()).toEqual(before);
		expect(await fs.readFile(WORKBOOK)).toEqual(workbookBytes);
	});

	it('takes a pre-restore safety backup of the state it is about to overwrite', async () => {
		await seedAttendance();
		const backup = await createBackup('manual', NOW);

		await restoreBackup(backup.path);

		const safety = (await listBackups()).find((entry) => entry.kind === 'pre_restore');
		expect(safety).toBeDefined();
		expect(await previewBackup(safety?.path as string)).toMatchObject({
			studentCount: 2,
			absentCount: 1
		});
	});
});

describe('preview', () => {
	useBackupFixture();

	it('reports the row counts the manifest recorded', async () => {
		await seedAttendance();
		await writeWorkbook(WORKBOOK, ['X', '', 'X']);

		const preview = await previewBackup((await createBackup('manual', NOW)).path);

		expect(preview).toMatchObject({
			studentCount: 2,
			classCount: 1,
			eventCount: 2,
			absentCount: 1,
			includesDatabase: true
		});
		expect(preview.workbooks).toEqual([
			{
				fileName: 'SF2-GRADE-3-MATAPAT.xlsx',
				relativePath: 'workbooks/SF2-GRADE-3-MATAPAT.xlsx',
				bytes: expect.any(Number),
				xCount: 2
			}
		]);
	});

	it('warns when the workbooks hold more absences than the database', async () => {
		await seedAttendance();
		await writeWorkbook(WORKBOOK, ['X', '', 'X']);

		const preview = await previewBackup((await createBackup('manual', NOW)).path);

		// The guard that exists because a restore makes the database match the
		// archive exactly: afterwards the workbook is ahead of it.
		expect(preview.warnings).toHaveLength(1);
		expect(preview.warnings[0]).toContain('record 2 X mark(s) but its database holds only 1');
	});

	it('says so, rather than warning, when the archive carries no workbooks', async () => {
		await seedAttendance();

		const preview = await previewBackup((await createBackup('manual', NOW)).path);

		expect(preview.warnings).toEqual([
			'This backup holds no SF2 workbooks. Restoring it leaves the current SF2 workbooks untouched.'
		]);
	});
});

/** What the app actually shows after a restore: who is enrolled and who was absent. */
async function snapshot(): Promise<{
	students: number;
	classes: number;
	events: number;
	absences: number;
}> {
	const driver = getDriver();
	const total = async (sql: string): Promise<number> =>
		Number((await driver.queryOne<{ n: number }>(sql))?.n ?? 0);
	return {
		students: await total('SELECT COUNT(*) AS n FROM students'),
		classes: await total('SELECT COUNT(*) AS n FROM classes'),
		events: await total('SELECT COUNT(*) AS n FROM events'),
		absences: await total("SELECT COUNT(*) AS n FROM events WHERE event_type = 'absent'")
	};
}
