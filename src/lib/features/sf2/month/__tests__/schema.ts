import { useDriver, type SqlDriver } from '$lib/db';
import { NodeSqlDriver } from '$lib/db/node-driver';
import { afterEach, beforeEach } from 'vitest';

/**
 * The month tables these tests need, in the shape the v19-v24 migration chain
 * leaves them in.
 *
 * The real migration runner belongs to the DB layer (D9 keeps the chain
 * verbatim), so this declares only the columns the month SQL touches. It reuses
 * the shared repo fixture for `classes`, `students` and `events` rather than
 * redeclaring them, because a second definition of those columns is how a repo
 * test starts passing against a schema the app does not have.
 */
const MONTH_SCHEMA = `
CREATE TABLE students (
	id TEXT PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	card_serial TEXT UNIQUE,
	class_id TEXT,
	created_at INTEGER NOT NULL,
	gender TEXT,
	sf2_learner_id TEXT
);

CREATE TABLE classes (
	id TEXT PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	day_start TEXT NOT NULL,
	day_end TEXT NOT NULL,
	late_after TEXT NOT NULL,
	created_at INTEGER NOT NULL,
	room TEXT NOT NULL DEFAULT 'N/A',
	sessions TEXT,
	days TEXT
);

CREATE TABLE events (
	id TEXT PRIMARY KEY NOT NULL,
	student_id TEXT NOT NULL,
	class_id TEXT,
	event_type TEXT NOT NULL,
	timestamp INTEGER NOT NULL,
	note TEXT,
	session_key TEXT,
	override_reason TEXT,
	updated_at INTEGER
);

CREATE TABLE settings (
	id TEXT PRIMARY KEY NOT NULL,
	day_start TEXT NOT NULL,
	day_end TEXT NOT NULL,
	late_after TEXT NOT NULL,
	quarter TEXT NOT NULL DEFAULT '1st Quarter',
	q1_start TEXT,
	q1_end TEXT,
	q2_start TEXT,
	q2_end TEXT,
	q3_start TEXT,
	q3_end TEXT,
	attendance_mode TEXT NOT NULL DEFAULT 'manual',
	school_id TEXT,
	school_name TEXT,
	school_year TEXT,
	report_month TEXT,
	grade_level TEXT,
	section TEXT,
	adviser_name TEXT,
	school_head_name TEXT,
	school_start_date TEXT,
	last_report_month TEXT,
	sf2_split_completed_at INTEGER,
	branding_title TEXT DEFAULT 'EES AMS'
);

CREATE TABLE sf2_month_templates (
	id TEXT PRIMARY KEY NOT NULL,
	active_class_id TEXT NOT NULL,
	school_year TEXT NOT NULL,
	report_month TEXT NOT NULL,
	report_year INTEGER NOT NULL,
	source_path TEXT NOT NULL,
	source_hash TEXT NOT NULL,
	school_id TEXT,
	school_name TEXT,
	grade_level TEXT,
	section TEXT,
	adviser_name TEXT,
	school_head_name TEXT,
	first_school_day INTEGER NOT NULL,
	first_school_day_override INTEGER,
	imported_at INTEGER NOT NULL,
	last_synced_at INTEGER,
	workbook_x_count INTEGER NOT NULL DEFAULT 0,
	workbook_scanned_at INTEGER,
	UNIQUE(active_class_id, school_year, report_month)
);

CREATE TABLE sf2_month_student_mappings (
	template_id TEXT NOT NULL,
	student_id TEXT NOT NULL,
	workbook_name TEXT NOT NULL,
	normalized_name TEXT NOT NULL,
	row_index INTEGER NOT NULL,
	gender_block TEXT,
	sf2_learner_id TEXT,
	PRIMARY KEY(template_id, student_id),
	UNIQUE(template_id, normalized_name)
);

CREATE TABLE sf2_month_date_mappings (
	template_id TEXT NOT NULL,
	date TEXT NOT NULL,
	column_letter TEXT NOT NULL,
	column_index INTEGER NOT NULL,
	sheet_name TEXT,
	PRIMARY KEY(template_id, date)
);

CREATE TABLE sf2_templates (
	id TEXT PRIMARY KEY NOT NULL,
	source_path TEXT NOT NULL,
	source_hash TEXT NOT NULL DEFAULT '',
	school_id TEXT,
	school_name TEXT,
	school_year TEXT,
	report_month TEXT,
	grade_level TEXT,
	section TEXT,
	adviser_name TEXT,
	school_head_name TEXT,
	layout_fingerprint TEXT,
	active_class_id TEXT,
	imported_at INTEGER NOT NULL,
	last_synced_at INTEGER
);

CREATE TABLE sf2_student_mappings (
	template_id TEXT NOT NULL,
	student_id TEXT NOT NULL,
	workbook_name TEXT NOT NULL,
	normalized_name TEXT NOT NULL,
	row_index INTEGER NOT NULL,
	gender_block TEXT
);

CREATE TABLE sf2_date_mappings (
	template_id TEXT NOT NULL,
	sheet_name TEXT,
	date TEXT NOT NULL,
	column_letter TEXT NOT NULL,
	column_index INTEGER NOT NULL
);
`;

let driver: SqlDriver;

export function useMonthTestDb(): void {
	beforeEach(async () => {
		driver = new NodeSqlDriver();
		await driver.script(MONTH_SCHEMA);
		useDriver(driver);
	});

	afterEach(async () => {
		useDriver(null);
		await driver.close();
	});
}

/** The driver the current test is running against, for direct SQL assertions. */
export function db(): SqlDriver {
	return driver;
}

/** A month row with only the columns every read needs set. */
export function monthTemplate(overrides: {
	id?: string;
	classId?: string;
	schoolYear?: string;
	reportMonth?: string;
	reportYear?: number;
	sourcePath?: string;
	firstSchoolDay?: number;
}): Record<string, string | number | null> {
	return {
		id: overrides.id ?? 'month-1',
		active_class_id: overrides.classId ?? 'class-1',
		school_year: overrides.schoolYear ?? '2026-2027',
		report_month: overrides.reportMonth ?? 'SEPTEMBER',
		report_year: overrides.reportYear ?? 2026,
		source_path: overrides.sourcePath ?? 'C:/workbooks/SF2-GRADE-3.xls',
		source_hash: 'hash-1',
		first_school_day: overrides.firstSchoolDay ?? 1,
		imported_at: 1_700_000_000
	};
}

export async function insertMonthTemplate(
	overrides: Parameters<typeof monthTemplate>[0]
): Promise<void> {
	const values = monthTemplate(overrides);
	const keys = Object.keys(values);
	await db().execute(
		`INSERT INTO sf2_month_templates (${keys.join(', ')})
		 VALUES (${keys.map(() => '?').join(', ')})`,
		Object.values(values)
	);
}
