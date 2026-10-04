import { CURRENT_SCHEMA_VERSION } from '$lib/db/migrations';
import { db, insertMonthTemplate, useMonthTestDb } from '$lib/features/sf2/month/__tests__/schema';
import { sf2MonthName } from '$lib/features/sf2/calendar';
import { registerSf2Preview } from '$lib/features/sf2/preview';
import { useSf2WorkbookDir } from '$lib/features/sf2/workbook-files';
import { MemoryFileSystem, getFileSystem, useFileSystem } from '$lib/platform/fs';
import { afterEach, beforeEach } from 'vitest';

/**
 * The whole `$lib/api` fixture: an in-memory SQLite, an in-memory disk, and a
 * bound month grid builder.
 *
 * Nothing here touches a real `Documents` folder, a real database file, Tauri or
 * Excel. `useSf2WorkbookDir` is the seam that puts the whole `Documents\EES-AMS`
 * tree at a fake path, so the backup folder, the exports folder and the workbook
 * folder all land in the same memory.
 *
 * The base schema is the month fixture rather than the repo fixture because the
 * SF2 adapters are the ones that need `sf2_month_templates` in full, and a test
 * that redeclares a table the production chain does not is a test that passes
 * against a schema the app does not have.
 */

export const ROOT = '/EES-AMS';
export const WORKBOOKS_DIR = `${ROOT}/workbooks`;
export const WORKBOOK_PATH = `${WORKBOOKS_DIR}/SF2-GRADE-3-MATAPAT.xlsx`;
export const EXPORTS_DIR = `${ROOT}/exports`;
export const BACKUPS_DIR = `${ROOT}/backups`;

export const CLASS_ID = 'class-1';
export const SCHOOL_YEAR = '2026-2027';

/** The columns the month fixture leaves out that `$lib/api` and a wipe touch. */
const EXTRA_SCHEMA = `
CREATE TABLE audit_events (
	id TEXT PRIMARY KEY NOT NULL,
	entity_type TEXT NOT NULL,
	entity_id TEXT,
	action TEXT NOT NULL,
	summary TEXT NOT NULL,
	before_json TEXT,
	after_json TEXT,
	metadata_json TEXT,
	created_at INTEGER NOT NULL,
	actor TEXT NOT NULL DEFAULT 'admin'
);

CREATE TABLE attendance_event_audit (
	id TEXT PRIMARY KEY NOT NULL,
	event_id TEXT,
	student_id TEXT NOT NULL,
	class_id TEXT,
	session_key TEXT,
	action TEXT NOT NULL CHECK(action IN ('create_override', 'update', 'delete')),
	reason TEXT NOT NULL,
	before_json TEXT,
	after_json TEXT,
	created_at INTEGER NOT NULL,
	actor TEXT NOT NULL DEFAULT 'admin'
);

CREATE TABLE attendance_day_status (
	id TEXT PRIMARY KEY NOT NULL,
	class_id TEXT
);

ALTER TABLE settings ADD COLUMN branding_logo BLOB;

PRAGMA user_version = ${CURRENT_SCHEMA_VERSION};
`;

let disk!: MemoryFileSystem;

export function useApiFixture(): void {
	useMonthTestDb();
	registerSf2Preview();

	beforeEach(async () => {
		disk = new MemoryFileSystem();
		useFileSystem(disk);
		useSf2WorkbookDir(WORKBOOKS_DIR);
		await db().script(EXTRA_SCHEMA);
	});

	afterEach(() => {
		useFileSystem(null);
		useSf2WorkbookDir(null);
	});
}

export function memory(): MemoryFileSystem {
	return disk;
}

/** The driver the current test runs against, for direct SQL assertions. */
export { db, insertMonthTemplate } from '$lib/features/sf2/month/__tests__/schema';

// ── Seeds ────────────────────────────────────────────────────────────────────

export async function seedRoster(): Promise<void> {
	await db().execute(
		`INSERT INTO classes (id, name, room, day_start, day_end, late_after, sessions, days, created_at)
		 VALUES (?, 'Grade 3 - Matapat', 'Room 1', '08:00', '15:00', '08:45', '[]', '[1,2,3,4,5]', 1000)`,
		[CLASS_ID]
	);
	await db().execute(
		`INSERT INTO students (id, name, gender, class_id, created_at)
		 VALUES ('s1', 'Dela Cruz, Juan', 'male', ?, 1000),
		        ('s2', 'Reyes, Maria', 'female', ?, 1001),
		        ('s3', 'Bautista, Ana', 'female', ?, 1002)`,
		[CLASS_ID, CLASS_ID, CLASS_ID]
	);
}

/** The month name the app opens on today, so a launch resolves with no fallback. */
export function currentMonth(): string {
	return sf2MonthName(new Date().getMonth() + 1);
}

/**
 * A month row, its roster and its day grid, plus the workbook file the grid read
 * stats. The workbook is a byte string on purpose: a grid read never opens it, and
 * a real `.xlsx` here would only prove ExcelJS is installed.
 */
export async function seedMonth(): Promise<string[]> {
	const month = currentMonth();
	await insertMonthTemplate({
		classId: CLASS_ID,
		schoolYear: SCHOOL_YEAR,
		reportMonth: month,
		reportYear: new Date().getFullYear(),
		sourcePath: WORKBOOK_PATH
	});

	const days = schoolDaysOfCurrentMonth();
	await db().execute(
		`INSERT INTO sf2_month_student_mappings (template_id, student_id, workbook_name, normalized_name, row_index, gender_block)
		 VALUES ('month-1', 's1', 'Dela Cruz, Juan', 'dela cruz juan', 8, 'MALE'),
		        ('month-1', 's2', 'Reyes, Maria', 'reyes maria', 9, 'FEMALE'),
		        ('month-1', 's3', 'Bautista, Ana', 'bautista ana', 0, NULL)`
	);
	for (const [index, date] of days.entries()) {
		await db().execute(
			`INSERT INTO sf2_month_date_mappings (template_id, date, column_letter, column_index)
			 VALUES ('month-1', ?, ?, ?)`,
			[date, String.fromCharCode(70 + index), 6 + index]
		);
	}

	await getFileSystem().writeFileAtomic(WORKBOOK_PATH, 'workbook-bytes');
	return days;
}

/** The first five weekdays of the current month, as `YYYY-MM-DD`. */
function schoolDaysOfCurrentMonth(): string[] {
	const now = new Date();
	const month = now.getMonth() + 1;
	const year = now.getFullYear();
	const lastDay = new Date(year, month, 0).getDate();
	const days: string[] = [];
	for (let day = 1; day <= lastDay && days.length < 5; day += 1) {
		const weekday = new Date(year, month - 1, day).getDay();
		if (weekday === 0 || weekday === 6) continue;
		days.push(
			`${String(year).padStart(4, '0')}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`
		);
	}
	return days;
}

/** The single settings row every read of the app's configuration goes through. */
export async function seedSettings(): Promise<void> {
	await db().execute(
		`INSERT INTO settings (id, day_start, day_end, late_after, quarter, attendance_mode, school_year, branding_title)
		 VALUES ('app', '08:00', '15:00', '08:45', '1st Quarter', 'manual', ?, 'EES AMS')`,
		[SCHOOL_YEAR]
	);
}
