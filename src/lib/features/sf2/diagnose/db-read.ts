/**
 * Every database read the diagnostic makes, and nothing else — the port of
 * `src-tauri/src/sf2/diagnose/db_read.rs`.
 *
 * ## Read-only by construction
 *
 * Every statement is a `SELECT` or a `PRAGMA` that only reads; see `./sql` for why the
 * Rust read-only *handle* could not be carried over and what replaces it. Tables a
 * given schema version has never heard of are asked about first and reported as
 * missing, never as empty.
 *
 * ## Determinism
 *
 * Rust built `legacy_rosters` and `month_rosters` as `HashMap`s, whose iteration order
 * was arbitrary. They are `Map`s here, keyed by `template_id` and filled in the same
 * template order the reader above returned, so iteration is stable - although nothing
 * downstream iterates them, so the order is a property rather than a guarantee.
 */

import { getDriver } from '$lib/db';
import type { SqlDriver } from '$lib/db';
import type {
	AbsentRecord,
	EventTypeCount,
	MappingSource,
	MappingTableState,
	RosterRow,
	SheetDayGridSummary,
	TableRowCount
} from './model';
import {
	ABSENT_EVENTS_SQL,
	ABSENT_EVENT_TYPE,
	EVENT_COUNTS_SQL,
	LEGACY_TABLES,
	LEGACY_TEMPLATES_SQL,
	MONTH_TEMPLATES_SQL,
	PER_MONTH_TABLES,
	SCHEMA_VERSION_SQL,
	STUDENTS_SQL,
	TABLE_EXISTS_SQL,
	TABLE_ROW_COUNT_SQL,
	gridSummarySql,
	rosterSql
} from './sql';

/** One row of the `students` table. */
export interface StudentRow {
	id: string;
	name: string;
	classId?: string;
}

/** The legacy per-class SF2 template row. */
export interface LegacyTemplateRow {
	id: string;
	sourcePath: string;
	schoolYear: string;
	reportMonth: string;
	activeClassId: string;
	gradeLevel: string;
	section: string;
	lastSyncedAt?: number;
}

/** One `sf2_month_templates` row (spec 6.2 v19). */
export interface MonthTemplateRow {
	id: string;
	activeClassId: string;
	schoolYear: string;
	reportMonth: string;
	reportYear: number;
	sourcePath: string;
	firstSchoolDay: number;
	lastSyncedAt?: number;
	workbookXCount: number;
	workbookScannedAt?: number;
}

/** Everything the diagnostic knows about the database before a workbook is involved. */
export interface DbSnapshot {
	schemaVersion?: number;
	legacyTemplates: LegacyTemplateRow[];
	monthTemplates: MonthTemplateRow[];
	/** Keyed by `template_id`. */
	legacyRosters: Map<string, RosterRow[]>;
	/** Keyed by `template_id`. */
	monthRosters: Map<string, RosterRow[]>;
	absentEvents: AbsentRecord[];
	eventCounts: EventTypeCount[];
	students: StudentRow[];
	tables: MappingTableState;
	totalAbsentEvents: number;
}

/**
 * The template the comparison is anchored on.
 *
 * The legacy row wins when it exists, because that is the row whose `source_path`
 * names the one workbook the app considers its own, and because on an install that
 * has never been split it is the only row there is.
 */
export function anchorTemplate(snapshot: DbSnapshot): LegacyTemplateRow | undefined {
	return snapshot.legacyTemplates[0];
}

/** The class this workbook is for, from whichever table has a row. */
export function activeClassId(snapshot: DbSnapshot): string | undefined {
	return anchorTemplate(snapshot)?.activeClassId ?? snapshot.monthTemplates[0]?.activeClassId;
}

/** The school year the twelve months hang off. */
export function schoolYear(snapshot: DbSnapshot): string | undefined {
	return anchorTemplate(snapshot)?.schoolYear ?? snapshot.monthTemplates[0]?.schoolYear;
}

/** Re-exported so callers need not import from two modules for the type. */
export type { MappingSource };

/**
 * Read every fact the diagnostic needs from the database.
 *
 * Takes no path, where the Rust reader took one: that reader opened the file itself on
 * a private handle, whereas here the driver is already the app's database (OPFS under
 * D8, `node:sqlite` in tests) and there is no second handle to be had. The
 * "there is no database at <path>" refusal therefore has no failure left to catch and
 * is gone; the diagnostic's `databasePath` is a display label taken from
 * `DB_FILENAME`.
 */
export async function readSnapshot(): Promise<DbSnapshot> {
	const driver = getDriver();

	const snapshot: DbSnapshot = {
		legacyTemplates: [],
		monthTemplates: [],
		legacyRosters: new Map(),
		monthRosters: new Map(),
		absentEvents: [],
		eventCounts: [],
		students: [],
		tables: { legacy: [], perMonth: [], legacyDateMappingSheets: [], monthDateMappingGrids: [] },
		totalAbsentEvents: 0
	};

	snapshot.schemaVersion = await readSchemaVersion(driver);
	snapshot.tables = await readTableState(driver);
	snapshot.legacyTemplates = await readLegacyTemplates(driver);
	snapshot.monthTemplates = await readMonthTemplates(driver);

	for (const template of snapshot.legacyTemplates) {
		snapshot.legacyRosters.set(
			template.id,
			await readRoster(driver, rosterSql('sf2_student_mappings'), template.id)
		);
	}
	for (const template of snapshot.monthTemplates) {
		snapshot.monthRosters.set(
			template.id,
			await readRoster(driver, rosterSql('sf2_month_student_mappings'), template.id)
		);
	}

	const legacyExists = await tableExists(driver, 'sf2_date_mappings');
	const legacyAnchor = anchorTemplate(snapshot);
	if (legacyExists && legacyAnchor !== undefined) {
		snapshot.tables.legacyDateMappingSheets = await readGridSummaries(
			driver,
			gridSummarySql('sf2_date_mappings', true),
			legacyAnchor.id
		);
	}
	if (await tableExists(driver, 'sf2_month_date_mappings')) {
		for (const template of snapshot.monthTemplates) {
			snapshot.tables.monthDateMappingGrids.push(
				...(await readGridSummaries(
					driver,
					gridSummarySql('sf2_month_date_mappings', false),
					template.id
				))
			);
		}
	}

	if (await tableExists(driver, 'events')) {
		snapshot.absentEvents = await readAbsentEvents(driver);
		snapshot.eventCounts = await readEventCounts(driver);
		snapshot.totalAbsentEvents =
			snapshot.eventCounts.find((count) => count.eventType === ABSENT_EVENT_TYPE)?.rows ?? 0;
	}
	if (await tableExists(driver, 'students')) {
		snapshot.students = await readStudents(driver);
	}

	return snapshot;
}

async function readSchemaVersion(driver: SqlDriver): Promise<number | undefined> {
	const row = await driver.queryOne<Record<string, unknown>>(SCHEMA_VERSION_SQL);
	const version = row?.user_version;
	if (typeof version === 'number') return version;
	if (typeof version === 'bigint') return Number(version);
	return undefined;
}

async function tableExists(driver: SqlDriver, table: string): Promise<boolean> {
	const row = await driver.queryOne<{ matches: number }>(TABLE_EXISTS_SQL, [table]);
	return Number(row?.matches ?? 0) > 0;
}

/**
 * Row counts for the six mapping tables, with an explicit "no such table" for the ones
 * a pre-v19 database has never heard of.
 */
async function readRowCounts(
	driver: SqlDriver,
	tables: readonly string[]
): Promise<TableRowCount[]> {
	const counts: TableRowCount[] = [];
	for (const table of tables) {
		const exists = await tableExists(driver, table);
		if (!exists) {
			counts.push({ table, exists, rows: undefined });
			continue;
		}
		const row = await driver.queryOne<{ rows: number | bigint }>(
			TABLE_ROW_COUNT_SQL.replace('{table}', table)
		);
		counts.push({ table, exists, rows: Number(row?.rows ?? 0) });
	}
	return counts;
}

async function readTableState(driver: SqlDriver): Promise<MappingTableState> {
	return {
		legacy: await readRowCounts(driver, LEGACY_TABLES),
		perMonth: await readRowCounts(driver, PER_MONTH_TABLES),
		legacyDateMappingSheets: [],
		monthDateMappingGrids: []
	};
}

async function readLegacyTemplates(driver: SqlDriver): Promise<LegacyTemplateRow[]> {
	if (!(await tableExists(driver, 'sf2_templates'))) return [];
	const rows = await driver.query<{
		id: string;
		source_path: string;
		school_year: string;
		report_month: string;
		active_class_id: string;
		grade_level: string;
		section: string;
		last_synced_at: number | null;
	}>(LEGACY_TEMPLATES_SQL);
	return rows.map((row) => ({
		id: row.id,
		sourcePath: row.source_path,
		schoolYear: row.school_year,
		reportMonth: row.report_month,
		activeClassId: row.active_class_id,
		gradeLevel: row.grade_level,
		section: row.section,
		lastSyncedAt: optional(row.last_synced_at)
	}));
}

async function readMonthTemplates(driver: SqlDriver): Promise<MonthTemplateRow[]> {
	if (!(await tableExists(driver, 'sf2_month_templates'))) return [];
	const rows = await driver.query<{
		id: string;
		active_class_id: string;
		school_year: string;
		report_month: string;
		report_year: number;
		source_path: string;
		first_school_day: number;
		last_synced_at: number | null;
		workbook_x_count: number;
		workbook_scanned_at: number | null;
	}>(MONTH_TEMPLATES_SQL);
	return rows.map((row) => ({
		id: row.id,
		activeClassId: row.active_class_id,
		schoolYear: row.school_year,
		reportMonth: row.report_month,
		reportYear: Number(row.report_year),
		sourcePath: row.source_path,
		firstSchoolDay: Number(row.first_school_day),
		lastSyncedAt: optional(row.last_synced_at),
		workbookXCount: Number(row.workbook_x_count),
		workbookScannedAt: optional(row.workbook_scanned_at)
	}));
}

async function readRoster(
	driver: SqlDriver,
	sql: string,
	templateId: string
): Promise<RosterRow[]> {
	const rows = await driver.query<{
		student_id: string;
		workbook_name: string;
		row_index: number;
	}>(sql, [templateId]);
	return rows.map((row) => ({
		studentId: row.student_id,
		workbookName: row.workbook_name,
		rowIndex: Number(row.row_index)
	}));
}

async function readGridSummaries(
	driver: SqlDriver,
	sql: string,
	templateId: string
): Promise<SheetDayGridSummary[]> {
	const rows = await driver.query<{
		sheet_name: string | null;
		year_month: string;
		first_date: string | null;
		last_date: string | null;
		day_columns: number | bigint;
	}>(sql, [templateId]);
	return rows.map((row) => ({
		sheetName: row.sheet_name ?? '',
		yearMonth: row.year_month,
		firstDate: row.first_date ?? '',
		lastDate: row.last_date ?? '',
		dayColumns: Number(row.day_columns)
	}));
}

async function readAbsentEvents(driver: SqlDriver): Promise<AbsentRecord[]> {
	const rows = await driver.query<{
		student_id: string;
		class_id: string | null;
		local_date: string;
	}>(ABSENT_EVENTS_SQL);
	return rows.map((row) => ({
		studentId: row.student_id,
		classId: optionalText(row.class_id),
		date: row.local_date
	}));
}

async function readEventCounts(driver: SqlDriver): Promise<EventTypeCount[]> {
	const rows = await driver.query<{ event_type: string; rows: number | bigint }>(EVENT_COUNTS_SQL);
	return rows.map((row) => ({ eventType: row.event_type, rows: Number(row.rows) }));
}

async function readStudents(driver: SqlDriver): Promise<StudentRow[]> {
	const rows = await driver.query<{
		id: string;
		name: string;
		class_id: string | null;
	}>(STUDENTS_SQL);
	return rows.map((row) => ({
		id: row.id,
		name: row.name,
		classId: optionalText(row.class_id)
	}));
}

function optional(value: number | bigint | string | null): number | undefined {
	return value === null ? undefined : Number(value);
}

/** A nullable text column. `''` is a real value here and is kept as one. */
function optionalText(value: string | null): string | undefined {
	return value === null ? undefined : value;
}
