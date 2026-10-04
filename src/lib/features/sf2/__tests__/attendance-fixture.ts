/**
 * The tables the attendance tests touch, in the shape the migration chain leaves
 * them in.
 *
 * `month/__tests__/schema.ts` declares most of this, but it is not used here: that
 * fixture has no `audit_events` table, and every write on this path records an
 * audit row — a class the tests seed, a mark the grid corrects, a workbook import.
 * Declaring the tables locally keeps the fixture honest rather than reaching into
 * it.
 */
import { useDriver, type SqlDriver } from '$lib/db';
import { NodeSqlDriver } from '$lib/db/node-driver';
import { afterEach, beforeEach } from 'vitest';
import { createClass } from '$lib/db/repos/classes';
import { createStudent } from '$lib/db/repos/students';
import type { Sf2MonthDateMappingRecord } from '$lib/features/sf2/month/templates';
import type { Sf2MonthStudentMapping } from '$lib/features/sf2/month/students';
import { TEMPLATE_SHEETS } from '$lib/features/excel/__tests__/template-fixture';

const ATTENDANCE_SCHEMA = `
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

CREATE TABLE settings (
	id TEXT PRIMARY KEY NOT NULL,
	day_start TEXT NOT NULL,
	day_end TEXT NOT NULL,
	late_after TEXT NOT NULL,
	quarter TEXT NOT NULL DEFAULT '1st Quarter',
	attendance_mode TEXT NOT NULL DEFAULT 'manual',
	school_year TEXT
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
	PRIMARY KEY(template_id, student_id)
);

CREATE TABLE sf2_month_date_mappings (
	template_id TEXT NOT NULL,
	date TEXT NOT NULL,
	column_letter TEXT NOT NULL,
	column_index INTEGER NOT NULL,
	sheet_name TEXT,
	PRIMARY KEY(template_id, date)
);
`;

let driver: SqlDriver;

export function useAttendanceTestDb(): void {
	beforeEach(async () => {
		driver = new NodeSqlDriver();
		await driver.script(ATTENDANCE_SCHEMA);
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

/** A month row with only the columns every read on this path needs set. */
export async function insertMonthTemplate(
	overrides: {
		id?: string;
		classId?: string;
		schoolYear?: string;
		reportMonth?: string;
		reportYear?: number;
		sourcePath?: string;
		firstSchoolDay?: number;
	} = {}
): Promise<void> {
	await driver.execute(
		`INSERT INTO sf2_month_templates
		   (id, active_class_id, school_year, report_month, report_year, source_path,
		    source_hash, first_school_day, imported_at)
		 VALUES (?, ?, ?, ?, ?, ?, 'hash-1', ?, 1700000000)`,
		[
			overrides.id ?? 'month-1',
			overrides.classId ?? 'class-1',
			overrides.schoolYear ?? '2026-2027',
			overrides.reportMonth ?? 'SEPTEMBER',
			overrides.reportYear ?? 2026,
			overrides.sourcePath ?? 'C:/workbooks/SF2-GRADE-3-MATAPAT.xlsx',
			overrides.firstSchoolDay ?? 1
		]
	);
}

// ── The data the tests are written in ──────────────────────────────────────────

/** The template's own first worksheet, which is a real month sheet. */
export const SHEET = TEMPLATE_SHEETS[0];

/** One learner row of a month, as the roster tables hold it. */
export function mapping(studentId: string, rowIndex: number): Sf2MonthStudentMapping {
	return {
		templateId: 'month-1',
		studentId,
		workbookName: `LEARNER ${rowIndex}`,
		normalizedName: `LEARNER ${rowIndex}`,
		rowIndex,
		genderBlock: rowIndex < 29 ? 'MALE' : 'FEMALE'
	};
}

/** One day column on {@link SHEET}, as `sf2_month_date_mappings` holds it. */
export function date(
	day: string,
	columnLetter: string,
	columnIndex: number
): Sf2MonthDateMappingRecord {
	return {
		templateId: 'month-1',
		date: `2026-09-${day}`,
		columnLetter,
		columnIndex,
		sheetName: SHEET
	};
}

/**
 * The month the service tests are about: `JUNE 2025`, the template's own first
 * worksheet, so a write lands on a sheet that exists in the real file.
 *
 * Every date a service test records or asserts on has to be one of these, because
 * a month read through `resolveMonthWriteContext` writes only its own mapped days.
 */
export const MONTH = 'JUNE';
export const SCHOOL_YEAR = '2025-2026';
export const SEEDED_DAYS: Sf2MonthDateMappingRecord[] = [
	{
		templateId: 'month-1',
		date: '2025-06-02',
		columnLetter: 'H',
		columnIndex: 8,
		sheetName: SHEET
	},
	{ templateId: 'month-1', date: '2025-06-03', columnLetter: 'I', columnIndex: 9, sheetName: SHEET }
];
export const DAY_ONE = SEEDED_DAYS[0].date;
export const DAY_TWO = SEEDED_DAYS[1].date;

/** A class with one learner, and the ids the tests assert on. */
export async function seedClass(): Promise<{ classId: string; firstId: string }> {
	const cls = await createClass({
		name: 'Grade 3 - Matapat',
		room: 'Room 1',
		dayStart: '07:30',
		dayEnd: '15:00',
		lateAfter: '08:00',
		sessions: [],
		days: []
	});
	const first = await createStudent({ classId: cls.id, name: 'Dela Cruz, Juan' });
	return { classId: cls.id, firstId: first.id };
}

/**
 * A month row, its roster and its day grid, all pointing at the workbook at `path`.
 *
 * Everything the service reads is seeded here rather than reached for, so a failure
 * points at the service rather than at a fixture three layers down.
 */
export async function seedMonth(params: { classId: string; path: string }): Promise<void> {
	await insertMonthTemplate({
		classId: params.classId,
		schoolYear: SCHOOL_YEAR,
		reportMonth: MONTH,
		reportYear: 2025,
		sourcePath: params.path,
		// JUNE 2025 opens on a Sunday; the first school day is Monday the 2nd.
		firstSchoolDay: 2
	});
	const stored = await driver.queryOne<{ id: string }>(
		'SELECT id FROM sf2_month_templates WHERE active_class_id = ?',
		[params.classId]
	);
	// Non-null because `insertMonthTemplate` just wrote the row; an `undefined` here
	// would be a silent no-op insert rather than a failure.
	const templateId = stored?.id ?? '';
	for (const learner of [mapping('s1', 8), mapping('s2', 30)]) {
		await driver.execute(
			`INSERT INTO sf2_month_student_mappings
			   (template_id, student_id, workbook_name, normalized_name, row_index, gender_block)
			 VALUES (?, ?, ?, ?, ?, ?)`,
			[
				templateId,
				learner.studentId,
				learner.workbookName,
				learner.normalizedName,
				learner.rowIndex,
				learner.genderBlock ?? null
			]
		);
	}
	for (const day of SEEDED_DAYS) {
		await driver.execute(
			`INSERT INTO sf2_month_date_mappings
			   (template_id, date, column_letter, column_index, sheet_name)
			 VALUES (?, ?, ?, ?, ?)`,
			[templateId, day.date, day.columnLetter, day.columnIndex, day.sheetName ?? null]
		);
	}
}
