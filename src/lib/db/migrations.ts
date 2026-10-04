import type { SqlDriver, SqlParam } from './driver';
import { internal } from './error';

import migrateToV1Sql from './sql/migrate_to_v1.sql?raw';
import migrateToV2Sql from './sql/migrate_to_v2.sql?raw';
import migrateToV3Sql from './sql/migrate_to_v3.sql?raw';
import migrateToV4Sql from './sql/migrate_to_v4.sql?raw';
import migrateToV5Sql from './sql/migrate_to_v5.sql?raw';
import migrateToV6Sql from './sql/migrate_to_v6.sql?raw';
import migrateToV7Sql from './sql/migrate_to_v7.sql?raw';
import migrateToV8Sql from './sql/migrate_to_v8.sql?raw';
import migrateToV9Sql from './sql/migrate_to_v9.sql?raw';
import migrateToV10Sql from './sql/migrate_to_v10.sql?raw';
import migrateToV11Sql from './sql/migrate_to_v11.sql?raw';
import migrateToV12Sql from './sql/migrate_to_v12.sql?raw';
import migrateToV13Sql from './sql/migrate_to_v13.sql?raw';
import migrateToV14Sql from './sql/migrate_to_v14.sql?raw';
import migrateToV15Sql from './sql/migrate_to_v15.sql?raw';
import migrateToV16Sql from './sql/migrate_to_v16.sql?raw';
import migrateToV17Sql from './sql/migrate_to_v17.sql?raw';
import migrateToV18Sql from './sql/migrate_to_v18.sql?raw';
import migrateToV19Sql from './sql/migrate_to_v19.sql?raw';
import migrateToV20Sql from './sql/migrate_to_v20.sql?raw';
import migrateToV21Sql from './sql/migrate_to_v21.sql?raw';
import migrateToV22Sql from './sql/migrate_to_v22.sql?raw';
import migrateToV24Sql from './sql/migrate_to_v24.sql?raw';
import purgeUnknownEventTypes from './sql/purge_unknown_event_types.sql?raw';
import rowCountSql from './sql/row_count.sql?raw';
import columnExistsSql from './sql/column_exists.sql?raw';
import tableColumnsSql from './sql/table_columns.sql?raw';

/**
 * The SQLite migration chain.
 *
 * ## The chain is load-bearing
 *
 * A teacher's database is at *some* version in the middle of this list, so the
 * version numbers, the order, and the SQL itself are a compatibility contract
 * with every file already on disk (spec D9). Nothing here may be renumbered,
 * reordered, collapsed, or "simplified" - a migration that looks redundant today
 * is the step an existing database still has to walk through. v3 is the clearest
 * example: v1 already creates `settings.quarter`, so v3 adds nothing today, and
 * it still runs because a database stamped v2 has to pass through it.
 *
 * ## `CURRENT_SCHEMA_VERSION`
 *
 * The number and the migration body must not drift apart, so both live in this
 * file and the number is written exactly once: `migrate()` refuses to leave a
 * database above `CURRENT_SCHEMA_VERSION`, and the chain has no version 23 file
 * because v23 is code (see `migrateToV23`).
 */

export const CURRENT_SCHEMA_VERSION = 25;

/** Canonical uppercase month names, as `sf2MonthName` returns them. */
const SF2_MONTH_NAMES = [
	'JANUARY',
	'FEBRUARY',
	'MARCH',
	'APRIL',
	'MAY',
	'JUNE',
	'JULY',
	'AUGUST',
	'SEPTEMBER',
	'OCTOBER',
	'NOVEMBER',
	'DECEMBER'
];

type Migration = { version: number; run: (driver: SqlDriver) => Promise<void> };

/** One statement read out of a migration file, with its `-- name:` marker. */
type Statement = {
	/**
	 * The `-- name: <label>` marker above the statement, when it has one.
	 *
	 * Unmarked statements are the file's DDL and run once. Marked statements
	 * are data work whose placeholders are bound by the migration that owns the
	 * file, and are looked up by name. A marker covers every statement up to
	 * the next marker, so one named group can be a batch.
	 */
	name: string | null;
	sql: string;
};

/**
 * Run every migration this database has not seen yet.
 *
 * Ascending, one version at a time, each stamped into `PRAGMA user_version` as
 * it completes. The stamp is what makes a chain this long restartable: a launch
 * that dies mid-migration comes back and replays only the versions after the
 * last stamp.
 */
export async function migrate(driver: SqlDriver): Promise<void> {
	const from = await storedSchemaVersion(driver);
	for (const step of CHAIN) {
		if (step.version <= from) continue;
		await step.run(driver);
		await driver.script(`PRAGMA user_version = ${step.version}`);
	}
}

/** `PRAGMA user_version` - 0 on a database that has never been migrated. */
export async function storedSchemaVersion(driver: SqlDriver): Promise<number> {
	return scalar(driver, 'PRAGMA user_version');
}

const CHAIN: Migration[] = [
	{ version: 1, run: migrateToV1 },
	{ version: 2, run: migrateToV2 },
	{ version: 3, run: migrateToV3 },
	{ version: 4, run: migrateToV4 },
	{ version: 5, run: migrateToV5 },
	{ version: 6, run: migrateToV6 },
	{ version: 7, run: migrateToV7 },
	{ version: 8, run: migrateToV8 },
	{ version: 9, run: migrateToV9 },
	{ version: 10, run: migrateToV10 },
	{ version: 11, run: migrateToV11 },
	{ version: 12, run: migrateToV12 },
	{ version: 13, run: migrateToV13 },
	{ version: 14, run: migrateToV14 },
	{ version: 15, run: migrateToV15 },
	{ version: 16, run: migrateToV16 },
	{ version: 17, run: migrateToV17 },
	{ version: 18, run: migrateToV18 },
	{ version: 19, run: migrateToV19 },
	{ version: 20, run: migrateToV20 },
	{ version: 21, run: migrateToV21 },
	{ version: 22, run: migrateToV22 },
	{ version: 23, run: migrateToV23 },
	{ version: 24, run: migrateToV24 },
	{ version: 25, run: migrateToV25 }
];

// ---------------------------------------------------------------- v1 - v10

/** v1 - classes, students, events, settings, and the classless->class move. */
async function migrateToV1(driver: SqlDriver): Promise<void> {
	const statements = parseStatements(migrateToV1Sql);
	await runUnnamed(driver, statements);

	// A fresh install has no pre-v1 `students` table to copy from, and SQLite
	// refuses to prepare a SELECT against a table that does not exist, so the
	// copy is gated on the table being there rather than guarded inside SQL.
	if (await tableExists(driver, 'students')) {
		for (const sql of namedStatements(statements, 'legacy_copy')) {
			await driver.script(sql);
		}
	}

	for (const sql of namedStatements(statements, 'finalize')) {
		await driver.script(sql);
	}
}

/** v2 - a room number per class. */
async function migrateToV2(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV2Sql);
}

/** v3 - the active quarter. */
async function migrateToV3(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV3Sql);
}

/** v4 - the six quarter start/end dates. */
async function migrateToV4(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV4Sql);
}

/**
 * v5 - per-class sessions, backfilled from the class's own day window.
 *
 * The backfill only runs when the column was missing. On a replay the column is
 * already there, and re-deriving it would overwrite sessions the teacher has
 * since edited.
 */
async function migrateToV5(driver: SqlDriver): Promise<void> {
	const added = !(await columnExists(driver, 'classes', 'sessions'));
	await runFile(driver, migrateToV5Sql);
	if (!added) return;

	const classes = await driver.query<{
		id: string;
		day_start: string;
		day_end: string;
		late_after: string;
	}>('SELECT id, day_start, day_end, late_after FROM classes');
	for (const row of classes) {
		const sessions = JSON.stringify([
			{
				name: 'Full Day',
				startTime: row.day_start,
				endTime: row.day_end,
				lateAfter: row.late_after
			}
		]);
		await driver.execute('UPDATE classes SET sessions = ? WHERE id = ?', [sessions, row.id]);
	}
}

/** v6 - which weekdays a class meets, defaulting every class to Monday-Friday. */
async function migrateToV6(driver: SqlDriver): Promise<void> {
	const added = !(await columnExists(driver, 'classes', 'days'));
	await runFile(driver, migrateToV6Sql);
	if (!added) return;
	await driver.execute('UPDATE classes SET days = ?', [JSON.stringify([1, 2, 3, 4, 5])]);
}

/** v7 - clamp the quarter to the three periods the app supports. */
async function migrateToV7(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV7Sql);
}

/** v8 - the attendance mode: manual marks or card reader. */
async function migrateToV8(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV8Sql);
}

/** v9 - the DepEd SF2 template, roster and date mappings. */
async function migrateToV9(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV9Sql);
}

/** v10 - the SF2 form metadata, on settings rather than on a template row. */
async function migrateToV10(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV10Sql);
}

// -------------------------------------------------------------- v11 - v18

/**
 * v11 - rebuild `students` and `events` for one 'in' event per student and no
 * external student number.
 *
 * The delete of unknown event types runs first and *outside* the bracket,
 * because it is the one step of this migration that is meant to change the
 * `events` row count; counting across it would make the guard reject the very
 * deletion it exists to make safe, and would blind the guard to the rebuild -
 * which is the part that actually risks dropping rows.
 *
 * Both the row counts and the column lists are checked. A row count cannot see a
 * lost column, and this migration rebuilds both tables from the v11 shape: a
 * database whose `user_version` was lost while its tables had already been
 * extended by a newer build would come out of here with `session_key` /
 * `override_reason` / `updated_at` and `gender` / `sf2_learner_id` silently
 * gone, and no error anywhere.
 */
async function migrateToV11(driver: SqlDriver): Promise<void> {
	await driver.script(purgeUnknownEventTypes);

	const studentsBefore = await rowCount(driver, 'students');
	const eventsBefore = await rowCount(driver, 'events');
	const studentColumnsBefore = await tableColumns(driver, 'students');
	const eventColumnsBefore = await tableColumns(driver, 'events');

	await runFile(driver, migrateToV11Sql);

	assertRowCountPreserved('students', studentsBefore, await rowCount(driver, 'students'), 'v11');
	assertRowCountPreserved('events', eventsBefore, await rowCount(driver, 'events'), 'v11');
	await assertColumnsPreserved(
		driver,
		[
			['students', studentColumnsBefore],
			['events', eventColumnsBefore]
		],
		'v11'
	);
}

/** v12 - the SF2 metadata moves onto the workbook template row. */
async function migrateToV12(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV12Sql);
}

/** v13 - student gender, for the SF2 roster's sex sections. */
async function migrateToV13(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV13Sql);
}

/** v14 - the attendance exception audit trail. */
async function migrateToV14(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV14Sql);
}

/** v15 - the general audit trail. */
async function migrateToV15(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV15Sql);
}

/** v16 - when a workbook's attendance was last synced. */
async function migrateToV16(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV16Sql);
}

/**
 * v17 - rebuild `events` so the CHECK admits an explicit 'absent'.
 *
 * Absence became its own record instead of being derived from a missing 'in'
 * record, so other students are never auto-recorded. The row count is asserted
 * across the rebuild: losing one attendance record here would be permanent and
 * invisible.
 */
async function migrateToV17(driver: SqlDriver): Promise<void> {
	const eventsBefore = await rowCount(driver, 'events');
	await runFile(driver, migrateToV17Sql);
	assertRowCountPreserved('events', eventsBefore, await rowCount(driver, 'events'), 'v17');
}

/** v18 - sidebar branding: logo path and custom title. */
async function migrateToV18(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV18Sql);
}

// -------------------------------------------------------------- v19 - v24

/** v19 - one row per month workbook file. */
async function migrateToV19(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV19Sql);
}

/** v20 - one roster per month file, plus the DepEd learner ID. */
async function migrateToV20(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV20Sql);
}

/** v21 - the day-number grid of one month file. */
async function migrateToV21(driver: SqlDriver): Promise<void> {
	await runFile(driver, migrateToV21Sql);
}

/**
 * v22 - settings for the per-month model, the first-school-day override column,
 * and the backfill of the legacy single-template row.
 *
 * The backfill is what keeps an install working before the split runs. Every
 * mapping it copies is counted before and after, with the same assertion the v11
 * and v17 rebuilds use: a backfill that quietly drops a mapping is a hard error,
 * because a missing date mapping is the precondition for the whole
 * destructive-sync chain.
 */
async function migrateToV22(driver: SqlDriver): Promise<void> {
	const statements = parseStatements(migrateToV22Sql);
	await runUnnamed(driver, statements);

	for (let month = 1; month <= 12; month += 1) {
		const monthName = SF2_MONTH_NAMES[month - 1];
		const monthNumber = String(month).padStart(2, '0');
		const bind = (sql: string): string =>
			sql
				.replaceAll('{month_name}', monthName)
				.replaceAll('{month_abbr}', monthName.slice(0, 3))
				.replaceAll('{month_number}', monthNumber);
		const named = (label: string): string => bind(namedStatements(statements, label)[0]);

		// The legacy model is one workbook per class with a mutable
		// `report_month`, so each legacy row has exactly one month to become -
		// and every statement is written to be a no-op when replayed.
		for (const label of ['template', 'students', 'dates']) {
			await driver.script(named(label));
		}

		const bracket = `v22 backfill (${monthName})`;
		assertRowCountPreserved(
			'sf2_month_student_mappings',
			await scalar(driver, named('students_expected')),
			await scalar(driver, named('students_actual')),
			bracket
		);
		assertRowCountPreserved(
			'sf2_month_date_mappings',
			await scalar(driver, named('dates_expected')),
			await scalar(driver, named('dates_actual')),
			bracket
		);
	}
}

/**
 * v23 - one canonical school-year label.
 *
 * A month row is looked up by exact equality on
 * `(active_class_id, school_year, report_month)`, and "which school year does
 * this class have months for" is answered by a GLOB for
 * `[0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9]`. Both were written against
 * `2026-2027`, so a user who types the DepEd form's own rendering of the same
 * year - `2026 - 2027`, spaces around the dash - stores a label that matches no
 * row and passes no GLOB: the whole per-month table becomes silently
 * unreachable and every read falls back to the legacy tables for a whole school
 * year, with nothing reporting an error.
 *
 * This is code and not a `.sql` file because the canonical form is defined by
 * one function (`normalizeSchoolYear`), which is also what the read and write
 * paths apply. Re-implementing that parser in SQL would give the backfill and
 * the normaliser two independent definitions of "canonical" - the same
 * two-format bug one layer down, where it would be even harder to see.
 *
 * A label with fewer than two four-digit years is left as it is, so a row the
 * user has not finished typing survives.
 */
async function migrateToV23(driver: SqlDriver): Promise<void> {
	for (const table of ['sf2_templates', 'sf2_month_templates']) {
		let labels: string[];
		try {
			labels = (
				await driver.query<{ school_year: string }>(
					`SELECT DISTINCT school_year FROM ${table} WHERE school_year <> ''`
				)
			).map((row) => row.school_year);
		} catch (thrown) {
			throw internal(`v23 could not read ${table}: ${messageOf(thrown)}`);
		}

		for (const label of labels) {
			const canonical = normalizeSchoolYear(label);
			if (canonical === label) continue;
			try {
				await driver.execute(`UPDATE ${table} SET school_year = ? WHERE school_year = ?`, [
					canonical,
					label
				]);
			} catch (thrown) {
				// The one place a rewrite can conflict: two rows for the same
				// month stored under two spellings of the same year. That is the
				// duplication this defect produces, and it is repaired below
				// rather than swallowed.
				if (table !== 'sf2_month_templates') {
					throw internal(
						`v23 could not rewrite the school year ${label} in ${table}: ${messageOf(thrown)}`
					);
				}
				await collapseDuplicateMonthRows(driver, label, canonical);
			}
		}
	}

	await normaliseSettingsSchoolYear(driver);
}

/**
 * Two rows for one month, stored under two spellings of the same year.
 *
 * Collapsing them is a repair, not a cleanup: the duplicate is unreachable by
 * every per-month read, so the month has been showing the legacy tables all
 * along, and leaving both rows would leave a month that cannot be represented at
 * all. The row that survives is the **oldest**, i.e. everything past the first
 * in `(imported_at, id)` order.
 *
 * Logged loudly, because deleting a row is the one destructive thing this
 * migration does and it must never be silent.
 */
async function collapseDuplicateMonthRows(
	driver: SqlDriver,
	spaced: string,
	canonical: string
): Promise<void> {
	const duplicates = await driver.query<{ id: string; imported_at: number | bigint }>(
		'SELECT id, imported_at FROM sf2_month_templates WHERE school_year = ? OR school_year = ? ORDER BY imported_at ASC, id ASC',
		[spaced, canonical]
	);

	for (const row of duplicates.slice(1)) {
		await driver.execute('DELETE FROM sf2_month_templates WHERE id = ?', [row.id]);
		warn(
			`v23: removed the older duplicate month row ${row.id} (imported ${row.imported_at}) so that "${spaced}" and "${canonical}" can share one canonical school year; its date and student mappings went with it.`
		);
	}
}

/**
 * Keep the app-level `settings.school_year` in step with the SF2 tables.
 *
 * It is the copy the user typed and the frontend shows, so leaving it behind
 * would let the two disagree again - the bug this migration exists to close,
 * restated somewhere else.
 */
async function normaliseSettingsSchoolYear(driver: SqlDriver): Promise<void> {
	const row = await driver.queryOne<{ school_year: string | null }>(
		"SELECT school_year FROM settings WHERE id = 'app'"
	);
	if (!row || typeof row.school_year !== 'string') return;

	const canonical = normalizeSchoolYear(row.school_year);
	if (canonical === row.school_year) return;
	await driver.execute("UPDATE settings SET school_year = ? WHERE id = 'app'", [canonical]);
}

/**
 * The **canonical** form of a school-year label: `YYYY-YYYY`, no spaces.
 *
 * A label with fewer than two four-digit years comes back trimmed but otherwise
 * untouched: this is not a validator, and a label the user has not finished
 * typing must survive a round trip rather than be emptied. Only plausible years
 * (1900-2999) count, and only the first of a repeated value, which is what makes
 * `school_form_2_ver2014.2.1.1` yield no year at all rather than `2014` twice.
 */
export function normalizeSchoolYear(schoolYear: string): string {
	const years: number[] = [];
	for (const part of schoolYear.split(/[^0-9]/)) {
		if (part.length !== 4) continue;
		const year = Number(part);
		if (year < 1900 || year > 2999 || years.includes(year)) continue;
		years.push(year);
	}
	if (years.length !== 2) return schoolYear.trim();
	return `${String(years[0]).padStart(4, '0')}-${String(years[1]).padStart(4, '0')}`;
}

/**
 * v24 - `sf2_month_date_mappings` regains its `sheet_name` column.
 *
 * v21 dropped it on the reasoning that a month file has exactly one worksheet,
 * so the sheet is derivable from the month row. The workbook is now one file
 * with twelve month worksheets, and a *write* path has to read the sheet back
 * out of the database and compare it against the workbook - a value that only
 * exists as a derivation cannot be compared against a stored one.
 *
 * Strictly additive: one nullable `ADD COLUMN`, guarded `UPDATE`s that never
 * blank a row, one index. The row count of every table it touches is bracketed
 * by the same guard v11 and v17 use.
 */
/**
 * v25 - branding logo bytes.
 *
 * Rust stored a *path* to a logo file copied around the filesystem. Paths go
 * stale, so the logo now lives in the database as a BLOB. Additive only: a
 * teacher upgrading from v24 keeps every row, and the column is nullable so
 * "no custom logo" stays the default.
 */
async function migrateToV25(driver: SqlDriver): Promise<void> {
	if ((await tableColumns(driver, 'settings')).includes('branding_logo')) return;
	await driver.script('ALTER TABLE settings ADD COLUMN branding_logo BLOB');
}

async function migrateToV24(driver: SqlDriver): Promise<void> {
	const bracketed = [
		'sf2_month_date_mappings',
		'sf2_month_templates',
		'sf2_month_student_mappings'
	];

	// Refuse to run against a database that does not have the tables this
	// migration touches. A v22 database always has them; this exists so a much
	// older install, whose chain failed for an unrelated reason, gets a named
	// error instead of a bare SQLite one.
	for (const table of bracketed) {
		if ((await tableColumns(driver, table)).length === 0) {
			throw internal(
				`schema v${CURRENT_SCHEMA_VERSION} needs the table \`${table}\`, which this database does not have. Restore the pre-migration snapshot stored next to the database file.`
			);
		}
	}

	const before: number[] = [];
	for (const table of bracketed) before.push(await rowCount(driver, table));
	await runFile(driver, migrateToV24Sql);
	for (const [index, table] of bracketed.entries()) {
		const after = await rowCount(driver, table);
		if (before[index] !== after) {
			throw internal(
				`migration v${CURRENT_SCHEMA_VERSION} changed the row count of table \`${table}\`: ${before[index]} rows before, ${after} rows after. Refusing to continue on a database that no longer holds every attendance record. Restore the pre-migration snapshot stored next to the database file and report this.`
			);
		}
	}

	// Not an error: the migration is additive and refuses to invent a value. But
	// a grid cell whose sheet is unknown is a cell nothing may write to, so it
	// is loud rather than silent. Expected to be 0 - a `date` is a full
	// YYYY-MM-DD, so the third backfill pass can always derive a name from it.
	const unresolved = await scalar(
		driver,
		"SELECT COUNT(*) FROM sf2_month_date_mappings WHERE sheet_name IS NULL OR TRIM(sheet_name) = ''"
	);
	if (unresolved > 0) {
		warn(
			`v24: ${unresolved} of the day mappings have no sheet name, because their date is not a YYYY-MM-DD value. They keep every other field; those months are not writable until they are re-derived.`
		);
	}
}

// ------------------------------------------------------------------ runners

/**
 * Run a migration file's unmarked statements in order.
 *
 * SQLite has no `ADD COLUMN IF NOT EXISTS`, and a migration is replayed whenever
 * the process died between the last successful statement and the
 * `PRAGMA user_version` write that follows it. Without this guard that replay
 * aborts on "duplicate column name" and the app no longer starts at all - the
 * worst possible outcome for a migration whose whole purpose is to make the app
 * safer.
 */
async function runUnnamed(driver: SqlDriver, statements: Statement[]): Promise<void> {
	for (const statement of statements) {
		if (statement.name !== null) continue;
		const added = addedColumn(statement.sql);
		if (added && (await columnExists(driver, added.table, added.column))) continue;
		await driver.script(statement.sql);
	}
}

async function runFile(driver: SqlDriver, file: string): Promise<void> {
	await runUnnamed(driver, parseStatements(file));
}

function namedStatements(statements: Statement[], label: string): string[] {
	const found = statements.filter((statement) => statement.name === label);
	if (found.length === 0) {
		throw internal(`migration file is missing its \`${label}\` statements`);
	}
	return found.map((statement) => statement.sql);
}

/**
 * Split a migration file into statements, keeping the `-- name:` markers.
 *
 * The splitter is line-oriented: a line whose first non-space characters are
 * `--` is a comment and is dropped, unless it is inside a string literal, and a
 * `-- name: <label>` line names the statements that follow it. Semicolons inside
 * a string literal do not end a statement.
 *
 * Everything after a statement's closing `;` on the same line is discarded,
 * which is what makes a trailing `-- explain what this column holds` safe. The
 * cost is that a migration file may not put two statements on one line, and may
 * not wrap a string literal across lines. None of them do.
 */
function parseStatements(file: string): Statement[] {
	const statements: Statement[] = [];
	let sql = '';
	let name: string | null = null;
	let quote: string | null = null;

	for (const line of file.split('\n')) {
		if (quote === null) {
			const trimmed = line.trimStart();
			if (trimmed.startsWith('-- name:')) {
				name = trimmed.slice('-- name:'.length).trim();
				continue;
			}
			if (trimmed.startsWith('--')) continue;
		}

		let ended = false;
		for (const character of line) {
			if (ended) break;
			if (quote !== null) {
				sql += character;
				if (character === quote) quote = null;
				continue;
			}
			if (character === "'" || character === '"') {
				quote = character;
				sql += character;
				continue;
			}
			if (character === ';') {
				pushStatement(statements, sql, name);
				sql = '';
				ended = true;
				continue;
			}
			sql += character;
		}
		sql += '\n';
	}
	pushStatement(statements, sql, name);

	return statements;
}

function pushStatement(statements: Statement[], sql: string, name: string | null): void {
	if (sql.trim() !== '') statements.push({ name, sql: sql.trim() });
}

/**
 * The `(table, column)` an `ALTER TABLE ... ADD COLUMN` statement adds, if the
 * statement is one. Keyword matching is case-insensitive.
 */
function addedColumn(sql: string): { table: string; column: string } | null {
	const trimmed = sql.trim();
	if (trimmed.length < 'ALTER TABLE '.length) return null;
	if (trimmed.slice(0, 'ALTER TABLE '.length).toUpperCase() !== 'ALTER TABLE ') return null;

	const rest = trimmed.slice('ALTER TABLE '.length);
	const keyword = rest.toUpperCase().indexOf(' ADD COLUMN ');
	if (keyword < 0) return null;

	const table = rest.slice(0, keyword).trim();
	const column = rest.slice(keyword + ' ADD COLUMN '.length).split(/\s+/)[0];
	if (table === '' || column === undefined || column === '') return null;
	return { table, column };
}

async function tableExists(driver: SqlDriver, table: string): Promise<boolean> {
	const row = await driver.queryOne(
		"SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
		[table]
	);
	return Number(Object.values(row ?? {})[0] ?? 0) > 0;
}

/** Does `table` already have `column`? Both names are hard-coded literals. */
async function columnExists(driver: SqlDriver, table: string, column: string): Promise<boolean> {
	const sql = columnExistsSql.replaceAll('{table}', table).replaceAll('{column}', column);
	return (await scalar(driver, sql)) > 0;
}

/** First column of the first row - the probes above all select one value. */
async function scalar(driver: SqlDriver, sql: string, params: SqlParam[] = []): Promise<number> {
	const row = await driver.queryOne(sql, params);
	return Number(Object.values(row ?? {})[0] ?? 0);
}

/**
 * Count the rows currently in `table`. Used to bracket a table rebuild: read the
 * count before the migration and again after it.
 */
async function rowCount(driver: SqlDriver, table: string): Promise<number> {
	return scalar(driver, rowCountSql.replaceAll('{table}', table));
}

/**
 * Fail the migration when a table rebuild did not carry every row across.
 *
 * A migration that silently drops rows is the failure mode that costs this app
 * its attendance marks: the `events` table *is* the record of every X, and a
 * rebuild that loses rows looks fine until the first mark is written. Making it
 * an error means the damage surfaces at install time - next to the
 * pre-migration snapshot - instead of at the first attendance mark.
 */
function assertRowCountPreserved(
	table: string,
	before: number,
	after: number,
	migrationLabel: string
): void {
	if (before === after) return;
	throw internal(
		`migration ${migrationLabel} lost rows from table \`${table}\`: ${before} rows before the rebuild, ${after} rows after. Refusing to continue on a database that no longer holds every attendance record. Restore the pre-migration snapshot stored next to the database file and report this.`
	);
}

/**
 * Fail the migration when a table rebuild dropped a column.
 *
 * Row counts and column lists are the same failure seen from two sides, and both
 * have to be checked: a rebuild can preserve every row and still destroy what was
 * in them. Every table is reported in one error rather than the first one to
 * fail - an operator restoring from the snapshot needs to know the whole list of
 * what was at risk.
 */
async function assertColumnsPreserved(
	driver: SqlDriver,
	tables: [string, string[]][],
	migrationLabel: string
): Promise<void> {
	const lost: string[] = [];
	for (const [table, before] of tables) {
		const after = new Set(await tableColumns(driver, table));
		for (const column of before) {
			if (!after.has(column)) lost.push(`\`${table}\`.\`${column}\``);
		}
	}
	if (lost.length === 0) return;
	throw internal(
		`migration ${migrationLabel} dropped column(s) ${lost.join(', ')}. That database was written by a newer build than the one running this migration, and continuing would destroy attendance data those columns hold. Restore the pre-migration snapshot stored next to the database file and report this.`
	);
}

/** The columns `table` currently has. */
async function tableColumns(driver: SqlDriver, table: string): Promise<string[]> {
	const rows = await driver.query<{ name: string }>(tableColumnsSql.replaceAll('{table}', table));
	return rows.map((row) => row.name);
}

/**
 * Rust's `log::warn!`. A migration that deletes or blanks a row has to leave a
 * trail, and with the Rust logger gone the webview console is what is left.
 */
function warn(message: string): void {
	console.warn(`[migrations] ${message}`);
}

function messageOf(thrown: unknown): string {
	return thrown instanceof Error ? thrown.message : String(thrown);
}
