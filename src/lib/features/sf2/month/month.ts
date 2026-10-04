import { appError, getDriver } from '$lib/db';
import { listClasses, getClass } from '$lib/db/repos/classes';
import { listEventsForClassAndDateRange } from '$lib/db/repos/events';
import { getSettings } from '$lib/db/repos/settings';
import { listStudents } from '$lib/db/repos/students';
import { sf2MonthName, sf2MonthNumber } from '$lib/features/sf2/calendar';
import {
	currentYear,
	defaultSchoolStartDate,
	deriveFirstSchoolDay,
	gridAnchorDay,
	isSchoolDay,
	lastDayOfMonth,
	naiveDate,
	parseIsoDate,
	reportYearForSchoolMonth
} from '$lib/features/sf2/first-school-day';
import { schoolYearMonthFiles } from '$lib/features/sf2/workbook-files';
import { mergeWorkbooks } from './merge';
import {
	buildSchoolYearWorkbook,
	isVerified,
	mismatchReason,
	type MonthAbsence,
	type MonthLearnerWrite
} from './workbook-builder';
import { getFileSystem } from '$lib/platform/fs';
import {
	FIRST_SCHOOL_DAY_UNDETERMINED,
	resolvedSheetName,
	type Sf2MonthDateMappingRecord
} from './templates';
import {
	findMonthTemplate,
	findMonthTemplateById,
	listAllMonthTemplates,
	listMonthDateMappings,
	listMonthTemplatesForSchoolYear,
	replaceMonthDateMappings,
	upsertMonthTemplate
} from './templates';
import {
	monthRosterForTemplate,
	replaceMonthRoster,
	type Sf2MonthStudentMapping
} from './students';
import type {
	Student,
	Sf2ExportPreview,
	Sf2ExportReadiness,
	Sf2LaunchMonth,
	Sf2MonthGridPreview,
	Sf2MonthPreview,
	Sf2MonthTemplate,
	Sf2PreviewDate,
	Sf2SchoolCalendarSettings,
	Sf2SplitOutcome,
	Sf2TemplateSummary
} from '$lib/types';

/**
 * The month service.
 *
 * The seven exported functions are the API `$lib/api/sf2-months.ts` already
 * calls, so the UI can be repointed here without a component changing.
 *
 * ## What is a read and what is not
 *
 * `getSf2MonthPreview`, `getSf2LaunchMonth`, `listSf2MonthWorkbooks`,
 * `getSf2SchoolCalendarSettings` and `setSf2SchoolStartDate` touch SQL and one
 * `stat` each. No Excel, no write beyond the one settings column, and no
 * progress event — so a month switch has nothing to show a modal over.
 *
 * `createSf2MonthFile` and `runSf2WorkbookSplit` do open a workbook, and both go
 * through `workbook-builder.ts` and `merge.ts` respectively.
 *
 * ## The June / September year wrap
 *
 * Two year-assignment rules exist in this codebase and they disagree:
 * `sf2ReportYear` wraps at **June** and belongs to the retired one-workbook
 * model; {@link reportYearForSchoolMonth} wraps at **September** and belongs to
 * this one, because a Philippine school year is SEPTEMBER → AUGUST. They differ
 * only for AUGUST.
 *
 * The reconciliation is that **this path never recomputes the year for a month
 * that has a row**. The stored `report_year` wins, and the September rule is used
 * only for a month with no row yet, where there is no stored answer to disagree
 * with.
 *
 * ## Where the mappings come from
 *
 * A grid cell is editable because a *mapping* exists for it. The per-month tables
 * are only populated for months that have been created or split, and the split is
 * an on-demand one-time job, so {@link resolveLegacyMappings} reads the pre-split
 * tables for whatever the per-month tables cannot answer. Nothing on this path
 * writes, and the pre-split tables are never deleted — on an install that has not
 * been split they are still where some of the data lives.
 */

/** Shown when a month is asked for that no name can resolve. */
export const UNKNOWN_MONTH_MESSAGE = 'Report month must be a valid month name';

/**
 * Shown when a month has no school days at all (edge case E2: April, May,
 * summer). A file is never created for such a month, because there is no day in it
 * for the file to record.
 */
export const NO_SCHOOL_DAYS_MESSAGE =
	'This month has no school days, so no SF2 workbook is created for it.';

/**
 * The two settings the per-month model dates itself from.
 *
 * Both stay `null` when unset. A missing settings row is not an error: it reads
 * as two `null`s, which is the "we do not know yet" state the callers above it are
 * already written to handle.
 */
export async function getSf2SchoolCalendarSettings(): Promise<Sf2SchoolCalendarSettings> {
	const row = await getDriver().queryOne<{
		school_start_date: string | null;
		last_report_month: string | null;
	}>(
		`SELECT NULLIF(TRIM(school_start_date), '') AS school_start_date,
		        NULLIF(TRIM(last_report_month), '') AS last_report_month
		 FROM settings WHERE id = 'app'`
	);
	return {
		schoolStartDate: row?.school_start_date ?? null,
		lastReportMonth: row?.last_report_month ?? null
	};
}

/**
 * Record — or clear — the real date classes started.
 *
 * `null` unsets it. A value that is not `YYYY-MM-DD` is refused here rather than
 * stored: every reader parses this column, so a `10/08/2026` that got in would be
 * discovered by a month grid failing to date itself, which is a far worse place to
 * hear about it than by the field that took it.
 *
 * Months whose first school day was overridden by hand keep their override — the
 * stored number is the one the teacher typed, and re-deriving is a separate,
 * explicit step (`deriveMonthFirstSchoolDay` in `templates.ts`).
 */
export async function setSf2SchoolStartDate(schoolStartDate: string | null): Promise<void> {
	const trimmed = schoolStartDate?.trim();
	let stored: string | null = null;
	if (trimmed !== undefined && trimmed !== '') {
		if (parseIsoDate(trimmed) === undefined) {
			throw appError(
				'InvalidInput',
				`\`${trimmed}\` is not a date. Classes started on takes YYYY-MM-DD.`
			);
		}
		stored = trimmed;
	}

	// `UPDATE`, not `REPLACE`: the settings row also holds `last_report_month` and
	// `sf2_split_completed_at`, and a whole-row write is how a school start date
	// ends up costing the user their split state.
	const updated = await getDriver().execute(
		"UPDATE settings SET school_start_date = ? WHERE id = 'app'",
		[stored]
	);
	if (updated === 0) {
		throw appError(
			'Internal',
			'the settings row is missing, so the school start date cannot be recorded. This database is not the schema this build expects; restore a backup.'
		);
	}
}

// â”€â”€ Resolution â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/** The class a month read is for: the one asked for, else the only one on record. */
async function resolveClassId(classId: string | undefined): Promise<string> {
	const asked = classId?.trim();
	if (asked) return asked;
	const classes = await listClasses();
	const first = classes[0];
	if (first === undefined) {
		throw appError('InvalidInput', 'No class is set up yet. Add a class first.');
	}
	return first.id;
}

/**
 * The school year a read is for: the one asked for, else the newest on record,
 * else the legacy settings label, else a year derived from the clock.
 */
async function resolveSchoolYear(classId: string, schoolYear: string | undefined): Promise<string> {
	const asked = schoolYear?.trim();
	if (asked) return asked;

	const latest = await getDriver().queryOne<{ school_year: string }>(
		`SELECT school_year FROM sf2_month_templates
		 WHERE active_class_id = ?
		   AND school_year GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9]'
		 ORDER BY CAST(SUBSTR(school_year, 1, 4) AS INTEGER) DESC
		 LIMIT 1`,
		[classId]
	);
	if (latest !== undefined) return latest.school_year;

	const legacy = (await getSettings()).schoolYear?.trim() ?? '';
	if (legacy !== '') return legacy;

	const year = currentYear();
	return `${year}-${year}`;
}

/** The canonical uppercase name of a month and its number, or an error naming what was wrong. */
function canonicalMonth(reportMonth: string): { month: string; monthNumber: number } {
	const monthNumber = sf2MonthNumber(reportMonth);
	if (monthNumber === undefined) throw appError('InvalidInput', UNKNOWN_MONTH_MESSAGE);
	return { month: sf2MonthName(monthNumber), monthNumber };
}

/**
 * The stored `school_start_date` as a `Date`, defaulting to June 1st of the
 * school year's start year when unset or unreadable.
 *
 * Classes start in June, so the default dates every month of the school year
 * from its own first weekday with no setup step.
 */
function storedStartDate(
	settings: Sf2SchoolCalendarSettings,
	schoolYear?: string
): Date | undefined {
	const parsed =
		settings.schoolStartDate === null
			? undefined
			: (parseIsoDate(settings.schoolStartDate) ?? undefined);
	if (parsed !== undefined) return parsed;
	return schoolYear === undefined ? undefined : defaultSchoolStartDate(schoolYear);
}

// â”€â”€ The one workbook on disk â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/**
 * The one workbook's name, and whether it is on disk.
 *
 * ## How it is resolved, in order
 *
 * 1. The month's own stored row. After the merge every month names the same file,
 *    so this is the answer for all twelve.
 * 2. Any other month row of the class. The file belongs to the *class*, not to a
 *    month, so a month that has not been built yet still names the workbook the
 *    other eleven are in. Answering `""` here instead would make the list report
 *    the file as missing for eleven of twelve rows.
 * 3. The pre-split `sf2_templates` row for the class — what an install has before
 *    the merge has run.
 *
 * A retired per-month file (`SF2-SEPTEMBER-2026.xls`) is **not** consulted. It is
 * not where a month is recorded any more.
 */
async function resolveWorkbookOnDisk(
	classId: string,
	template: Sf2MonthTemplate | undefined
): Promise<{ fileName: string; exists: boolean }> {
	if (template !== undefined) return workbookOnDiskAt(template.sourcePath);

	const other = (await listAllMonthTemplates()).find((row) => row.classId === classId);
	if (other !== undefined) return workbookOnDiskAt(other.sourcePath);

	const legacy = await latestLegacyTemplate(classId);
	// No identity on record at all, so there is no workbook name to report. Not
	// the existence of the *directory*: an empty directory is not a workbook, and
	// reporting one would make every D5 fallback believe its month is on disk.
	if (legacy === undefined) return { fileName: '', exists: false };
	return workbookOnDiskAt(legacy.sourcePath);
}

async function workbookOnDiskAt(
	sourcePath: string
): Promise<{ fileName: string; exists: boolean }> {
	const separator = Math.max(sourcePath.lastIndexOf('/'), sourcePath.lastIndexOf('\\'));
	const fileName = separator >= 0 ? sourcePath.slice(separator + 1) : sourcePath;
	try {
		const stat = await getFileSystem().stat(sourcePath);
		return { fileName, exists: stat.isFile };
	} catch {
		return { fileName, exists: false };
	}
}

// â”€â”€ The pre-split tables â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/**
 * The pre-split template row for a class.
 *
 * `sf2::repository::Sf2Repository::latest_template_for_class`. `sf2/repository.ts`
 * is not ported yet; move this there when it lands.
 */
async function latestLegacyTemplate(
	classId: string
): Promise<
	{ id: string; sourcePath: string; sourceHash: string; metadata: LegacyMetadata } | undefined
> {
	const row = await getDriver().queryOne<{
		id: string;
		source_path: string;
		source_hash: string;
		school_id: string | null;
		school_name: string | null;
		school_year: string | null;
		report_month: string | null;
		grade_level: string | null;
		section: string | null;
		adviser_name: string | null;
		school_head_name: string | null;
		active_class_id: string | null;
		imported_at: number;
		last_synced_at: number | null;
	}>(
		`SELECT id, source_path, source_hash, school_id, school_name, school_year,
		        report_month, grade_level, section, adviser_name, school_head_name,
		        active_class_id, imported_at, last_synced_at
		 FROM sf2_templates WHERE active_class_id = ? ORDER BY imported_at DESC LIMIT 1`,
		[classId]
	);
	if (row === undefined) return undefined;
	return {
		id: row.id,
		sourcePath: row.source_path,
		sourceHash: row.source_hash,
		metadata: {
			schoolId: row.school_id,
			schoolName: row.school_name,
			schoolYear: row.school_year,
			reportMonth: row.report_month,
			gradeLevel: row.grade_level,
			section: row.section,
			adviserName: row.adviser_name,
			schoolHeadName: row.school_head_name,
			activeClassId: row.active_class_id,
			importedAt: row.imported_at,
			lastSyncedAt: row.last_synced_at
		}
	};
}

interface LegacyMetadata {
	schoolId: string | null;
	schoolName: string | null;
	schoolYear: string | null;
	reportMonth: string | null;
	gradeLevel: string | null;
	section: string | null;
	adviserName: string | null;
	schoolHeadName: string | null;
	activeClassId: string | null;
	importedAt: number;
	lastSyncedAt: number | null;
}

/**
 * One month of the pre-split day-number grid.
 *
 * `sf2_date_mappings` is keyed by a full `YYYY-MM-DD` and is NOT scoped to a month,
 * so the month is applied as a **closed date range** over the requested year. That
 * is the load-bearing part: a September read must be physically unable to receive
 * October's columns.
 */
async function legacyDatesInMonth(
	templateId: string,
	reportYear: number,
	monthNumber: number
): Promise<Sf2MonthDateMappingRecord[]> {
	const start = `${String(reportYear).padStart(4, '0')}-${String(monthNumber).padStart(2, '0')}-01`;
	const end = `${String(reportYear).padStart(4, '0')}-${String(monthNumber).padStart(2, '0')}-${String(
		lastDayOfMonth(reportYear, monthNumber)
	).padStart(2, '0')}`;
	const rows = await getDriver().query<{
		template_id: string;
		date: string;
		column_letter: string;
		column_index: number;
		sheet_name: string | null;
	}>(
		`SELECT template_id, date, column_letter, column_index, sheet_name
		 FROM sf2_date_mappings
		 WHERE template_id = ? AND date >= ? AND date <= ?
		   AND date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
		 ORDER BY date ASC`,
		[templateId, start, end]
	);
	// The pre-split table recorded the sheet each day was on, and it is the only
	// record of it — but it is not trusted. Under the old hide/rename/clear cycle
	// the visible sheet was whichever month happened to be current, so the month
	// being read is the authority and the stored name is kept only for comparison.
	return rows.map((row) => ({
		templateId: row.template_id,
		date: row.date,
		columnLetter: row.column_letter,
		columnIndex: row.column_index,
		sheetName: row.sheet_name ?? undefined
	}));
}

/**
 * The class roster the pre-split table already holds.
 *
 * The roster is class-wide, not per month (one class, one roster), so there is no
 * month to narrow it by.
 */
async function legacyRoster(templateId: string): Promise<Sf2MonthStudentMapping[]> {
	const rows = await getDriver().query<{
		template_id: string;
		student_id: string;
		workbook_name: string;
		normalized_name: string;
		row_index: number;
		gender_block: string | null;
	}>(
		`SELECT template_id, student_id, workbook_name, normalized_name, row_index, gender_block
		 FROM sf2_student_mappings WHERE template_id = ? ORDER BY row_index ASC`,
		[templateId]
	);
	return rows.map((row) => ({
		templateId: row.template_id,
		studentId: row.student_id,
		workbookName: row.workbook_name,
		normalizedName: row.normalized_name,
		rowIndex: row.row_index,
		genderBlock: row.gender_block ?? undefined,
		// The pre-split table never read the DepEd ID out of a workbook, so there
		// is none to copy. Inventing one would be worse than not having it.
		sf2LearnerId: undefined
	}));
}

interface LegacyMappings {
	dates: Sf2MonthDateMappingRecord[];
	roster: Sf2MonthStudentMapping[];
	legacyTemplate: Awaited<ReturnType<typeof latestLegacyTemplate>>;
}

/**
 * Serve a month from the pre-split tables, for the mappings its own tables cannot
 * supply.
 *
 * Writes nothing. The pre-split tables are read and never modified, and they are
 * not deleted: on an install that has not been split they are the only place some
 * of the data lives.
 */
async function resolveLegacyMappings(
	classId: string,
	monthDates: Sf2MonthDateMappingRecord[],
	monthRoster: Sf2MonthStudentMapping[],
	reportYear: number,
	monthNumber: number
): Promise<LegacyMappings> {
	const legacyTemplate = await latestLegacyTemplate(classId);
	if (legacyTemplate === undefined) {
		// Nothing pre-split on record either. Both sets stay empty, which is the
		// honest answer: there are no mappings for this month anywhere.
		return { dates: monthDates, roster: monthRoster, legacyTemplate: undefined };
	}

	const dates =
		monthDates.length > 0
			? monthDates
			: await legacyDatesInMonth(legacyTemplate.id, reportYear, monthNumber);
	const roster = monthRoster.length > 0 ? monthRoster : await legacyRoster(legacyTemplate.id);
	return { dates, roster, legacyTemplate };
}

// â”€â”€ The grid half: the seam onto `sf2::preview::export_preview` â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/** Everything `export_preview` takes, so the seam needs no database access. */
export interface MonthGridInput {
	/** The workbook identity the month is described by, with only `reportMonth` rewritten. */
	template: Sf2TemplateSummary;
	roster: Sf2MonthStudentMapping[];
	dates: Sf2PreviewDate[];
	className: string;
	classStudents: Student[];
	events: Awaited<ReturnType<typeof listEventsForClassAndDateRange>>;
	readiness: Sf2ExportReadiness;
}

/**
 * `sf2::preview::export_preview`, owned by the `preview` port.
 *
 * It is a builder, not a query, and it is the single implementation of what an X
 * in the grid means — a second one here is how "an X in the grid has to mean the
 * same thing whichever read produced the grid" stops holding. So it is a seam,
 * registered at startup the same way `useDriver()` and `useFileSystem()` are.
 */
export type MonthGridBuilder = (input: MonthGridInput) => Sf2ExportPreview;

let gridBuilder: MonthGridBuilder | null = null;

/** Test/boot seam. Pass `null` to unregister. */
export function useMonthGridBuilder(builder: MonthGridBuilder | null): void {
	gridBuilder = builder;
}

function requireGridBuilder(): MonthGridBuilder {
	if (gridBuilder === null) {
		throw appError(
			'Internal',
			'the SF2 month preview grid builder is not registered. `$lib/features/sf2/preview` registers it at startup.'
		);
	}
	return gridBuilder;
}

/**
 * Every Monday-Friday of the month, in date order.
 *
 * A day the month's grid has a column for carries that column; a day it does not is
 * still listed, with a blank column, because the grid shows every weekday the
 * school had whether or not the workbook got as far as writing a number there.
 *
 * The worksheet each day belongs on is its **own** `sheetName`, not the sheet of
 * the month being read: twelve worksheets share one file, and a column letter is
 * not an address without a sheet.
 */
function expandToMonthWeekdays(
	reportYear: number,
	monthNumber: number,
	sheetName: string,
	mappings: Sf2MonthDateMappingRecord[]
): Sf2PreviewDate[] {
	const mappingByDate = new Map(mappings.map((mapping) => [mapping.date, mapping]));
	const dates: Sf2PreviewDate[] = [];
	for (let day = 1; day <= lastDayOfMonth(reportYear, monthNumber); day += 1) {
		if (!isSchoolDay(naiveDate(reportYear, monthNumber, day) ?? new Date(0))) continue;
		const date = `${String(reportYear).padStart(4, '0')}-${String(monthNumber).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
		const mapping = mappingByDate.get(date);
		const resolved = mapping === undefined ? '' : resolvedSheetName(mapping);
		dates.push({
			date,
			sheetName: resolved === '' ? sheetName : resolved,
			columnLetter: mapping?.columnLetter ?? '',
			columnIndex: mapping?.columnIndex ?? 0
		});
	}
	return dates;
}

// â”€â”€ The seven commands â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/**
 * Read one month of the SF2 reports from SQL. This is the whole month switch.
 *
 * Read-only by construction: no Excel, no write, and deliberately no progress
 * event, so a switch has nothing to show a modal over. The Reports page caches the
 * result per `(class, school year, month)`.
 *
 * `classId` and `schoolYear` are both optional: left out, they resolve to the only
 * class on record and the school year that class actually has months for.
 */
export async function getSf2MonthPreview(
	reportMonth: string,
	classId?: string,
	schoolYear?: string
): Promise<Sf2MonthGridPreview> {
	const { month, monthNumber } = canonicalMonth(reportMonth);
	const resolvedClassId = await resolveClassId(classId);
	const resolvedSchoolYear = await resolveSchoolYear(resolvedClassId, schoolYear);
	const schoolStartDate = storedStartDate(await getSf2SchoolCalendarSettings(), resolvedSchoolYear);

	const template = await findMonthTemplate(resolvedClassId, resolvedSchoolYear, month);
	// The stored year wins. Recomputing it here is how a month file and a legacy
	// template row would come to disagree about AUGUST.
	const reportYear =
		template?.reportYear ??
		reportYearForSchoolMonth(resolvedSchoolYear, monthNumber, currentYear());

	const workbook = await resolveWorkbookOnDisk(resolvedClassId, template);
	// A month with no row of its own has no file and no sheet to name.
	const sheetName = template === undefined ? '' : `${month} ${reportYear}`;

	const monthDates = template === undefined ? [] : await listMonthDateMappings(template.id);
	const monthRoster = template === undefined ? [] : await monthRosterForTemplate(template.id);
	// The month tables answer on their own once a month is split, so the pre-split
	// tables are only read when they cannot.
	const legacy = await resolveLegacyMappings(
		resolvedClassId,
		monthDates,
		monthRoster,
		reportYear,
		monthNumber
	);
	const firstSchoolDay = template?.firstSchoolDay ?? FIRST_SCHOOL_DAY_UNDETERMINED;

	const dates = expandToMonthWeekdays(reportYear, monthNumber, sheetName, legacy.dates);
	const classStudents = await listStudents(resolvedClassId);
	const className = (await getClass(resolvedClassId))?.name ?? '';
	// A deleted student leaves no grid row behind. The delete already removes its
	// mappings, but a stale mapping that survived (empty-class sync, legacy
	// fallback) must not resurface as an orphan row or inflate the counts.
	const liveIds = new Set(classStudents.map((student) => student.id));
	const roster = legacy.roster.filter((mapping) => liveIds.has(mapping.studentId));

	const first = dates[0];
	const last = dates[dates.length - 1];
	const events =
		first === undefined || last === undefined
			? []
			: await listEventsForClassAndDateRange(resolvedClassId, first.date, last.date);

	const issues = monthIssues(
		month,
		template !== undefined,
		workbook.exists,
		legacy.dates,
		roster,
		classStudents
	);
	const readiness: Sf2ExportReadiness = {
		template: undefined,
		mappedStudents: roster.length,
		mappedDates: legacy.dates.length,
		canExport: issues.length === 0,
		issues,
		warnings: []
	};

	const summary =
		template === undefined
			? legacy.legacyTemplate === undefined
				? undefined
				: templateSummaryFromLegacy(legacy.legacyTemplate, month, resolvedSchoolYear)
			: templateSummaryFromMonth(template);

	const preview = requireGridBuilder()({
		template: summary ?? {
			id: template?.id ?? '',
			sourcePath: template?.sourcePath ?? '',
			schoolId: '',
			schoolName: '',
			schoolYear: resolvedSchoolYear,
			reportMonth: month,
			gradeLevel: '',
			section: '',
			adviserName: '',
			schoolHeadName: '',
			classId: resolvedClassId,
			importedAt: template?.importedAt ?? 0
		},
		roster,
		dates,
		className,
		classStudents,
		events,
		readiness
	});

	const warnings = [...preview.warnings];
	if (legacy.legacyTemplate !== undefined && template === undefined) {
		// Said once, in the warnings the sidebar already renders, so a teacher
		// whose grid is being served from the pre-split tables knows their marks are
		// on screen and from where — rather than concluding from a blank identity
		// panel that they are gone.
		warnings.push(
			`${month} is drawn from the SF2 mappings recorded before per-month workbooks existed. ` +
				"Your marks are shown and editable; creating this month's workbook moves it onto the per-month tables."
		);
	}

	return {
		month,
		reportYear,
		schoolYear: resolvedSchoolYear,
		classId: resolvedClassId,
		className,
		sheetName,
		fileName: workbook.fileName,
		fileExists: workbook.exists,
		hasTemplate: template !== undefined,
		firstSchoolDay,
		hasSchoolDays: deriveFirstSchoolDay(schoolStartDate, monthNumber, reportYear) !== undefined,
		gridEmpty: legacy.dates.length === 0,
		workbookXCount: template?.workbookXCount ?? 0,
		workbookScannedAt: template?.workbookScannedAt ?? undefined,
		lastSyncedAt: template?.lastSyncedAt ?? undefined,
		template: summary,
		usesLegacyMappings: legacy.legacyTemplate !== undefined && template === undefined,
		dates: preview.dates,
		students: preview.students,
		absentList: preview.absentList,
		mappedStudents: preview.mappedStudents,
		mappedDates: preview.mappedDates,
		presentCount: preview.presentCount,
		absenceCount: preview.absenceCount,
		unmappedStudentCount: preview.unmappedStudentCount,
		issues: preview.issues,
		warnings
	};
}

/**
 * Which month to open on launch, and whether the app may offer to create it (spec
 * D5, acceptance #12).
 *
 * Today's calendar month, falling back to `settings.last_report_month` when today's
 * month has no workbook. The fallback is never silent: `fellBack` is set and both
 * months are named, so a teacher who opens the app in June and lands on May is told
 * why rather than left to guess. June is the month classes start, so it is the year
 * the months wrap at - not the month the app opens on.
 */
export async function getSf2LaunchMonth(classId?: string): Promise<Sf2LaunchMonth> {
	const resolvedClassId = await resolveClassId(classId);
	const schoolYear = await resolveSchoolYear(resolvedClassId, undefined);
	const settings = await getSf2SchoolCalendarSettings();
	const schoolStartDate = storedStartDate(settings, schoolYear);

	const today = new Date();
	const todayNumber = today.getMonth() + 1;
	const todayMonth = sf2MonthName(todayNumber);
	const todayReportYear = reportYearForSchoolMonth(schoolYear, todayNumber, today.getFullYear());

	const describe = async (
		month: string,
		reportYear: number
	): Promise<{
		fileName: string;
		fileExists: boolean;
		hasTemplate: boolean;
		hasSchoolDays: boolean;
	}> => {
		const monthNumber = sf2MonthNumber(month) ?? todayNumber;
		const row = await findMonthTemplate(resolvedClassId, schoolYear, month);
		// The file is shared, so "is the workbook there?" is one question for the
		// whole year, and a month is on record when its worksheet is.
		const workbook = await resolveWorkbookOnDisk(resolvedClassId, row);
		return {
			fileExists: workbook.exists,
			hasTemplate: row !== undefined,
			hasSchoolDays: deriveFirstSchoolDay(schoolStartDate, monthNumber, reportYear) !== undefined,
			fileName: workbook.fileName
		};
	};

	const isOnRecord = (state: { hasTemplate: boolean }): boolean => state.hasTemplate;
	const todayState = await describe(todayMonth, todayReportYear);
	const issues: string[] = [];
	let month = todayMonth;
	let reportYear = todayReportYear;
	let fellBack = false;

	if (!isOnRecord(todayState)) {
		fellBack = true;
		const last = sf2MonthNumber(settings.lastReportMonth ?? '');
		if (last !== undefined) {
			const lastName = sf2MonthName(last);
			const lastYear = reportYearForSchoolMonth(schoolYear, last, currentYear());
			if (isOnRecord(await describe(lastName, lastYear))) {
				month = lastName;
				reportYear = lastYear;
			}
		}
		if (month === todayMonth) {
			issues.push(`No SF2 workbook exists for ${todayMonth} or for the last month you used.`);
		}
	}

	const state = await describe(month, reportYear);
	if (!state.hasSchoolDays && !isOnRecord(state)) {
		issues.push(NO_SCHOOL_DAYS_MESSAGE);
	}

	return {
		fileName: state.fileName,
		month,
		reportYear,
		schoolYear,
		classId: resolvedClassId,
		fileExists: state.fileExists,
		hasTemplate: state.hasTemplate,
		hasSchoolDays: state.hasSchoolDays,
		todayMonth,
		todayReportYear,
		fellBack,
		// Only offer a create for a month that is genuinely absent and genuinely has
		// days to record. E2 is the `hasSchoolDays` half of this.
		canCreate: !isOnRecord(state) && state.hasSchoolDays,
		todayCanCreate: !isOnRecord(todayState) && todayState.hasSchoolDays,
		needsSchoolStartDate: schoolStartDate === undefined,
		issues
	};
}

/**
 * All twelve month workbooks of the class's school year, as the Settings screen
 * lists them.
 *
 * Twelve rows, always: a month with no stored row still comes back, because "this
 * month has not been set up yet" and "this month is not part of the year" are
 * different answers and only one of them is ever true. Read-only.
 */
export async function listSf2MonthWorkbooks(classId?: string): Promise<Sf2MonthPreview[]> {
	const resolvedClassId = await resolveClassId(classId);
	const schoolYear = await resolveSchoolYear(resolvedClassId, undefined);
	const stored = await listMonthTemplatesForSchoolYear(resolvedClassId, schoolYear);

	const previews: Sf2MonthPreview[] = [];
	for (const entry of schoolYearMonthFiles(schoolYear, currentYear())) {
		const row = stored.find((candidate) => candidate.reportMonth === entry.month);
		const workbook = await resolveWorkbookOnDisk(resolvedClassId, row);
		previews.push({
			month: entry.month,
			// Read off the stored row when there is one, so a month never disagrees
			// with its own grid about which year it belongs to.
			reportYear: row?.reportYear ?? entry.reportYear,
			schoolYear,
			fileName: workbook.fileName,
			fileExists: workbook.exists,
			hasTemplate: row !== undefined,
			firstSchoolDay: row?.firstSchoolDay ?? FIRST_SCHOOL_DAY_UNDETERMINED,
			firstSchoolDayOverridden: row?.firstSchoolDayOverride !== undefined,
			workbookXCount: row?.workbookXCount ?? 0,
			workbookScannedAt: row?.workbookScannedAt ?? null,
			lastSyncedAt: row?.lastSyncedAt ?? null,
			learnerCount: row === undefined ? 0 : (await monthRosterForTemplate(row.id)).length,
			mappedDateCount: row === undefined ? 0 : (await listMonthDateMappings(row.id)).length
		});
	}
	return previews;
}

/**
 * Add the `{MONTH} {year}` worksheet to the workbook the class already has, so the
 * user can switch to that month in one click (edge case E1).
 *
 * Under spec §0 A1 there is no per-month file to create: a month is a worksheet in
 * the one workbook the class already has. This lays that month's day grid over it,
 * copies the roster onto it, writes the absences the database holds, and records
 * the month row and its grid.
 *
 * ## What it will not do
 *
 * - It does not touch the other eleven worksheets. A full merge does that; a
 *   single-month create is additive.
 * - It does not create a worksheet with a mark in it. The absences come from
 *   `events`, and a month nobody has recorded anything in gets an empty grid.
 * - It refuses a month with no school days (E2) rather than producing a worksheet
 *   with no day to record.
 */
export async function createSf2MonthFile(
	reportMonth: string,
	classId?: string
): Promise<Sf2MonthTemplate> {
	const { month, monthNumber } = canonicalMonth(reportMonth);
	const resolvedClassId = await resolveClassId(classId);
	const schoolYear = await resolveSchoolYear(resolvedClassId, undefined);
	const schoolStartDate = storedStartDate(await getSf2SchoolCalendarSettings(), schoolYear);
	const reportYear = reportYearForSchoolMonth(schoolYear, monthNumber, currentYear());

	// E2, checked before anything else so it is refused for the right reason.
	const firstSchoolDay = deriveFirstSchoolDay(schoolStartDate, monthNumber, reportYear);
	if (firstSchoolDay === undefined) throw appError('InvalidInput', NO_SCHOOL_DAYS_MESSAGE);

	if ((await findMonthTemplate(resolvedClassId, schoolYear, month)) !== undefined) {
		throw appError('InvalidInput', `The SF2 month ${month} is already on record.`);
	}

	// The one workbook, from the pre-split row: that is the file the merge builds and
	// the file every other month already names.
	const legacy = await latestLegacyTemplate(resolvedClassId);
	if (legacy === undefined) {
		throw appError(
			'InvalidInput',
			" There is no SF2 workbook for this class yet. Import the school's SF2 workbook first.".trim()
		);
	}
	if (!(await workbookOnDiskAt(legacy.sourcePath)).exists) {
		throw appError(
			'InvalidInput',
			`The SF2 workbook at ${legacy.sourcePath} is missing. Restore it from a backup.`
		);
	}

	// Roster carry-over. `previousSchoolMonth` walks the school year, so this is the
	// month before in the order classes actually meet. The roster is the same on
	// every sheet, so the nearest month that has one is as good a source as any.
	const months = schoolYearMonthFiles(schoolYear, currentYear());
	const previousIndex = months.findIndex((entry) => sf2MonthNumber(entry.month) === monthNumber);
	const previous =
		previousIndex > 0
			? await findMonthTemplate(resolvedClassId, schoolYear, months[previousIndex - 1].month)
			: undefined;
	const roster = previous === undefined ? [] : await monthRosterForTemplate(previous.id);

	const learners = numberTheRoster(
		roster.map((mapping) => ({
			studentId: mapping.studentId,
			rowIndex: mapping.rowIndex,
			name: mapping.workbookName,
			itemNumber: 0,
			genderBlock: mapping.genderBlock
		}))
	);

	const absences = await absencesForMonth(resolvedClassId, reportYear, monthNumber);
	const templateId = crypto.randomUUID();
	const report = await buildSchoolYearWorkbook(legacy.sourcePath, [
		{
			request: {
				templateId,
				reportMonth: month,
				reportYear,
				firstSchoolDay: gridAnchorDay(firstSchoolDay, reportYear, monthNumber),
				header: {
					schoolId: legacy.metadata.schoolId ?? '',
					schoolName: legacy.metadata.schoolName ?? '',
					schoolYear: legacy.metadata.schoolYear ?? schoolYear,
					reportMonth: month,
					gradeLevel: legacy.metadata.gradeLevel ?? '',
					section: legacy.metadata.section ?? '',
					adviserName: legacy.metadata.adviserName ?? '',
					schoolHeadName: legacy.metadata.schoolHeadName ?? ''
				},
				learners,
				absences,
				sourceFemaleStartRow: 0
			},
			// Additive: the other eleven worksheets are left alone.
			removeStaleSheets: false
		}
	]);
	const monthReport = report.months[0];
	if (monthReport === undefined || !isVerified(monthReport.verification)) {
		throw appError(
			'Internal',
			`The ${month} worksheet was not saved: ${mismatchReason(monthReport?.verification ?? { verified: false, expectedX: 0, foundX: 0, expectedLearners: 0, foundLearners: 0 }) ?? 'the build did not verify'}. Nothing was changed.`
		);
	}

	const now = Math.floor(Date.now() / 1000);
	await upsertMonthTemplate({
		id: templateId,
		classId: resolvedClassId,
		schoolYear,
		reportMonth: month,
		reportYear,
		sourcePath: legacy.sourcePath,
		sourceHash: legacy.sourceHash,
		// Every month of a class carries the same school, grade, section and adviser,
		// so the nearest month that has one describes this one too.
		schoolId: previous?.schoolId,
		schoolName: previous?.schoolName,
		gradeLevel: previous?.gradeLevel,
		section: previous?.section,
		adviserName: previous?.adviserName,
		schoolHeadName: previous?.schoolHeadName,
		firstSchoolDay,
		firstSchoolDayOverride: undefined,
		importedAt: now,
		lastSyncedAt: undefined,
		workbookXCount: monthReport.writtenMarks,
		// The build counted the marks it just wrote, so this month is measured rather
		// than "never scanned".
		workbookScannedAt: now
	});

	const dates = monthReport.dates.map((date) => ({
		templateId,
		date: date.date,
		columnLetter: date.columnLetter,
		columnIndex: date.columnIndex,
		sheetName: date.sheetName ?? resolvedSheetName(date)
	}));
	if (dates.length > 0) {
		await replaceMonthDateMappings(templateId, dates);
	}
	if (roster.length > 0) {
		await replaceMonthRoster(
			templateId,
			roster.map((mapping) => ({ ...mapping, templateId }))
		);
	}

	const stored = await findMonthTemplateById(templateId);
	if (stored === undefined) {
		throw appError('Internal', `the ${month} workbook row vanished after it was written`);
	}
	return stored;
}

/**
 * Re-run the one-time split of the pre-split workbook into the twelve-sheet
 * workbook.
 *
 * Idempotent, and deliberately without a `force` flag: the job decides for itself
 * which months still need building, so pressing it twice is safe, and a month that
 * is already split is never rebuilt over the marks the app has written into it
 * since. Months that still need a human are named in the returned `message`.
 */
export async function runSf2WorkbookSplit(): Promise<Sf2SplitOutcome> {
	return mergeWorkbooks();
}

// â”€â”€ Supporting detail â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/**
 * The problems a month read should surface, in the order a teacher needs them.
 *
 * Every one of these says "this month is not ready yet", which is the normal state
 * before the split has run. None of them says the database is wrong, and none of
 * them is a reason to write anything.
 */
function monthIssues(
	month: string,
	hasTemplate: boolean,
	fileExists: boolean,
	mappings: Sf2MonthDateMappingRecord[],
	roster: Sf2MonthStudentMapping[],
	classStudents: Student[]
): string[] {
	const issues: string[] = [];
	if (!hasTemplate) {
		issues.push(`No SF2 workbook is stored for ${month} yet. Create it to switch to this month.`);
	} else if (!fileExists) {
		issues.push(
			"This month's SF2 workbook is missing from disk. Restore it from a backup; nothing will be written to it until it is back."
		);
	}
	if (mappings.length === 0) {
		issues.push('No attendance days are mapped to this month yet.');
	}
	if (roster.length === 0) {
		issues.push("No learners are mapped to this month's SF2 workbook yet.");
	}
	const mapped = new Set(roster.map((mapping) => mapping.studentId));
	const unmapped = classStudents.filter((student) => !mapped.has(student.id)).length;
	if (unmapped > 0) {
		issues.push(
			`${unmapped} of the class's ${classStudents.length} students ${unmapped === 1 ? 'is' : 'are'} not mapped to this month's SF2 workbook.`
		);
	}
	return issues;
}

/**
 * The month row as the template summary the Reports page already reads.
 *
 * The same school, grade, section, adviser and school head on all twelve month
 * files of a class is the point: the sidebar's identity panel is fed from whichever
 * month is on screen.
 */
function templateSummaryFromMonth(template: Sf2MonthTemplate): Sf2TemplateSummary {
	return {
		id: template.id,
		sourcePath: template.sourcePath,
		schoolId: template.schoolId ?? '',
		schoolName: template.schoolName ?? '',
		schoolYear: template.schoolYear,
		reportMonth: template.reportMonth,
		gradeLevel: template.gradeLevel ?? '',
		section: template.section ?? '',
		adviserName: template.adviserName ?? '',
		schoolHeadName: template.schoolHeadName ?? '',
		classId: template.classId,
		importedAt: template.importedAt
	};
}

/**
 * The pre-split row as the template summary, with the month rewritten to the one on
 * screen.
 *
 * Only `reportMonth` is rewritten: the pre-split row carries whichever month was
 * last written to it, and putting that name on a grid full of another month's days
 * is the same lie in a different field.
 */
function templateSummaryFromLegacy(
	legacy: NonNullable<Awaited<ReturnType<typeof latestLegacyTemplate>>>,
	month: string,
	schoolYear: string
): Sf2TemplateSummary {
	return {
		id: legacy.id,
		sourcePath: legacy.sourcePath,
		schoolId: legacy.metadata.schoolId?.trim() ?? '',
		schoolName: legacy.metadata.schoolName?.trim() ?? '',
		schoolYear,
		reportMonth: month,
		gradeLevel: legacy.metadata.gradeLevel?.trim() ?? '',
		section: legacy.metadata.section?.trim() ?? '',
		adviserName: legacy.metadata.adviserName?.trim() ?? '',
		schoolHeadName: legacy.metadata.schoolHeadName?.trim() ?? '',
		classId: legacy.metadata.activeClassId ?? '',
		importedAt: legacy.metadata.importedAt
	};
}

/**
 * `No.` numbers are 1..n within each gender block — Rust's
 * `merge::number_the_roster`, which `merge.ts` inlines into its own roster
 * resolution and does not export.
 */
function numberTheRoster(learners: MonthLearnerWrite[]): MonthLearnerWrite[] {
	let male = 0;
	let female = 0;
	return learners.map((learner) => {
		const itemNumber = learner.genderBlock === 'FEMALE' ? (female += 1) : (male += 1);
		return { ...learner, itemNumber };
	});
}

/** Every absence the database holds for one month, as `(student, date)` pairs. */
async function absencesForMonth(
	classId: string,
	reportYear: number,
	monthNumber: number
): Promise<MonthAbsence[]> {
	const prefix = `${String(reportYear).padStart(4, '0')}-${String(monthNumber).padStart(2, '0')}`;
	const events = await listEventsForClassAndDateRange(
		classId,
		`${prefix}-01`,
		`${prefix}-${String(lastDayOfMonth(reportYear, monthNumber)).padStart(2, '0')}`
	);
	return events
		.filter((event) => event.type === 'absent')
		.map((event) => ({ studentId: event.studentId, date: localDateOf(event.timestamp) }));
}

/**
 * The local calendar day an ISO instant falls on.
 *
 * SQLite holds epoch seconds and the grid is keyed on the teacher's local date, so
 * this is where UTC becomes local. Reading the UTC day here is what puts an
 * evening's absence in the next column.
 */
function localDateOf(timestamp: string): string {
	const date = new Date(timestamp);
	return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(
		date.getDate()
	).padStart(2, '0')}`;
}
