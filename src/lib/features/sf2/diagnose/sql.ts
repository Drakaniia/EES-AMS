/**
 * Every statement the diagnostic runs against the database, in one file.
 *
 * ## Why they are here rather than inline
 *
 * Ported from `src-tauri/src/sf2/diagnose/sql/*.sql`, which kept them in `.sql` files
 * so that the whole read side of the feature could be grepped for
 * `INSERT`/`UPDATE`/`DELETE`/`PRAGMA =` and "there are none" was a check somebody
 * could actually run. That property is worth keeping, and `DIAGNOSTIC_STATEMENTS`
 * keeps it: the test asserts over this array, so a statement added next to its reader
 * without being added here fails the check.
 *
 * ## Why there is no read-only connection any more
 *
 * The Rust diagnostic opened `attendance.db` on its own `SQLITE_OPEN_READ_ONLY`
 * handle, deliberately not borrowing the app's read-write pool, so that "this wrote
 * nothing" was structural rather than a matter of reviewing which statements were
 * chosen - SQLite itself refused the write.
 *
 * The TypeScript layer has one driver and it is read-write, and the TS spec's escape
 * hatch (`$lib/db`) owns no second-handle mechanism. The guarantee is therefore
 * carried by the statements themselves: every query below is a `SELECT` or a
 * `PRAGMA` that only reads, and {@link DIAGNOSTIC_STATEMENTS} is exported so a test
 * can assert that. Do not add a statement here that is not a read.
 */

/** The schema version stamped on the database file: `PRAGMA user_version`. */
export const SCHEMA_VERSION_SQL = 'PRAGMA user_version;';

/**
 * Does `name` exist as a table?
 *
 * The diagnostic has to run against a database that predates the per-month tables, so
 * "the table is missing" is a normal answer here and must be reported as such - never
 * as a row count of zero, and never as an error.
 */
export const TABLE_EXISTS_SQL =
	"SELECT COUNT(*) AS matches FROM sqlite_master WHERE type = 'table' AND name = ?";

/** Row count of one table. `{table}` is a literal from {@link LEGACY_TABLES}. */
export const TABLE_ROW_COUNT_SQL = 'SELECT COUNT(*) AS rows FROM "{table}"';

/** Every legacy per-class SF2 template row, oldest import first. */
export const LEGACY_TEMPLATES_SQL = `
	SELECT id, source_path, school_year, report_month, active_class_id, grade_level, section,
		last_synced_at
	FROM sf2_templates
	ORDER BY imported_at`;

/** Every per-month template row, in the order the school year runs. */
export const MONTH_TEMPLATES_SQL = `
	SELECT id, active_class_id, school_year, report_month, report_year, source_path,
		first_school_day, last_synced_at, workbook_x_count, workbook_scanned_at
	FROM sf2_month_templates
	ORDER BY report_year, report_month`;

/**
 * One roster read, shared by both mapping tables.
 *
 * The column list is deliberately identical in both, so one reader serves both; a
 * file that grew or reordered a column would otherwise fail with a column *type*
 * error about a column the caller never asked for. `{table}` is a literal from this
 * module, never a value read out of the database.
 */
export function rosterSql(table: string): string {
	return `SELECT student_id, workbook_name, row_index FROM ${table}
		WHERE template_id = ? ORDER BY row_index`;
}

/**
 * One stored day grid, grouped by what it covers.
 *
 * `sf2_month_date_mappings` has no `sheet_name` column (spec 0 A4), so the per-month
 * variant selects an empty literal first: same five columns in the same order as the
 * legacy variant, one reader for both.
 *
 * A month's grid also cannot be tied back to a worksheet from the database alone,
 * which is one reason the diagnostic reads the workbook's own day-number row instead
 * of trusting the stored grid.
 */
export function gridSummarySql(table: string, hasSheetName: boolean): string {
	const sheetColumn = hasSheetName ? 'sheet_name' : "''";
	return `
	SELECT ${sheetColumn} AS sheet_name,
		substr(date, 1, 7) AS year_month,
		MIN(date) AS first_date,
		MAX(date) AS last_date,
		COUNT(*) AS day_columns
	FROM ${table}
	WHERE template_id = ?
	GROUP BY ${hasSheetName ? 'sheet_name, ' : ''}year_month
	ORDER BY year_month${hasSheetName ? ', sheet_name' : ''}`;
}

/**
 * Every absence the database holds, with the local calendar date it falls on.
 *
 * `event_type = 'absent'` is the only thing that is an `X` mark (spec 3.2): a present
 * student is a blank cell and has no row here, so this result is the complete set of
 * absences, not a sample.
 *
 * The date is derived exactly the way the Rust reader derived it - `timestamp` is UTC
 * seconds, the school day is the local calendar day - so the months this reports are
 * the months the writer would have written marks into. Doing the conversion in SQL
 * keeps the whole read on one pass over the table.
 *
 * `class_id` comes back so the caller can separate "absences for the class this
 * workbook is for" from "absences recorded against some other class". The first is
 * what a workbook comparison is about; the second is still worth reporting, because an
 * absence with no class is an absence nothing will ever place on a grid.
 */
export const ABSENT_EVENTS_SQL = `
	SELECT student_id, class_id,
		strftime('%Y-%m-%d', timestamp, 'unixepoch', 'localtime') AS local_date
	FROM events
	WHERE event_type = 'absent'
	ORDER BY local_date, student_id`;

/**
 * How many rows `events` holds, split by event type.
 *
 * The absent count is the single number a write path should never shrink without the
 * user asking, so it is reported here next to the total rather than only as a
 * per-month figure that could hide a deletion inside a month nobody was looking at.
 */
export const EVENT_COUNTS_SQL =
	'SELECT event_type, COUNT(*) AS rows FROM events GROUP BY event_type ORDER BY event_type';

/**
 * Every student, with the class they belong to.
 *
 * Needed for two things the mapping tables cannot answer: which absences belong to
 * this workbook's class, and which workbook row a learner name refers to when a
 * worksheet's own roster has to be matched against the database by name.
 */
export const STUDENTS_SQL = 'SELECT id, name, class_id FROM students ORDER BY name';

/**
 * Every read statement the diagnostic can issue.
 *
 * The no-write check iterates this. Keep it complete: a statement that is not listed
 * here is not covered by that check.
 */
export const DIAGNOSTIC_STATEMENTS: readonly string[] = [
	SCHEMA_VERSION_SQL,
	TABLE_EXISTS_SQL,
	TABLE_ROW_COUNT_SQL,
	LEGACY_TEMPLATES_SQL,
	MONTH_TEMPLATES_SQL,
	rosterSql('sf2_student_mappings'),
	rosterSql('sf2_month_student_mappings'),
	gridSummarySql('sf2_date_mappings', true),
	gridSummarySql('sf2_month_date_mappings', false),
	ABSENT_EVENTS_SQL,
	EVENT_COUNTS_SQL,
	STUDENTS_SQL
];

/** The legacy tables, then the per-month tables (spec 6.2). */
export const LEGACY_TABLES = [
	'sf2_templates',
	'sf2_student_mappings',
	'sf2_date_mappings'
] as const;

export const PER_MONTH_TABLES = [
	'sf2_month_templates',
	'sf2_month_student_mappings',
	'sf2_month_date_mappings'
] as const;

/** The `event_type` value that is an `X` mark (spec 3.2). */
export const ABSENT_EVENT_TYPE = 'absent';
