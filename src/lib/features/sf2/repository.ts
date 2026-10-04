import { getDriver, invalidInput } from '$lib/db';
import { normalizeSchoolYear } from '$lib/db/migrations';
import type { Sf2DateMapping } from '$lib/features/sf2/metadata';
import type { Sf2TemplateSummary } from '$lib/types';

/**
 * The pre-split SF2 tables — `sf2_templates`, `sf2_student_mappings`,
 * `sf2_date_mappings`.
 *
 * These tables are what an install has *before* the per-month split runs, and the
 * split does not delete them: on an install that has not been split they are still
 * where some of the data lives, which is why the month reads fall back to them.
 */

/** One row of `sf2_templates`. */
export interface Sf2TemplateRecord {
	id: string;
	sourcePath: string;
	sourceHash: string;
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
	layoutFingerprint: string;
	activeClassId: string;
	importedAt: number;
	/** Epoch seconds of the last attendance→workbook sync; absent means never synced. */
	lastSyncedAt?: number;
}

/** One row of `sf2_student_mappings`: a learner row the app has claimed. */
export interface Sf2StudentMappingRecord {
	templateId: string;
	studentId: string;
	workbookName: string;
	normalizedName: string;
	rowIndex: number;
	genderBlock?: string;
}

/**
 * Reject a workbook analysis that produced no calendar dates before it can delete
 * the mappings the database already holds.
 *
 * The Excel analysis only reads *visible* monthly sheets, so it returns an empty set
 * whenever the target sheet failed to become visible or could not be renamed to a
 * parseable `MONTH YEAR` name. Committing that empty set deletes every date mapping
 * for the template, which is the signature of the destructive-sync chain: no mappings
 * means no weekday columns, an empty reports grid, and a total workbook clear with
 * nothing to write back.
 */
export const EMPTY_DATE_ANALYSIS_MESSAGE =
	'The SF2 workbook produced no calendar dates. The existing mappings were left untouched.';

/** Same guard for an empty roster: committing it would unmap every learner in the class. */
export const EMPTY_ROSTER_ANALYSIS_MESSAGE =
	'The SF2 workbook produced no learners. The existing mappings were left untouched.';

const TEMPLATE_COLUMNS = `id, source_path, source_hash, school_id, school_name, school_year,
    report_month, grade_level, section, adviser_name, school_head_name,
    layout_fingerprint, active_class_id, imported_at, last_synced_at`;
const STUDENT_MAPPING_COLUMNS =
	'template_id, student_id, workbook_name, normalized_name, row_index, gender_block';
const DATE_MAPPING_COLUMNS = 'template_id, sheet_name, date, column_letter, column_index';

interface DateRow {
	template_id: string;
	sheet_name: string | null;
	date: string;
	column_letter: string;
	column_index: number;
}

function toDateMapping(row: DateRow): Sf2DateMapping {
	return {
		templateId: row.template_id,
		// The pre-split table recorded the sheet each day was on, and it is the only
		// record of it. A row without one reads as blank rather than as a guess.
		sheetName: row.sheet_name ?? '',
		date: row.date,
		columnLetter: row.column_letter,
		columnIndex: row.column_index
	};
}

interface TemplateRow {
	id: string;
	source_path: string | null;
	source_hash: string | null;
	school_id: string | null;
	school_name: string | null;
	school_year: string | null;
	report_month: string | null;
	grade_level: string | null;
	section: string | null;
	adviser_name: string | null;
	school_head_name: string | null;
	layout_fingerprint: string | null;
	active_class_id: string | null;
	imported_at: number;
	last_synced_at: number | null;
}

/** Rust read these columns as `String`, which a NULL would fail on. Blank is the honest read. */
function text(value: string | null): string {
	return value ?? '';
}

function toTemplateRecord(row: TemplateRow): Sf2TemplateRecord {
	return {
		id: row.id,
		sourcePath: text(row.source_path),
		sourceHash: text(row.source_hash),
		schoolId: text(row.school_id),
		schoolName: text(row.school_name),
		// Normalised on read so the legacy table and the per-month table can never hand
		// the caller two spellings of the same school year - the two are compared
		// against each other in `month_preview`.
		schoolYear: normalizeSchoolYear(text(row.school_year)),
		reportMonth: text(row.report_month),
		gradeLevel: text(row.grade_level),
		section: text(row.section),
		adviserName: text(row.adviser_name),
		schoolHeadName: text(row.school_head_name),
		layoutFingerprint: text(row.layout_fingerprint),
		activeClassId: text(row.active_class_id),
		importedAt: row.imported_at,
		lastSyncedAt: row.last_synced_at ?? undefined
	};
}

/** The template row as the summary the Settings and Reports surfaces already read. */
export function templateSummary(record: Sf2TemplateRecord): Sf2TemplateSummary {
	return {
		id: record.id,
		sourcePath: record.sourcePath,
		schoolId: record.schoolId,
		schoolName: record.schoolName,
		schoolYear: record.schoolYear,
		reportMonth: record.reportMonth,
		gradeLevel: record.gradeLevel,
		section: record.section,
		adviserName: record.adviserName,
		schoolHeadName: record.schoolHeadName,
		classId: record.activeClassId,
		importedAt: record.importedAt
	};
}

export async function findTemplate(
	sourceHash: string,
	gradeLevel: string,
	section: string
): Promise<Sf2TemplateRecord | undefined> {
	const row = await getDriver().queryOne<TemplateRow>(
		`SELECT ${TEMPLATE_COLUMNS} FROM sf2_templates
		 WHERE source_hash = ? AND grade_level = ? AND section = ? LIMIT 1`,
		[sourceHash, gradeLevel, section]
	);
	return row === undefined ? undefined : toTemplateRecord(row);
}

/** The newest pre-split template row of a class, if it has one. */
export async function latestTemplateForClass(
	classId: string
): Promise<Sf2TemplateRecord | undefined> {
	const row = await getDriver().queryOne<TemplateRow>(
		`SELECT ${TEMPLATE_COLUMNS} FROM sf2_templates
		 WHERE active_class_id = ? ORDER BY imported_at DESC LIMIT 1`,
		[classId]
	);
	return row === undefined ? undefined : toTemplateRecord(row);
}

export async function listTemplates(): Promise<Sf2TemplateSummary[]> {
	const rows = await getDriver().query<TemplateRow>(
		`SELECT ${TEMPLATE_COLUMNS} FROM sf2_templates ORDER BY imported_at DESC`
	);
	return rows.map((row) => templateSummary(toTemplateRecord(row)));
}

async function replaceMappings(
	templateId: string,
	students: readonly Sf2StudentMappingRecord[],
	dates: readonly Sf2DateMapping[]
): Promise<void> {
	const driver = getDriver();
	await driver.execute('DELETE FROM sf2_student_mappings WHERE template_id = ?', [templateId]);
	for (const student of students) {
		await driver.execute(
			`INSERT OR REPLACE INTO sf2_student_mappings (${STUDENT_MAPPING_COLUMNS})
			 VALUES (?, ?, ?, ?, ?, ?)`,
			[
				student.templateId,
				student.studentId,
				student.workbookName,
				student.normalizedName,
				student.rowIndex,
				student.genderBlock ?? null
			]
		);
	}

	// Delete ALL existing date mappings before inserting the new ones. Mappings from a
	// previous import may hold dates from a DIFFERENT year (e.g. "2025-07-01" from the
	// original 2025 template vs "2026-07-01" from the current school year). INSERT OR
	// REPLACE on PRIMARY KEY (template_id, date) would NOT replace the old row, which
	// corrupts mark placement: 2025-Jul-1 is a Tuesday (col H), 2026-Jul-1 a Wednesday
	// (col I), so the same normalised day would claim two columns.
	await driver.execute('DELETE FROM sf2_date_mappings WHERE template_id = ?', [templateId]);
	for (const date of dates) {
		await driver.execute(
			`INSERT OR REPLACE INTO sf2_date_mappings (${DATE_MAPPING_COLUMNS})
			 VALUES (?, ?, ?, ?, ?)`,
			[date.templateId, date.sheetName, date.date, date.columnLetter, date.columnIndex]
		);
	}
}

/** Create a pre-split template row, or refresh the one the identity already has. */
export async function upsertTemplateWithMappings(
	template: Sf2TemplateRecord,
	students: readonly Sf2StudentMappingRecord[],
	dates: readonly Sf2DateMapping[]
): Promise<void> {
	const driver = getDriver();
	await driver.transaction(async () => {
		await driver.execute(
			`INSERT INTO sf2_templates (
				id, source_path, source_hash, school_id, school_name, school_year,
				report_month, grade_level, section, adviser_name, school_head_name,
				layout_fingerprint, active_class_id, imported_at, last_synced_at)
			 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			 ON CONFLICT(source_hash, grade_level, section) DO UPDATE SET
				source_path = excluded.source_path,
				school_id = excluded.school_id,
				school_name = excluded.school_name,
				school_year = excluded.school_year,
				report_month = excluded.report_month,
				grade_level = excluded.grade_level,
				section = excluded.section,
				adviser_name = excluded.adviser_name,
				school_head_name = excluded.school_head_name,
				layout_fingerprint = excluded.layout_fingerprint,
				active_class_id = excluded.active_class_id,
				imported_at = excluded.imported_at,
				last_synced_at = excluded.last_synced_at`,
			[
				template.id,
				template.sourcePath,
				template.sourceHash,
				template.schoolId,
				template.schoolName,
				template.schoolYear,
				template.reportMonth,
				template.gradeLevel,
				template.section,
				template.adviserName,
				template.schoolHeadName,
				template.layoutFingerprint,
				template.activeClassId,
				template.importedAt,
				template.lastSyncedAt ?? null
			]
		);
		await replaceMappings(template.id, students, dates);
	});
}

/**
 * Refresh a template's metadata and replace its student + date mappings.
 *
 * Rejects a degenerate workbook analysis (no dates, or no learners) *before* the
 * transaction deletes anything, so a failed workbook read can never commit an empty
 * mapping set over a good one.
 */
export async function updateTemplateWithMappings(
	template: Sf2TemplateRecord,
	students: readonly Sf2StudentMappingRecord[],
	dates: readonly Sf2DateMapping[]
): Promise<void> {
	if (dates.length === 0) throw invalidInput(EMPTY_DATE_ANALYSIS_MESSAGE);
	if (students.length === 0) throw invalidInput(EMPTY_ROSTER_ANALYSIS_MESSAGE);

	const driver = getDriver();
	await driver.transaction(async () => {
		const rowsUpdated = await driver.execute(
			`UPDATE sf2_templates
			 SET source_path = ?, source_hash = ?, school_id = ?, school_name = ?,
			     school_year = ?, report_month = ?, grade_level = ?, section = ?,
			     adviser_name = ?, school_head_name = ?, layout_fingerprint = ?,
			     active_class_id = ?, imported_at = ?, last_synced_at = ?
			 WHERE id = ?`,
			[
				template.sourcePath,
				template.sourceHash,
				template.schoolId,
				template.schoolName,
				template.schoolYear,
				template.reportMonth,
				template.gradeLevel,
				template.section,
				template.adviserName,
				template.schoolHeadName,
				template.layoutFingerprint,
				template.activeClassId,
				template.importedAt,
				template.lastSyncedAt ?? null,
				template.id
			]
		);
		if (rowsUpdated === 0) throw invalidInput('Selected SF2 workbook was not found');
		await replaceMappings(template.id, students, dates);
	});
}

/**
 * Record - or clear - the timestamp (seconds) of the last successful
 * attendance→workbook sync. Pass `undefined` to reset.
 *
 * This is a *record* now, not a decision input. It used to gate the workbook write
 * through a "has any event landed since the last sync?" shortcut, which compared it
 * against the newest event timestamp and treated "no events, ever synced" as "in
 * sync" - a row count standing in for evidence.
 */
export async function setLastSyncedAt(templateId: string, syncedAt?: number): Promise<void> {
	await getDriver().execute('UPDATE sf2_templates SET last_synced_at = ? WHERE id = ?', [
		syncedAt ?? null,
		templateId
	]);
}

export async function studentMappingsForTemplate(
	templateId: string
): Promise<Sf2StudentMappingRecord[]> {
	const rows = await getDriver().query<{
		template_id: string;
		student_id: string;
		workbook_name: string;
		normalized_name: string;
		row_index: number;
		gender_block: string | null;
	}>(
		`SELECT ${STUDENT_MAPPING_COLUMNS} FROM sf2_student_mappings
		 WHERE template_id = ? ORDER BY row_index ASC`,
		[templateId]
	);
	return rows.map((row) => ({
		templateId: row.template_id,
		studentId: row.student_id,
		workbookName: row.workbook_name,
		normalizedName: row.normalized_name,
		rowIndex: row.row_index,
		genderBlock: row.gender_block ?? undefined
	}));
}

export async function dateMappingsForTemplate(templateId: string): Promise<Sf2DateMapping[]> {
	const rows = await getDriver().query<DateRow>(
		`SELECT ${DATE_MAPPING_COLUMNS} FROM sf2_date_mappings
		 WHERE template_id = ? ORDER BY date ASC`,
		[templateId]
	);
	return rows.map(toDateMapping);
}

/**
 * One month of a template's day-number grid, as a closed date range.
 *
 * `sf2_date_mappings` is not scoped to a month - it is keyed by a full `YYYY-MM-DD`
 * and a template analysed across a school year holds a row per day it ever saw. This
 * is the month-scoped read, and it exists so that a caller serving one month
 * physically cannot be handed another's columns.
 *
 * `startDate` / `endDate` are inclusive fixed-width ISO bounds, which is what makes the
 * `>=` / `<=` comparison a chronological one. The GLOB is a well-formedness guard: a
 * malformed stored date is not a date, and letting one into the range would be a
 * silent off-by-one on the boundary rows.
 */
export async function legacyDateMappingsInMonth(
	templateId: string,
	startDate: string,
	endDate: string
): Promise<Sf2DateMapping[]> {
	const rows = await getDriver().query<DateRow>(
		`SELECT ${DATE_MAPPING_COLUMNS} FROM sf2_date_mappings
		 WHERE template_id = ?
		   AND date >= ?
		   AND date <= ?
		   AND date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
		 ORDER BY date ASC`,
		[templateId, startDate, endDate]
	);
	return rows.map(toDateMapping);
}
