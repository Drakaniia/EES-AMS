import type { SqlDriver } from '$lib/db';
import type { Worksheet } from 'exceljs';
import { WORKBOOK_PATH } from '$lib/features/excel/__tests__/template-fixture';
import {
	CLASS_ID,
	FEMALE_ONE,
	FEMALE_ROW,
	MALE_ONE,
	MALE_ROW,
	MALE_TWO,
	SECOND_MALE_ROW,
	SEPTEMBER_SHEET,
	SCHOOL_YEAR,
	TEMPLATE_ID
} from './install-fixture';

export function findSheet(workbook: { worksheets: Worksheet[] }, name: string): Worksheet {
	const sheet = workbook.worksheets.find((candidate) => candidate.name === name);
	if (sheet === undefined) throw new Error(`no worksheet named ${name}`);
	return sheet;
}

/**
 * A seed that looks like a teacher's `.sqlite`: both mapping models populated, an
 * `sf2_templates` row naming a workbook, and the awkward absences.
 *
 * Only the tables the diagnostic's SQL touches are declared, which is the convention
 * the shared schema fixture documents: tests declare the columns their own SQL reads.
 */
const SCHEMA = `
CREATE TABLE students (
	id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, card_serial TEXT,
	class_id TEXT, created_at INTEGER NOT NULL, gender TEXT, sf2_learner_id TEXT);

CREATE TABLE classes (
	id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, day_start TEXT,
	day_end TEXT, late_after TEXT, created_at INTEGER, room TEXT DEFAULT 'N/A',
	sessions TEXT, days TEXT);

CREATE TABLE events (
	id TEXT PRIMARY KEY NOT NULL, student_id TEXT NOT NULL, class_id TEXT,
	event_type TEXT NOT NULL, timestamp INTEGER NOT NULL, note TEXT,
	session_key TEXT, override_reason TEXT, updated_at INTEGER);

CREATE TABLE sf2_templates (
	id TEXT PRIMARY KEY NOT NULL, source_path TEXT, source_hash TEXT, school_id TEXT,
	school_name TEXT, school_year TEXT, report_month TEXT, grade_level TEXT,
	section TEXT, adviser_name TEXT, school_head_name TEXT, layout_fingerprint TEXT,
	active_class_id TEXT, imported_at INTEGER, last_synced_at INTEGER);

CREATE TABLE sf2_student_mappings (
	template_id TEXT, student_id TEXT, workbook_name TEXT, normalized_name TEXT,
	row_index INTEGER, gender_block TEXT, PRIMARY KEY (template_id, student_id));

CREATE TABLE sf2_date_mappings (
	template_id TEXT, sheet_name TEXT, date TEXT, column_letter TEXT,
	column_index INTEGER, PRIMARY KEY (template_id, date));

CREATE TABLE sf2_month_templates (
	id TEXT PRIMARY KEY NOT NULL, active_class_id TEXT, school_year TEXT,
	report_month TEXT, report_year INTEGER, source_path TEXT, source_hash TEXT,
	school_id TEXT, school_name TEXT, grade_level TEXT, section TEXT,
	adviser_name TEXT, school_head_name TEXT, first_school_day INTEGER,
	imported_at INTEGER, last_synced_at INTEGER, workbook_x_count INTEGER,
	workbook_scanned_at INTEGER, first_school_day_override INTEGER);

CREATE TABLE sf2_month_student_mappings (
	template_id TEXT, student_id TEXT, workbook_name TEXT, normalized_name TEXT,
	row_index INTEGER, gender_block TEXT, sf2_learner_id TEXT,
	PRIMARY KEY (template_id, student_id));

CREATE TABLE sf2_month_date_mappings (
	template_id TEXT, date TEXT, column_letter TEXT, column_index INTEGER,
	PRIMARY KEY (template_id, date));
`;

/** One absence row as the tests declare it. */
export type SeedAbsence = {
	id: string;
	studentId: string;
	classId?: string;
	/** `YYYY-MM-DD` in the machine's local zone - what the SQL derives. */
	date: string;
	/** Local hour, so a test can put an evening absence either side of midnight UTC. */
	hour?: number;
};

export type SeedOptions = {
	/**
	 * The workbook `sf2_templates` points at. Defaults to the path
	 * {@link loadTemplate} writes, which is the only workbook in the in-memory folder.
	 */
	sourcePath?: string | undefined;
	/** Declare the per-month tables at all, for a pre-v19 database. Defaults to true. */
	withMonthTables?: boolean;
	absences?: readonly SeedAbsence[];
	/** Stamp `PRAGMA user_version`. Defaults to 19. */
	schemaVersion?: number;
};

export async function createSchema(driver: SqlDriver, withMonthTables = true): Promise<void> {
	await driver.script(
		SCHEMA.split('\n\n')
			.filter((block) => withMonthTables || !block.includes('sf2_month_'))
			.join('\n\n')
	);
}

/**
 * Seed one class, one legacy template row, both mapping models, and the absences.
 *
 * The default absences are the four from the Rust fixture, chosen so each of the
 * three "is this absence ours?" answers is exercised: named class and known student,
 * another class but a known student, no class at all but a known student, and a student
 * of another class with neither.
 */
export async function seedInstall(driver: SqlDriver, options: SeedOptions = {}): Promise<void> {
	const {
		sourcePath = WORKBOOK_PATH,
		withMonthTables = true,
		schemaVersion = 19,
		absences = DEFAULT_ABSENCES
	} = options;

	await createSchema(driver, withMonthTables);

	await driver.script(`
		INSERT INTO classes (id, name, day_start, day_end, late_after, created_at)
			VALUES ('c1', 'Grade 3 - MATAPAT', '08:00', '15:00', '08:45', 1);
		INSERT INTO students VALUES ('s1', '${MALE_ONE}', NULL, 'c1', 1, 'male', NULL);
		INSERT INTO students VALUES ('s2', '${MALE_TWO}', NULL, 'c1', 1, 'male', NULL);
		INSERT INTO students VALUES ('s3', '${FEMALE_ONE}', NULL, 'c1', 1, 'female', NULL);
		INSERT INTO students VALUES ('s9', 'DELA CRUZ, JUAN', NULL, 'other-class', 1, 'male', NULL);
		INSERT INTO sf2_templates VALUES
			('${TEMPLATE_ID}', '${sourcePath}', 'hash', '132839', 'Espiritu Elementary School',
			 '${SCHOOL_YEAR}', 'SEPTEMBER', 'Grade 3', 'MATAPAT', 'ADVISER', 'HEAD',
			 'fingerprint', '${CLASS_ID}', 1, 1);
		INSERT INTO sf2_student_mappings VALUES
			('${TEMPLATE_ID}', 's1', '${MALE_ONE}', 'ALVARADO,ZYRON JAY E.', ${MALE_ROW}, 'MALE');
		INSERT INTO sf2_student_mappings VALUES
			('${TEMPLATE_ID}', 's2', '${MALE_TWO}', 'BAPTISMA,JONATHAN', ${SECOND_MALE_ROW}, 'MALE');
		INSERT INTO sf2_student_mappings VALUES
			('${TEMPLATE_ID}', 's3', '${FEMALE_ONE}', 'SALIMBOT,RAFA LATISHA', ${FEMALE_ROW}, 'FEMALE');
		INSERT INTO sf2_date_mappings VALUES
			('${TEMPLATE_ID}', '${SEPTEMBER_SHEET}', '2026-09-01', 'F', 6);
		INSERT INTO sf2_date_mappings VALUES
			('${TEMPLATE_ID}', '${SEPTEMBER_SHEET}', '2026-09-02', 'H', 8);
	`);

	if (withMonthTables) {
		await driver.script(`
			INSERT INTO sf2_month_templates
				(id, active_class_id, school_year, report_month, report_year, source_path,
				 source_hash, school_id, school_name, grade_level, section, adviser_name,
				 school_head_name, first_school_day, imported_at, last_synced_at,
				 workbook_x_count, workbook_scanned_at, first_school_day_override)
			VALUES
				('${TEMPLATE_ID}', '${CLASS_ID}', '${SCHOOL_YEAR}', 'SEPTEMBER', 2026,
				 '${sourcePath}', 'hash', '132839', 'Espiritu Elementary School', 'Grade 3',
				 'MATAPAT', 'ADVISER', 'HEAD', 1, 1, NULL, 0, NULL, NULL);
			INSERT INTO sf2_month_student_mappings VALUES
				('${TEMPLATE_ID}', 's1', '${MALE_ONE}', 'ALVARADO,ZYRON JAY E.', ${MALE_ROW}, 'MALE', NULL);
			INSERT INTO sf2_month_student_mappings VALUES
				('${TEMPLATE_ID}', 's2', '${MALE_TWO}', 'BAPTISMA,JONATHAN', ${SECOND_MALE_ROW}, 'MALE', NULL);
			INSERT INTO sf2_month_student_mappings VALUES
				('${TEMPLATE_ID}', 's3', '${FEMALE_ONE}', 'SALIMBOT,RAFA LATISHA', ${FEMALE_ROW}, 'FEMALE', NULL);
			INSERT INTO sf2_month_date_mappings VALUES ('${TEMPLATE_ID}', '2026-09-01', 'F', 6);
			INSERT INTO sf2_month_date_mappings VALUES ('${TEMPLATE_ID}', '2026-09-02', 'H', 8);
		`);
	}

	for (const absence of absences) {
		await driver.execute('INSERT INTO events VALUES (?, ?, ?, ?, ?, NULL, NULL, NULL, NULL)', [
			absence.id,
			absence.studentId,
			absence.classId ?? null,
			'absent',
			unixSecondsOf(absence.date, absence.hour ?? 12)
		]);
	}
	await driver.execute(
		"INSERT INTO events VALUES ('e-in', 's1', ?, 'in', ?, NULL, NULL, NULL, NULL)",
		[CLASS_ID, unixSecondsOf('2026-09-03', 12)]
	);
	await driver.execute(`PRAGMA user_version = ${schemaVersion}`);
}

/**
 * The four absences from the Rust fixture, plus a present event that must never be
 * counted as an `X`.
 */
export const DEFAULT_ABSENCES: readonly SeedAbsence[] = [
	{ id: 'e1', studentId: 's1', classId: CLASS_ID, date: '2026-09-01' },
	{ id: 'e2', studentId: 's1', classId: 'other-class', date: '2026-09-02' },
	{ id: 'e3', studentId: 's1', date: '2026-09-07' },
	{ id: 'e5', studentId: 's9', classId: 'other-class', date: '2026-09-08' }
];

/** Every row of every table, for proving a run changed nothing. */
export async function snapshotTables(driver: SqlDriver): Promise<Record<string, unknown[]>> {
	const tables = await driver.query<{ name: string }>(
		"SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name"
	);
	const snapshot: Record<string, unknown[]> = {};
	for (const { name } of tables) {
		snapshot[name] = await driver.query(`SELECT * FROM "${name}" ORDER BY 1, 2, 3`);
	}
	return snapshot;
}

/**
 * Epoch seconds for a local wall-clock instant.
 *
 * An absence at 12:00 local is midday on the machine running the tests, so the
 * `strftime(..., 'localtime')` the SQL applies lands on the intended calendar day in
 * any zone.
 */
export function unixSecondsOf(date: string, hour: number): number {
	return Math.floor(new Date(`${date}T${pad2(hour)}:00:00`).getTime() / 1000);
}

/** The local calendar day, computed without SQL - so a test can check the SQL. */
export function localDateOf(date: string, hour = 12): string {
	const local = new Date(unixSecondsOf(date, hour) * 1000);
	return `${local.getFullYear()}-${pad2(local.getMonth() + 1)}-${pad2(local.getDate())}`;
}

function pad2(value: number): string {
	return String(value).padStart(2, '0');
}
