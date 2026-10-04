import ExcelJS from 'exceljs';
import { getDriver } from '$lib/db';
import { CURRENT_SCHEMA_VERSION } from '$lib/db/migrations';
import { useSf2WorkbookDir } from '$lib/features/sf2/workbook-files';
import { db, useTestDb } from '$lib/db/repos/__tests__/schema';
import { getFileSystem, MemoryFileSystem, useFileSystem } from '$lib/platform/fs';
import { afterEach, beforeEach, expect } from 'vitest';

/**
 * Every backup test runs against an in-memory disk and an in-memory SQLite.
 *
 * Nothing here touches a real `Documents` folder or a real database file, and
 * nothing needs Tauri. `useSf2WorkbookDir` is the seam that puts the whole
 * `Documents\EES-AMS` tree at a fake path, so the backup folder, the workbook
 * folder and the state file all land in the same memory.
 */

export const ROOT = '/EES-AMS';
export const WORKBOOKS_DIR = `${ROOT}/workbooks`;

/** The slice of the file system the backup tests actually read and write. */
type BackupDisk = Pick<
	MemoryFileSystem,
	'readFile' | 'readTextFile' | 'writeFileAtomic' | 'exists'
>;

/**
 * The tables `repos/__tests__/schema.ts` leaves out that the backup path touches,
 * plus the schema stamp.
 *
 * `repos/__tests__/schema.ts` declares only the columns its own SQL addresses.
 * `wipeAll` empties `sf2_date_mappings` and re-inserts a settings row, so those
 * tables have to exist for a wipe to run at all.
 *
 * The stamp is what keeps `replaceDatabaseFromImage`'s migration step a no-op: a
 * restore runs the chain over whatever it just swapped in, and an un-stamped subset
 * schema would send all twenty-five migrations at tables that are not there. The
 * fixture therefore claims to be a database that is already current, which is
 * what every archive this build writes is.
 */
const EXTRA_SCHEMA = `
CREATE TABLE settings (
	id TEXT PRIMARY KEY NOT NULL,
	day_start TEXT NOT NULL DEFAULT '08:00',
	day_end TEXT NOT NULL DEFAULT '15:00',
	late_after TEXT NOT NULL DEFAULT '08:45',
	quarter TEXT NOT NULL DEFAULT '1st Quarter',
	attendance_mode TEXT NOT NULL DEFAULT 'manual'
);

CREATE TABLE sf2_date_mappings (
	id TEXT PRIMARY KEY NOT NULL,
	class_id TEXT
);

CREATE TABLE sf2_month_templates (
	id TEXT PRIMARY KEY NOT NULL,
	school_year TEXT
);

PRAGMA user_version = ${CURRENT_SCHEMA_VERSION};
`;

/**
 * A fresh memory disk and a fresh database for each test.
 *
 * The disk is re-created in `beforeEach` rather than shared, because a leftover
 * archive from the previous test is indistinguishable from one this test wrote —
 * and the retention and daily-guard assertions all count archives.
 */
export function useBackupFixture(): BackupDisk {
	let fs!: MemoryFileSystem;

	// The shared repo fixture owns the driver, `students`/`classes`/`events` and
	// its own before/after hooks; this only adds the tables it does not declare.
	useTestDb();

	beforeEach(() => {
		fs = new MemoryFileSystem();
		useFileSystem(fs);
		useSf2WorkbookDir(WORKBOOKS_DIR);
		return db().script(EXTRA_SCHEMA);
	});

	afterEach(() => {
		useFileSystem(null);
		useSf2WorkbookDir(null);
	});

	return {
		readFile: (path) => fs.readFile(path),
		readTextFile: (path) => fs.readTextFile(path),
		writeFileAtomic: (path, contents) => fs.writeFileAtomic(path, contents),
		exists: (path) => fs.exists(path)
	};
}

/** One class, two students, one of them absent once — enough to tell rows apart. */
export async function seedAttendance(): Promise<void> {
	const driver = getDriver();
	await driver.execute(
		"INSERT INTO settings (id, day_start, day_end, late_after, quarter, attendance_mode) VALUES ('app', '08:00', '15:00', '08:45', '1st Quarter', 'manual')"
	);
	await driver.execute(
		"INSERT INTO classes (id, name, day_start, day_end, late_after, created_at) VALUES ('c1', 'Grade 3 - Matapat', '08:00', '15:00', '08:45', 1000)"
	);
	await driver.execute(
		"INSERT INTO students (id, name, gender, class_id, created_at) VALUES ('s1', 'Dela Cruz, Juan', 'male', 'c1', 1000)"
	);
	await driver.execute(
		"INSERT INTO students (id, name, gender, class_id, created_at) VALUES ('s2', 'Reyes, Maria', 'female', 'c1', 1001)"
	);
	await driver.execute(
		"INSERT INTO events (id, student_id, class_id, event_type, timestamp) VALUES ('e1', 's1', 'c1', 'absent', 1750000000)"
	);
	await driver.execute(
		"INSERT INTO events (id, student_id, class_id, event_type, timestamp) VALUES ('e2', 's2', 'c1', 'in', 1750000001)"
	);
}

/**
 * A real `.xlsx` in the bound disk, with one mark per day column from `marks`.
 *
 * It has to be a real workbook: `countXmarks` opens it through ExcelJS, and a
 * fixture that was a byte string would prove nothing about the production path.
 */
export async function writeWorkbook(path: string, marks: string[]): Promise<void> {
	const workbook = new ExcelJS.Workbook();
	const sheet = workbook.addWorksheet('SEPTEMBER 2026');
	sheet.getCell('A1').value = 'School Form 2 (SF2)';
	sheet.getCell('A8').value = '1';
	sheet.getCell('C8').value = 'Dela Cruz, Juan';
	sheet.getCell('A9').value = '2';
	sheet.getCell('C9').value = 'Reyes, Maria';
	marks.forEach((mark, index) => {
		sheet.getCell(8, 6 + index).value = mark;
	});
	const buffer = await workbook.xlsx.writeBuffer();
	await getFileSystem().writeFileAtomic(path, new Uint8Array(buffer));
}

/**
 * Assert that something rejected with an `AppError` whose detail matches.
 *
 * The repo layer rejects with `{ kind, detail }` objects, not `Error`s, so
 * vitest's `rejects.toThrow(/…/)` matches an empty string and every message
 * assertion silently passes.
 */
export async function expectAppError(work: Promise<unknown>, pattern: RegExp): Promise<void> {
	try {
		await work;
	} catch (thrown) {
		const detail =
			typeof thrown === 'object' && thrown !== null && 'detail' in thrown
				? String((thrown as { detail: unknown }).detail)
				: String(thrown);
		expect(detail).toMatch(pattern);
		return;
	}
	throw new Error(`expected a rejection matching ${pattern}, but it resolved`);
}

/** {@link expectAppError} for a synchronous throw. */
export function expectAppErrorSync(work: () => unknown, pattern: RegExp): void {
	try {
		work();
	} catch (thrown) {
		const detail =
			typeof thrown === 'object' && thrown !== null && 'detail' in thrown
				? String((thrown as { detail: unknown }).detail)
				: String(thrown);
		expect(detail).toMatch(pattern);
		return;
	}
	throw new Error(`expected a throw matching ${pattern}, but it returned`);
}
