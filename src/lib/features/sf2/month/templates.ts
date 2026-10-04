import { appError, getDriver } from '$lib/db';
import { normalizeSchoolYear } from '$lib/db/migrations';
import { sf2MonthName, sf2MonthNumber } from '$lib/features/sf2/calendar';
import { monthWorkbookSheetName } from '$lib/features/sf2/workbook-files';
import type { Sf2MonthTemplate } from '$lib/types';

/**
 * `sf2_month_templates` and `sf2_month_date_mappings` — the port of
 * `src-tauri/src/sf2/month/{mod,template_repo,date_repo}.rs`.
 *
 * Every lookup here is keyed on `(class, school year, month)` or on the row id,
 * never on "the class's current month". That is the whole point of the table: a
 * month is a row, so switching months is a read rather than a mutation of shared
 * state.
 */

const MONTH_TEMPLATE_COLUMNS = `id, active_class_id, school_year, report_month, report_year,
    source_path, source_hash, school_id, school_name, grade_level, section,
    adviser_name, school_head_name, first_school_day, first_school_day_override,
    imported_at, last_synced_at, workbook_x_count, workbook_scanned_at`;

interface MonthTemplateRow {
	id: string;
	active_class_id: string;
	school_year: string;
	report_month: string;
	report_year: number;
	source_path: string;
	source_hash: string;
	school_id: string | null;
	school_name: string | null;
	grade_level: string | null;
	section: string | null;
	adviser_name: string | null;
	school_head_name: string | null;
	first_school_day: number;
	first_school_day_override: number | null;
	imported_at: number;
	last_synced_at: number | null;
	workbook_x_count: number;
	workbook_scanned_at: number | null;
}

/** One day column of one month, as stored in `sf2_month_date_mappings`. */
export interface Sf2MonthDateMappingRecord {
	templateId: string;
	/** `YYYY-MM-DD`. */
	date: string;
	columnLetter: string;
	columnIndex: number;
	/**
	 * The worksheet this day is written to, e.g. `SEPTEMBER 2026`.
	 *
	 * Nullable in the database because a row whose `template_id` has no month row
	 * is an orphan and the v24 migration refuses to invent a name for it.
	 * `resolvedSheetName()` is what every reader uses instead of the field
	 * directly: it falls back to the name the date itself implies, which is the
	 * same string, so an absent column costs a branch and not correctness.
	 */
	sheetName?: string;
}

/**
 * `first_school_day` value meaning "not derived yet".
 *
 * A month file that has not been dated yet stores this rather than a guess. It
 * is a real day number that can never occur, so it is distinguishable from any
 * derived or overridden value without an extra column, and the split job
 * replaces it with a derived day.
 */
export const FIRST_SCHOOL_DAY_UNDETERMINED = 0;

/**
 * Reject a month workbook analysis that produced no calendar dates before it
 * can delete the mappings the database already holds.
 *
 * Committing an empty date-mapping set deletes every day column for the month,
 * which leaves the reports grid empty and the next workbook write with nothing to
 * write back. The degenerate analysis is a symptom, never an instruction.
 */
export const EMPTY_DATE_ANALYSIS_MESSAGE =
	'The SF2 workbook produced no calendar dates. The existing mappings were left untouched.';

/** Same guard for a roster that came back empty: committing it would unmap every learner. */
export const EMPTY_ROSTER_ANALYSIS_MESSAGE =
	'The SF2 workbook produced no learners. The existing mappings were left untouched.';

/**
 * Sort key that reads a school year the way it runs: SEPTEMBER -> DECEMBER of
 * the start year, then JANUARY -> AUGUST of the next. Alphabetical month order
 * would be misleading.
 */
export function schoolYearOrderKey(template: Sf2MonthTemplate): [number, number] {
	return [template.reportYear, sf2MonthNumber(template.reportMonth) ?? 0];
}

function optional(value: string | null): string | undefined {
	return value ?? undefined;
}

function toMonthTemplate(row: MonthTemplateRow): Sf2MonthTemplate {
	return {
		id: row.id,
		classId: row.active_class_id,
		// Normalised on read as well as on write. Normalising only the write
		// side would make every row this build creates findable and every row an
		// older build created invisible, which is the same class of silent miss;
		// normalising only the read side would leave two formats in one database
		// for every future reader to trip over. Both sides is the only version
		// that is not a latent bug.
		schoolYear: normalizeSchoolYear(row.school_year),
		reportMonth: row.report_month,
		reportYear: row.report_year,
		sourcePath: row.source_path,
		sourceHash: row.source_hash,
		schoolId: optional(row.school_id),
		schoolName: optional(row.school_name),
		gradeLevel: optional(row.grade_level),
		section: optional(row.section),
		adviserName: optional(row.adviser_name),
		schoolHeadName: optional(row.school_head_name),
		firstSchoolDay: row.first_school_day,
		firstSchoolDayOverride:
			row.first_school_day_override === null ? undefined : row.first_school_day_override,
		importedAt: row.imported_at,
		lastSyncedAt: row.last_synced_at ?? undefined,
		workbookXCount: row.workbook_x_count,
		workbookScannedAt: row.workbook_scanned_at ?? undefined
	};
}

function toDateMapping(row: {
	template_id: string;
	date: string;
	column_letter: string;
	column_index: number;
	sheet_name: string | null;
}): Sf2MonthDateMappingRecord {
	return {
		templateId: row.template_id,
		date: row.date,
		columnLetter: row.column_letter,
		columnIndex: row.column_index,
		sheetName: optional(row.sheet_name)
	};
}

/**
 * The worksheet name a `YYYY-MM-DD` date implies: `SEPTEMBER 2026`.
 *
 * `undefined` for anything that is not an ISO date, which is the honest answer
 * for a row the v24 migration could not name either.
 */
export function sheetNameFromDate(date: string): string | undefined {
	const trimmed = date.trim();
	if (!/^\d{4}-\d{2}-\d{2}$/.test(trimmed)) return undefined;
	const month = sf2MonthName(Number(trimmed.slice(5, 7)));
	if (month === '') return undefined;
	return monthWorkbookSheetName(month, Number(trimmed.slice(0, 4)));
}

/**
 * The worksheet this day belongs on, derived from the date when the stored
 * column is absent.
 *
 * A date is a full `YYYY-MM-DD`, so its month and year are always readable from
 * it. That makes the derivation total, which is what lets the read path trust the
 * column and lets the migration leave a row it cannot name alone instead of
 * writing a guess.
 */
export function resolvedSheetName(mapping: Sf2MonthDateMappingRecord): string {
	const stored = mapping.sheetName?.trim();
	if (stored) return stored;
	return sheetNameFromDate(mapping.date) ?? '';
}

/**
 * The row for one month of one school year — the read a month switch performs.
 *
 * The school year is normalised on the way in as well as on the way out. A row
 * stored before v23 with `2026 - 2027` in it must still be found when the caller
 * asks for `2026-2027`, because equality on the label is what the whole
 * per-month model is keyed on.
 */
export async function findMonthTemplate(
	classId: string,
	schoolYear: string,
	reportMonth: string
): Promise<Sf2MonthTemplate | undefined> {
	const row = await getDriver().queryOne<MonthTemplateRow>(
		`SELECT ${MONTH_TEMPLATE_COLUMNS} FROM sf2_month_templates
		 WHERE active_class_id = ? AND school_year = ? AND report_month = ?`,
		[classId, normalizeSchoolYear(schoolYear), reportMonth]
	);
	return row === undefined ? undefined : toMonthTemplate(row);
}

/** The row for a month file by id. */
export async function findMonthTemplateById(
	templateId: string
): Promise<Sf2MonthTemplate | undefined> {
	const row = await getDriver().queryOne<MonthTemplateRow>(
		`SELECT ${MONTH_TEMPLATE_COLUMNS} FROM sf2_month_templates WHERE id = ?`,
		[templateId]
	);
	return row === undefined ? undefined : toMonthTemplate(row);
}

/**
 * The newest month row of a class, whatever month it is for.
 *
 * The open-path fallback when the month on screen has no row: the class's
 * latest template is what the retired `latest_template_for_class` would have
 * opened, so the month is derived from that row rather than refused.
 */
export async function latestMonthTemplateForClass(
	classId: string
): Promise<Sf2MonthTemplate | undefined> {
	const row = await getDriver().queryOne<MonthTemplateRow>(
		`SELECT ${MONTH_TEMPLATE_COLUMNS} FROM sf2_month_templates
		 WHERE active_class_id = ? ORDER BY imported_at DESC LIMIT 1`,
		[classId]
	);
	return row === undefined ? undefined : toMonthTemplate(row);
}

/**
 * Every month row of one school year, ordered the way the school year runs.
 * Fewer than 12 rows is normal: months are created as the split or the user
 * creates them.
 */
export async function listMonthTemplatesForSchoolYear(
	classId: string,
	schoolYear: string
): Promise<Sf2MonthTemplate[]> {
	const rows = await getDriver().query<MonthTemplateRow>(
		`SELECT ${MONTH_TEMPLATE_COLUMNS} FROM sf2_month_templates
		 WHERE active_class_id = ? AND school_year = ?
		 ORDER BY report_year ASC`,
		[classId, normalizeSchoolYear(schoolYear)]
	);
	return rows.map(toMonthTemplate).sort((left, right) => {
		const [leftYear, leftMonth] = schoolYearOrderKey(left);
		const [rightYear, rightMonth] = schoolYearOrderKey(right);
		return leftYear - rightYear || leftMonth - rightMonth;
	});
}

/** Every month row on record, newest school year first. */
export async function listAllMonthTemplates(): Promise<Sf2MonthTemplate[]> {
	const rows = await getDriver().query<MonthTemplateRow>(
		`SELECT ${MONTH_TEMPLATE_COLUMNS} FROM sf2_month_templates
		 ORDER BY school_year DESC, report_year ASC`
	);
	return rows.map(toMonthTemplate).sort((left, right) => {
		if (left.schoolYear !== right.schoolYear) {
			return right.schoolYear.localeCompare(left.schoolYear);
		}
		const [leftYear, leftMonth] = schoolYearOrderKey(left);
		const [rightYear, rightMonth] = schoolYearOrderKey(right);
		return leftYear - rightYear || leftMonth - rightMonth;
	});
}

/**
 * Create the row for a month file, or refresh the metadata of the row already
 * there.
 *
 * Leaves the first attendance day, the sync timestamp and the X measurement alone
 * when the row already exists: those are provenance and state, not metadata, and
 * a retried split re-inserting its row must not move a month's first day or
 * forget that it was already synced.
 *
 * The school year is stored in its canonical form, because the conflict target
 * `(class, school year, month)` is what decides whether this is a new row or a
 * refresh. Storing `2026 - 2027` beside an existing `2026-2027` would insert a
 * **second** row for the same month.
 */
export async function upsertMonthTemplate(template: Sf2MonthTemplate): Promise<void> {
	await getDriver().execute(
		`INSERT INTO sf2_month_templates (
			id, active_class_id, school_year, report_month, report_year, source_path,
			source_hash, school_id, school_name, grade_level, section, adviser_name,
			school_head_name, first_school_day, imported_at, last_synced_at,
			workbook_x_count, workbook_scanned_at)
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		 ON CONFLICT(active_class_id, school_year, report_month) DO UPDATE SET
			report_year = excluded.report_year,
			source_path = excluded.source_path,
			source_hash = excluded.source_hash,
			school_id = excluded.school_id,
			school_name = excluded.school_name,
			grade_level = excluded.grade_level,
			section = excluded.section,
			adviser_name = excluded.adviser_name,
			school_head_name = excluded.school_head_name,
			imported_at = excluded.imported_at`,
		[
			template.id,
			template.classId,
			normalizeSchoolYear(template.schoolYear),
			template.reportMonth,
			template.reportYear,
			template.sourcePath,
			template.sourceHash,
			template.schoolId ?? null,
			template.schoolName ?? null,
			template.gradeLevel ?? null,
			template.section ?? null,
			template.adviserName ?? null,
			template.schoolHeadName ?? null,
			template.firstSchoolDay,
			template.importedAt,
			template.lastSyncedAt ?? null,
			template.workbookXCount,
			template.workbookScannedAt ?? null
		]
	);
}

/** Refresh a month file's metadata after it was re-opened. */
export async function updateMonthTemplate(template: Sf2MonthTemplate): Promise<void> {
	const updated = await getDriver().execute(
		`UPDATE sf2_month_templates
		 SET school_year = ?, report_month = ?, report_year = ?, source_path = ?,
		     source_hash = ?, school_id = ?, school_name = ?, grade_level = ?,
		     section = ?, adviser_name = ?, school_head_name = ?, imported_at = ?,
		     last_synced_at = ?
		 WHERE id = ?`,
		[
			normalizeSchoolYear(template.schoolYear),
			template.reportMonth,
			template.reportYear,
			template.sourcePath,
			template.sourceHash,
			template.schoolId ?? null,
			template.schoolName ?? null,
			template.gradeLevel ?? null,
			template.section ?? null,
			template.adviserName ?? null,
			template.schoolHeadName ?? null,
			template.importedAt,
			template.lastSyncedAt ?? null,
			template.id
		]
	);
	if (updated === 0) {
		throw appError('InvalidInput', `No SF2 month workbook is stored for \`${template.id}\``);
	}
}

/**
 * Record a per-month first-attendance-day override.
 *
 * From here on, re-deriving the value from `school_start_date` cannot change it
 * — that is the promise the Reports sidebar override makes.
 */
export async function overrideMonthFirstSchoolDay(
	templateId: string,
	day: number
): Promise<boolean> {
	const updated = await getDriver().execute(
		'UPDATE sf2_month_templates SET first_school_day_override = ?, first_school_day = ? WHERE id = ?',
		[day, day, templateId]
	);
	return updated > 0;
}

/**
 * Write a *derived* first attendance day.
 *
 * Returns `false` — without error — when the month already carries an override,
 * because a caller re-deriving every month on startup must not treat a protected
 * value as a failure. It also refuses to write the "not derived yet" sentinel
 * over a real value.
 */
export async function deriveMonthFirstSchoolDay(templateId: string, day: number): Promise<boolean> {
	const updated = await getDriver().execute(
		`UPDATE sf2_month_templates SET first_school_day = ?
		 WHERE id = ? AND first_school_day_override IS NULL AND ? > 0`,
		[day, templateId, day]
	);
	return updated > 0;
}

/**
 * Drop an override and go back to `derivedDay`.
 *
 * Pass {@link FIRST_SCHOOL_DAY_UNDETERMINED} to leave the month undated until
 * `school_start_date` is known.
 */
export async function clearFirstSchoolDayOverride(
	templateId: string,
	derivedDay: number
): Promise<boolean> {
	const updated = await getDriver().execute(
		`UPDATE sf2_month_templates
		 SET first_school_day_override = NULL, first_school_day = ?
		 WHERE id = ?`,
		[derivedDay, templateId]
	);
	return updated > 0;
}

/** Timestamp (seconds) of the last successful attendance → workbook sync. */
export async function setMonthLastSyncedAt(
	templateId: string,
	syncedAt: number | undefined
): Promise<boolean> {
	const updated = await getDriver().execute(
		'UPDATE sf2_month_templates SET last_synced_at = ? WHERE id = ?',
		[syncedAt ?? null, templateId]
	);
	return updated > 0;
}

/**
 * Mark every one of the class's months as not-yet-synced.
 *
 * A grid correction writes to `events`, which belongs to no month, so all twelve
 * of the class's files are stale with respect to it the moment it lands. Scoped
 * by class for exactly that reason.
 */
export async function clearLastSyncedAtForClass(classId: string): Promise<number> {
	return getDriver().execute(
		'UPDATE sf2_month_templates SET last_synced_at = NULL WHERE active_class_id = ?',
		[classId]
	);
}

/**
 * Store the X count last counted in the file, and when it was counted.
 *
 * `scannedAt` is what separates "the file has no X marks" from "the file was
 * never measured". Only this method sets it, and a month whose file is missing is
 * never measured at all.
 */
export async function recordMonthWorkbookXCount(
	templateId: string,
	xCount: number,
	scannedAt: number
): Promise<boolean> {
	const updated = await getDriver().execute(
		'UPDATE sf2_month_templates SET workbook_x_count = ?, workbook_scanned_at = ? WHERE id = ?',
		[xCount, scannedAt, templateId]
	);
	return updated > 0;
}

/**
 * Forget a month row. Its date and student mappings cascade with it.
 *
 * Never deletes the workbook file — a missing row is recoverable by re-deriving
 * it, a deleted workbook is not.
 */
export async function deleteMonthTemplate(templateId: string): Promise<boolean> {
	const deleted = await getDriver().execute('DELETE FROM sf2_month_templates WHERE id = ?', [
		templateId
	]);
	return deleted > 0;
}

// ── The day-number grid ─────────────────────────────────────────────────────
// Port of `sf2/month/date_repo.rs`. `sf2/date-repo.ts` is a separate deliverable
// in the migration; move these three functions there when it lands.

/**
 * Replace one month file's grid with `dates`, in one transaction.
 *
 * Every statement here is scoped by `template_id`. There is deliberately no
 * "delete this class's calendar": with twelve months sharing one workbook, a
 * re-analysis of September must not be able to reach October's columns.
 *
 * The `templateId` argument is the month the grid belongs to, and it is what every
 * row is filed under - not the id the records carry, which is whatever the build
 * guessed before the month row was written and may name a month that does not exist.
 */
export async function replaceMonthDateMappings(
	templateId: string,
	dates: Sf2MonthDateMappingRecord[]
): Promise<void> {
	if (dates.length === 0) throw appError('InvalidInput', EMPTY_DATE_ANALYSIS_MESSAGE);

	const driver = getDriver();
	await driver.transaction(async () => {
		await driver.execute('DELETE FROM sf2_month_date_mappings WHERE template_id = ?', [templateId]);
		for (const date of dates) {
			await driver.execute(
				`INSERT OR REPLACE INTO sf2_month_date_mappings
				 (template_id, date, column_letter, column_index, sheet_name)
				 VALUES (?, ?, ?, ?, ?)`,
				[templateId, date.date, date.columnLetter, date.columnIndex, resolvedSheetName(date)]
			);
		}
	});
}

/** One month file's grid, in date order. */
export async function listMonthDateMappings(
	templateId: string
): Promise<Sf2MonthDateMappingRecord[]> {
	const rows = await getDriver().query<{
		template_id: string;
		date: string;
		column_letter: string;
		column_index: number;
		sheet_name: string | null;
	}>(
		`SELECT template_id, date, column_letter, column_index, sheet_name
		 FROM sf2_month_date_mappings WHERE template_id = ? ORDER BY date ASC`,
		[templateId]
	);
	return rows.map(toDateMapping);
}

/** How many day columns a month file currently has. */
export async function countMonthDateMappings(templateId: string): Promise<number> {
	const row = await getDriver().queryOne<{ count: number }>(
		'SELECT COUNT(*) AS count FROM sf2_month_date_mappings WHERE template_id = ?',
		[templateId]
	);
	return Number(row?.count ?? 0);
}
