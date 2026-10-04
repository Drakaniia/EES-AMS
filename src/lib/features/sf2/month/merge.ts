/**
 * The merge job: turn one pre-split workbook into the **one file with twelve
 * month worksheets**.
 *
 * ## What it does
 *
 * ```text
 * 1. Resolve the class's workbook identity from the pre-split `sf2_templates`
 *    row - school, grade, section, adviser, school year - and the one file that
 *    identity names.
 * 2. Preserve the pre-split workbook in `sf2-workbooks/_legacy/` *before
 *    anything is written*, and start the rebuild from the bundled DepEd template
 *    when there is nothing of the user's there yet.
 * 3. Read the pre-split workbook read-only, month by month, and compare each
 *    month's `X` marks with the database's absences for that month. A month whose
 *    sheet holds a mark the database cannot produce is reported and the file is
 *    not written: the pre-split workbook is the recovery source, so the recovery
 *    runs first.
 * 4. Build all twelve month worksheets - `month/workbook-builder.ts` - each with
 *    its own day-number grid, the one roster, its own header and its own `X`
 *    marks, then write the summary block (rows 53-71) that the build does not.
 * 5. Record twelve `sf2_month_templates` rows, twelve day grids carrying their
 *    `sheet_name`, and the roster; stamp `settings.sf2_split_completed_at` only
 *    when all twelve verified.
 * ```
 *
 * ## What it will not do
 *
 * * **It will not import a mark from a worksheet it cannot vouch for.**
 * * **It will not write a month with no school days.** A month with no day column
 *   gets a worksheet with an empty grid and zero marks. Never a fabricated day,
 *   and never a mark it invented.
 * * **It will not re-run over a finished merge.** Twelve verified rows naming the
 *   one file is the "done" state, and re-running is a no-op - the file may hold
 *   marks Excel wrote since.
 *
 * ## Naming
 *
 * The Rust types were named for the merge; the wire names are unchanged
 * (`Sf2SplitOutcome`, `run_sf2_workbook_split`, "Re-run the workbook split")
 * because the Settings screen that calls them is outside this change's scope.
 */

import ExcelJS from 'exceljs';
import type { Worksheet } from 'exceljs';
import { getDriver, internal, invalidInput } from '$lib/db';
import getSchoolStartDateSql from '$lib/db/sql/month_get_school_start_date.sql?raw';
import getSplitCompletedAtSql from '$lib/db/sql/month_get_split_completed_at.sql?raw';
import legacyFirstSchoolDaySql from '$lib/db/sql/month_legacy_first_school_day.sql?raw';
import latestTemplateForClassSql from '$lib/db/sql/latest_template_for_class.sql?raw';
import listTemplatesSql from '$lib/db/sql/list_templates.sql?raw';
import setLastReportMonthSql from '$lib/db/sql/month_set_last_report_month.sql?raw';
import setSplitCompletedAtSql from '$lib/db/sql/month_set_split_completed_at.sql?raw';
import studentMappingsForTemplateSql from '$lib/db/sql/student_mappings_for_template.sql?raw';
import { listEvents } from '$lib/db/repos/events';
import { getSettings } from '$lib/db/repos/settings';
import { createStudent, listStudents } from '$lib/db/repos/students';
import { nowEpochSeconds } from '$lib/domain/models';
import type { Student } from '$lib/domain/models';
import {
	SF2_ATTENDANCE_FIRST_COLUMN,
	SF2_ATTENDANCE_LAST_COLUMN,
	SF2_FIRST_LEARNER_ROW,
	bundledTemplateTotalRows,
	type Sf2SummaryCountsByColumn
} from '$lib/features/excel/constants';
import {
	learnerAbsentPresentFormulaMarks,
	summaryFormulaMarks,
	totalFormulaMarks,
	type Sf2DayColumn
} from '$lib/features/excel/formula-marks';
import { writeFormulaMarks, writeMarksForce } from '$lib/features/excel/marks';
import { isLearnerName } from '$lib/features/excel/roster';
import {
	cellText,
	columnLetter,
	numericCellValue,
	openWorkbook,
	saveWorkbookAtomic,
	type Sf2DayGrid
} from '$lib/features/excel/workbook';
import { dayNumbersForSlots, sf2MonthName } from '$lib/features/sf2/calendar';
import {
	currentYear,
	defaultSchoolStartDate,
	FIRST_SCHOOL_DAY_UNDETERMINED,
	gridAnchorDay,
	parseIsoDate,
	resolveFirstSchoolDay,
	schoolYearStartYear
} from '$lib/features/sf2/first-school-day';
import { SF2_ABSENT_MARK, normalizeLearnerName } from '$lib/features/sf2/logic';
import { monthSheetName, weekdaySlots } from '$lib/features/sf2/month/workbook-sheets';
import {
	matchRosterLearner,
	monthRosterForTemplate,
	replaceMonthRoster,
	setStudentLearnerIds,
	type Sf2MonthStudentMapping
} from '$lib/features/sf2/month/students';
import {
	buildSchoolYearWorkbook,
	mismatchReason,
	readLegacyMonths,
	type LegacyMonthSnapshot,
	type MonthBuildReport,
	type MonthSheetBuild
} from '$lib/features/sf2/month/workbook-builder';
import {
	countMonthDateMappings,
	deriveMonthFirstSchoolDay,
	findMonthTemplate,
	recordMonthWorkbookXCount,
	replaceMonthDateMappings,
	upsertMonthTemplate
} from '$lib/features/sf2/month/templates';
import {
	canonicalMonthName,
	getSf2WorkbookDir,
	sf2LegacyWorkbookDir,
	schoolYearMonthFiles,
	singleWorkbookPath,
	writeBundledTemplateTo
} from '$lib/features/sf2/workbook-files';
import { getFileSystem } from '$lib/platform/fs';
import type { Sf2MonthTemplate } from '$lib/types';
import type { Sf2SplitMonthOutcome, Sf2SplitOutcome } from '$lib/types';

/** How many months a school year has, and therefore how many must verify. */
export const SPLIT_MONTH_COUNT = 12;

/** The one file all twelve months live in. */
export type SplitWorkbook = {
	/** The class the workbook belongs to, and the school year it is dated in. */
	gradeLevel: string;
	section: string;
	templateId: string;
};

/** The pre-split workbook row: the class's workbook identity. */
type LegacyTemplate = {
	id: string;
	activeClassId: string;
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
};

/** One month of the school year, in the order the school year runs it. */
type SchoolYearMonth = { month: string; reportYear: number };

// ── Which months count ───────────────────────────────────────────────────────

/** Is this month accounted for - either merged now, or by an earlier run? */
export function isAccountedFor(month: Sf2SplitMonthOutcome): boolean {
	return month.status !== 'needsAttention';
}

/** The months the user has to do something about, named the way the report does. */
export function needsAttentionLabels(months: readonly Sf2SplitMonthOutcome[]): string[] {
	return months
		.filter((month) => !isAccountedFor(month))
		.map((month) => `${month.reportMonth} ${month.reportYear}`);
}

/** How many months are accounted for. */
export function verifiedCount(months: readonly Sf2SplitMonthOutcome[]): number {
	return months.filter(isAccountedFor).length;
}

/**
 * May the completion stamp be written?
 *
 * Only when every one of the twelve months is accounted for. A partial run leaves
 * it unset, which is what makes the next run a resume rather than a second,
 * conflicting build.
 */
export function isMergeComplete(months: readonly Sf2SplitMonthOutcome[]): boolean {
	return months.length === SPLIT_MONTH_COUNT && months.every(isAccountedFor);
}

/**
 * The sentence the Settings screen shows under *Re-run the workbook split*.
 *
 * Always says where the original workbook is kept, because the merge is the one
 * operation in this app that rewrites the file the original marks lived in, and
 * the user is entitled to know the original is still there.
 */
export function mergeSummaryMessage(
	completedAt: number | null | undefined,
	months: readonly Sf2SplitMonthOutcome[],
	legacyFileName: string
): string {
	const verified = verifiedCount(months);
	const attention = needsAttentionLabels(months);
	const kept =
		`All ${SPLIT_MONTH_COUNT} months are on one workbook, each on its own visible worksheet. ` +
		`The original workbook is kept in sf2-workbooks\\_legacy (${legacyFileName}).`;
	if (completedAt !== undefined && completedAt !== null) return `${kept} All ${verified} verified.`;
	if (attention.length === 0) return `${kept} ${verified} verified.`;
	return `${kept} ${verified} verified, ${attention.length} needs attention (${attention.join(', ')}).`;
}

// ── The roster ───────────────────────────────────────────────────────────────

/** Males run from row 8; a bundled template has room for 21 before it has to grow. */
const FIRST_MALE_ROW = 8;
const MALE_SLOTS = 21;
const FEMALE = 'FEMALE';

/**
 * The first learner row of the female block, for a roster of `maleCount` males.
 *
 * Males run from row 8 and the MALE TOTAL row sits at `8 + max(maleCount, 21)`,
 * which is the same arithmetic `bundledTemplateTotalRows` uses to place the TOTAL
 * rows - so the roster and the `COUNTIF` formulas agree by construction rather
 * than by two copies of a constant. It is what lets one row index mean the same
 * learner on all twelve sheets.
 */
export function femaleBlockStart(maleCount: number): number {
	return FIRST_MALE_ROW + 1 + Math.max(MALE_SLOTS, maleCount);
}

/** The one roster, shared by all twelve worksheets. */
export type ResolvedRoster = {
	/** The learner rows to write onto every sheet, in row order. */
	writes: {
		studentId: string;
		rowIndex: number;
		name: string;
		itemNumber: number;
		genderBlock: string;
	}[];
	/**
	 * The per-month mappings, in row order. Identical for all twelve months,
	 * because there is one class and one roster.
	 */
	mappings: Sf2MonthStudentMapping[];
	/** `studentId` -> `rowIndex`, for the read-only comparison. */
	rowByStudent: Map<string, number>;
};

/**
 * Turn the class's roster into the rows every one of the twelve worksheets is
 * written with.
 *
 * The order is the pre-split mapping's own row order, grouped by gender block, so
 * the roster the user already sees keeps its shape; row indices are then
 * re-derived from the block sizes.
 *
 * A learner the workbook only knows is created; a learner the app already has is
 * reused rather than duplicated. A learner already claimed by an earlier row is
 * never reused: the mapping table keys on `(template_id, student_id)`, and two
 * workbook rows pointing at one student would collide. Nothing here renames a
 * student - the worksheets get the names the pre-split workbook had, and the name
 * in the database is the teacher's to change.
 */
export async function resolveRoster(
	classId: string,
	reference: readonly Sf2MonthStudentMapping[],
	students: readonly Student[]
): Promise<ResolvedRoster> {
	const existingStudentIds = new Set(students.map((student) => student.id));
	const inReference = new Set(reference.map((mapping) => mapping.studentId));
	// A student the app has and the pre-split workbook does not: the teacher typed them
	// into the Students page. Without them here every sheet left them out and no X of
	// theirs could ever be written, which is the whole reason this list is the app's
	// roster and not the workbook's.
	const added = students.filter((student) => !inReference.has(student.id)).map(addedStudentMapping);
	const named = [...reference.filter((mapping) => isLearnerName(mapping.workbookName)), ...added];
	const femaleFirstRow = femaleBlockStart(
		named.filter((mapping) => mapping.genderBlock !== FEMALE).length
	);
	const ordered = [...named].sort(
		(left, right) =>
			(left.genderBlock === FEMALE ? 1 : 0) - (right.genderBlock === FEMALE ? 1 : 0) ||
			left.rowIndex - right.rowIndex
	);

	const writes: ResolvedRoster['writes'] = [];
	const mappings: Sf2MonthStudentMapping[] = [];
	const rowByStudent = new Map<string, number>();
	const seenNames = new Set<string>();
	const claimed = new Set<string>();
	let male = 0;
	let female = 0;

	for (const mapping of ordered) {
		const name = mapping.workbookName.trim();
		const isFemale = mapping.genderBlock === FEMALE;
		const matched = matchRosterLearner(ordered, {
			name,
			rowIndex: mapping.rowIndex,
			sf2LearnerId: mapping.sf2LearnerId
		});
		const studentId =
			matched !== undefined &&
			existingStudentIds.has(matched.studentId) &&
			!claimed.has(matched.studentId)
				? matched.studentId
				: (
						await createStudent({
							name,
							gender: isFemale ? 'female' : 'male',
							classId
						})
					).id;
		claimed.add(studentId);

		const rowIndex = isFemale ? femaleFirstRow + female : FIRST_MALE_ROW + male;
		if (isFemale) female += 1;
		else male += 1;
		rowByStudent.set(studentId, rowIndex);

		writes.push({
			studentId,
			rowIndex,
			name,
			itemNumber: isFemale ? female : male,
			genderBlock: isFemale ? FEMALE : 'MALE'
		});
		const normalized = name.replace(/\s+/g, ' ').replace(', ', ',').trim().toUpperCase();
		const unique = seenNames.has(normalized) ? `${normalized}#${mapping.rowIndex}` : normalized;
		seenNames.add(unique);
		mappings.push({
			// Filled in with the stored id by `recordMonth`.
			templateId: '',
			studentId,
			workbookName: name,
			normalizedName: unique,
			rowIndex,
			genderBlock: isFemale ? FEMALE : 'MALE',
			sf2LearnerId: mapping.sf2LearnerId
		});
	}
	return { writes, mappings, rowByStudent };
}

/**
 * Sorts after every real workbook row, so an added student lands at the end of their
 * gender block and no existing learner changes row when one is added.
 */
const LAST_ROW = Number.MAX_SAFE_INTEGER;

/**
 * A student the app has and the workbook does not, as a roster row that carries their
 * own id.
 *
 * The form has one block per gender and no third block, so a student with no gender
 * cannot be placed at all. That is refused here for the same reason
 * `templateRosterAssignments` refuses it: guessing would put a child in the wrong half
 * of a submitted form.
 */
function addedStudentMapping(student: Student): Sf2MonthStudentMapping {
	if (student.gender !== 'male' && student.gender !== 'female') {
		throw invalidInput(
			`Set Male/Female for these students before writing the SF2 workbook: ${student.name.trim()}`
		);
	}
	return {
		templateId: '',
		studentId: student.id,
		workbookName: student.name,
		normalizedName: normalizeLearnerName(student.name),
		rowIndex: LAST_ROW,
		genderBlock: student.gender === 'female' ? FEMALE : 'MALE'
	};
}

// ── §0 A5: the read-only comparison ──────────────────────────────────────────

/** One absence the database holds, before it has been given a cell. */
type MonthAbsence = { studentId: string; date: string };

/**
 * Every `absent` event the class holds, split by whether this school year has a
 * worksheet for it.
 *
 * ## The half that is not in the school year
 *
 * A school year is twelve months, SEPTEMBER -> AUGUST, so a database that predates
 * it - or that simply spans two of them - holds absences in months no worksheet
 * exists for. They are **not** silently dropped: they are counted and reported, so
 * a teacher is told the workbook cannot show them rather than concluding from a
 * complete-looking file that those absences were never recorded. They stay in the
 * database and in the reports grid, which reads `events` and needs no worksheet.
 */
export async function loadAbsences(
	classId: string,
	months: readonly SchoolYearMonth[]
): Promise<{ byMonth: Map<string, MonthAbsence[]>; outside: MonthAbsence[] }> {
	const byMonth = new Map<string, MonthAbsence[]>(
		months.map((month) => [month.month.toUpperCase(), [] as MonthAbsence[]])
	);
	const outside: MonthAbsence[] = [];
	for (const event of await listEvents()) {
		if (event.type !== 'absent' || event.classId !== classId) continue;
		// Local, not UTC: a teacher's month is their own calendar.
		const local = new Date(event.timestamp);
		const key = sf2MonthName(local.getMonth() + 1).toUpperCase();
		const date = `${local.getFullYear()}-${String(local.getMonth() + 1).padStart(2, '0')}-${String(local.getDate()).padStart(2, '0')}`;
		const bucket = months.some(
			(month) => month.month.toUpperCase() === key && month.reportYear === local.getFullYear()
		)
			? byMonth.get(key)
			: undefined;
		if (bucket === undefined) outside.push({ studentId: event.studentId, date });
		else bucket.push({ studentId: event.studentId, date });
	}
	return { byMonth, outside };
}

/**
 * Months the pre-split workbook holds marks the database cannot produce.
 *
 * A month with no pre-split worksheet is not in here: there is nothing in the
 * workbook to compare against, and this check is a comparison, not an assumption.
 * A `__SF2_HIDDEN_{n}` worksheet can never land in this set either - it carries no
 * month in its name, so `readLegacyMonths` never matches one, which is what makes
 * "never import the template's sample data" structural rather than a promise.
 */
export function unprovenMonths(
	preExisting: readonly LegacyMonthSnapshot[],
	absences: ReadonlyMap<string, MonthAbsence[]>,
	rowByStudent: ReadonlyMap<string, number>
): Map<string, string> {
	const studentByRow = new Map([...rowByStudent].map(([student, row]) => [row, student]));
	const unproven = new Map<string, string>();

	for (const snapshot of preExisting) {
		const month = snapshot.reportMonth.toUpperCase();
		const monthAbsences = absences.get(month);
		if (monthAbsences === undefined) continue;
		const placeable = new Set(
			monthAbsences
				.filter((absence) => rowByStudent.has(absence.studentId))
				.map((absence) => `${absence.studentId}|${absence.date}`)
		);
		const unmatched: string[] = [];
		for (const mark of snapshot.marks) {
			if (mark.value.trim().toUpperCase() !== SF2_ABSENT_MARK) continue;
			// A mark in a column the sheet prints no day for cannot be turned into a
			// date, so it cannot be compared. It is not counted as a missing database
			// record, because there is no record it could be.
			const day = snapshot.dayByColumn.get(mark.columnIndex);
			const studentId = studentByRow.get(mark.rowIndex);
			if (day === undefined || studentId === undefined) continue;
			const date = `${snapshot.reportYear}-${String(monthNumberOf(snapshot.reportMonth)).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
			if (!placeable.has(`${studentId}|${date}`)) unmatched.push(`row ${mark.rowIndex} (${date})`);
		}
		if (unmatched.length > 0) {
			unproven.set(
				month,
				`the original ${month} sheet holds ${unmatched.length} X mark(s) the database has no ` +
					`record of (first: ${unmatched[0]}). Nothing was written. Recover those absences from ` +
					'the workbook first, then run this again.'
			);
		}
	}
	return unproven;
}

// ── One month, recorded ──────────────────────────────────────────────────────

/**
 * Write the month's rows: the template row, the day grid, and the roster.
 *
 * All twelve point `sourcePath` at the same file, which is the whole of "one file,
 * twelve rows, one file named by all of them".
 */
async function recordMonth(
	legacy: LegacyTemplate,
	workbookPath: string,
	schoolYear: string,
	month: SchoolYearMonth,
	firstSchoolDay: number | undefined,
	report: MonthBuildReport,
	mappings: readonly Sf2MonthStudentMapping[]
): Promise<void> {
	const reportMonth = month.month.toUpperCase();
	const existing = await findMonthTemplate(legacy.activeClassId, schoolYear, reportMonth);
	const now = nowEpochSeconds();
	await upsertMonthTemplate({
		id: crypto.randomUUID(),
		classId: legacy.activeClassId,
		schoolYear,
		reportMonth,
		reportYear: month.reportYear,
		sourcePath: workbookPath,
		// The file is a copy of the bundled template, so it keeps the pre-split
		// row's hash and the layout check still recognises it.
		sourceHash: legacy.sourceHash,
		schoolId: nonEmpty(legacy.schoolId),
		schoolName: nonEmpty(legacy.schoolName),
		gradeLevel: nonEmpty(legacy.gradeLevel),
		section: nonEmpty(legacy.section),
		adviserName: nonEmpty(legacy.adviserName),
		schoolHeadName: nonEmpty(legacy.schoolHeadName),
		firstSchoolDay: firstSchoolDay ?? FIRST_SCHOOL_DAY_UNDETERMINED,
		firstSchoolDayOverride: existing?.firstSchoolDayOverride,
		importedAt: now,
		lastSyncedAt: existing?.lastSyncedAt ?? null,
		workbookXCount: report.writtenMarks,
		// The build counted the marks in the file it just wrote, so the month is
		// measured - not "never scanned" - from here on (spec 9.1, E4).
		workbookScannedAt: now
	});

	// The conflict target is the month, not the id, so a row the v22 backfill
	// already created survives with its own id. Read it back rather than assume.
	const stored = await findMonthTemplate(legacy.activeClassId, schoolYear, reportMonth);
	if (stored === undefined) {
		throw internal(`the ${reportMonth} ${month.reportYear} row could not be stored`);
	}

	// A derived day never overwrites one the user typed; the repository reports
	// that by updating nothing.
	if (firstSchoolDay !== undefined) await deriveMonthFirstSchoolDay(stored.id, firstSchoolDay);
	await recordMonthWorkbookXCount(stored.id, report.writtenMarks, now);

	if (report.dates.length === 0) {
		// A month with no day columns keeps whatever grid it already had. The grid is
		// not this job's to destroy, and an empty grid is the state every write path
		// refuses to act on anyway.
		console.warn(
			`workbook merge: ${reportMonth} ${month.reportYear} has no day columns in the sheet, ` +
				'so its stored day grid was left as it was'
		);
	} else {
		await replaceMonthDateMappings(
			stored.id,
			report.dates.map((date) => ({ ...date, sheetName: date.sheetName ?? report.sheetName }))
		);
	}

	const monthMappings = mappings.map((mapping) => ({ ...mapping, templateId: stored.id }));
	await replaceMonthRoster(stored.id, monthMappings);

	// §6.3's backfill: the DepEd IDs this roster proved, stored on the students so a
	// later run can recognise the learner without opening a workbook. The repository
	// refuses a pair that would give one student another learner's ID.
	await setStudentLearnerIds(
		monthMappings.flatMap((mapping) =>
			mapping.sf2LearnerId === undefined ? [] : [[mapping.studentId, mapping.sf2LearnerId]]
		)
	);
}

// ── The job ──────────────────────────────────────────────────────────────────

/** Everything the merge resolves for itself, so a test can hand it fixed answers. */
export type MergeJob = {
	/** The class the workbook belongs to, and the school year it is dated in. */
	identity: SplitWorkbook;
	/** Where the one workbook lives. */
	workbookPath: string;
	/** The pre-split workbook the comparison reads, when one exists. */
	legacySourcePath?: string;
	/** The enrolment counts behind the summary block. */
	summaryCounts: Sf2SummaryCountsByColumn;
	/** The clock, so the school-year fallback is injectable. */
	now?: () => number;
	/**
	 * Rebuild even when every month is already merged.
	 *
	 * The default is the safe one: a workbook that has been written to since the last
	 * merge is never rebuilt over, because whatever is in it now may be somebody's
	 * deliberate edit. A roster change is the one case where that is wrong - the rows
	 * have moved, so the file has to be rewritten or the new learner is missing from
	 * every sheet - and the caller says so explicitly rather than the job guessing.
	 */
	force?: boolean;
};

/**
 * The school year to build: the pre-split workbook's own label first, the settings
 * label second.
 */
async function resolveSchoolYear(legacy: LegacyTemplate): Promise<string> {
	const stored = legacy.schoolYear.trim();
	if (schoolYearStartYear(stored) !== undefined) return stored;
	const fallback = ((await getSettings()).schoolYear ?? '').trim();
	if (schoolYearStartYear(fallback) !== undefined) return fallback;
	throw invalidInput(
		`the workbook's school year (\`${stored}\`) holds no year, so the twelve month worksheets ` +
			'cannot be named'
	);
}

/**
 * The pre-split workbook row: the class's workbook identity.
 */
async function legacyTemplateRow(): Promise<LegacyTemplate> {
	const summaries = await getDriver().query<{ active_class_id: string | null }>(listTemplatesSql);
	const classId = summaries[0]?.active_class_id;
	if (classId === undefined || classId === null) {
		throw invalidInput("There is no SF2 workbook yet. Import the school's SF2 workbook first.");
	}
	const row = await getDriver().queryOne<Record<string, string | null>>(latestTemplateForClassSql, [
		classId
	]);
	if (row === undefined) throw invalidInput('The stored SF2 workbook could not be read.');
	const text = (value: string | null) => value ?? '';
	return {
		id: text(row.id),
		activeClassId: text(row.active_class_id),
		sourcePath: text(row.source_path),
		sourceHash: text(row.source_hash),
		schoolId: text(row.school_id),
		schoolName: text(row.school_name),
		schoolYear: text(row.school_year),
		reportMonth: text(row.report_month),
		gradeLevel: text(row.grade_level),
		section: text(row.section),
		adviserName: text(row.adviser_name),
		schoolHeadName: text(row.school_head_name)
	};
}

/**
 * The roster mappings the pre-split app had, as per-month mappings.
 *
 * The v22 backfill already copied them onto the pre-split template's own month row,
 * and those copies carry whatever DepEd learner ID the student already had. When
 * the backfill found nothing, the pre-split table is read directly.
 */
async function referenceMappings(legacyId: string): Promise<Sf2MonthStudentMapping[]> {
	const monthMappings = await monthRosterForTemplate(legacyId);
	if (monthMappings.length > 0) return monthMappings;
	const rows = await getDriver().query<{
		student_id: string;
		workbook_name: string;
		normalized_name: string;
		row_index: number;
		gender_block: string | null;
	}>(studentMappingsForTemplateSql, [legacyId]);
	return rows.map((row) => ({
		templateId: legacyId,
		studentId: row.student_id,
		workbookName: row.workbook_name,
		normalizedName: row.normalized_name,
		rowIndex: row.row_index,
		genderBlock: row.gender_block ?? undefined,
		sf2LearnerId: undefined
	}));
}

/**
 * Copy the original into `_legacy/` and return the copy's path.
 *
 * Unlike the pre-split job's best-effort copy, this one is **required**: the file
 * being rebuilt is the user's only known copy of the original marks, and this copy
 * is the artefact that protects them. A copy that already exists is left exactly as
 * it is - it is the snapshot of the pre-merge workbook, and overwriting it with a
 * later copy would replace the one artefact that holds the original marks.
 */
async function requirePreservedLegacyWorkbook(source: string): Promise<string> {
	const fileSystem = getFileSystem();
	if (!(await fileSystem.exists(source))) return source;
	const destination = `${await sf2LegacyWorkbookDir()}/${fileNameOf(source)}`;
	if (await fileSystem.exists(destination)) return destination;
	await fileSystem.writeFileAtomic(destination, await fileSystem.readFile(source));
	return destination;
}

function fileNameOf(path: string): string {
	const separator = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'));
	return separator < 0 ? path : path.slice(separator + 1);
}

function nonEmpty(value: string): string | undefined {
	const trimmed = value.trim();
	return trimmed === '' ? undefined : trimmed;
}

function monthNumberOf(reportMonth: string): number {
	const upper = reportMonth.trim().toUpperCase();
	const abbreviations = [
		'JAN',
		'FEB',
		'MAR',
		'APR',
		'MAY',
		'JUN',
		'JUL',
		'AUG',
		'SEP',
		'OCT',
		'NOV',
		'DEC'
	];
	const index = abbreviations.findIndex((abbreviation) => upper.includes(abbreviation));
	return index < 0 ? 0 : index + 1;
}

async function splitCompletedAt(): Promise<number | null> {
	const row = await getDriver().queryOne<{ sf2_split_completed_at: number | null }>(
		getSplitCompletedAtSql
	);
	return row?.sf2_split_completed_at ?? null;
}

async function setSplitCompletedAt(completedAt: number): Promise<void> {
	const updated = await getDriver().execute(setSplitCompletedAtSql, [completedAt]);
	if (updated === 0) {
		throw internal('the workbook finished but there is no settings row to record it on');
	}
}

async function legacyFirstSchoolDay(
	templateId: string,
	month: number
): Promise<number | undefined> {
	const row = await getDriver().queryOne<{ first_school_day: number | null }>(
		legacyFirstSchoolDaySql,
		[templateId, String(month).padStart(2, '0')]
	);
	const day = row?.first_school_day;
	return day === undefined || day === null || day === 0 ? undefined : day;
}

/**
 * Is every month of the school year naming the one workbook, with a day grid to
 * write into?
 *
 * The stored year has to be the one this school year gives the month, or a row
 * left over from a previous school year would read as merged. A month with no day
 * columns has not been merged either: every write path refuses a month whose grid
 * is empty, so calling it merged would leave the install with no way back short of
 * editing the database by hand.
 */
async function allMonthsMerged(
	classId: string,
	schoolYear: string,
	workbookPath: string,
	months: readonly SchoolYearMonth[]
): Promise<boolean> {
	if (!(await getFileSystem().exists(workbookPath))) return false;
	for (const month of months) {
		const row: Sf2MonthTemplate | undefined = await findMonthTemplate(
			classId,
			schoolYear,
			month.month.toUpperCase()
		);
		if (row === undefined) return false;
		if (row.sourcePath !== workbookPath) return false;
		if (row.reportYear !== month.reportYear) return false;
		if ((await countMonthDateMappings(row.id)) === 0) return false;
	}
	return true;
}

/**
 * Build the single twelve-sheet workbook from the class's pre-split one.
 *
 * Idempotent, resumable, and safe to re-run. A month that cannot be proven is
 * abandoned on its own and the file is not written at all - a partial file would be
 * worse than none, because the pre-split workbook is what the user recovers from.
 */
export async function mergeWorkbookYear(job: MergeJob): Promise<Sf2SplitOutcome> {
	const legacy = await legacyTemplateRow();
	const schoolYear = await resolveSchoolYear(legacy);
	const months = schoolYearMonthFiles(schoolYear, job.now?.() ?? currentYear());
	const workbookPath = job.workbookPath;
	const fileName = fileNameOf(workbookPath);
	const classId = legacy.activeClassId;

	const completedAt = await splitCompletedAt();
	if (
		job.force !== true &&
		completedAt !== null &&
		(await allMonthsMerged(classId, schoolYear, workbookPath, months))
	) {
		// Done, and every month still names the one file. Doing nothing is the correct
		// answer and the only safe one: the file may have been written to since.
		return alreadyMergedOutcome(completedAt, months, workbookPath, job.legacySourcePath);
	}

	const legacyCopy = await requirePreservedLegacyWorkbook(
		job.legacySourcePath ?? legacy.sourcePath
	);
	const roster = await resolveRoster(
		classId,
		await referenceMappings(legacy.id),
		await listStudents(classId)
	);
	const { byMonth, outside } = await loadAbsences(classId, months);

	const preExisting =
		job.legacySourcePath === undefined
			? []
			: (
					await readLegacyMonths(
						job.legacySourcePath,
						months.map((month) => ({ reportMonth: month.month, reportYear: month.reportYear }))
					)
				)
					.map((read) => read.snapshot)
					.filter((snapshot): snapshot is LegacyMonthSnapshot => snapshot !== undefined);
	const unproven = unprovenMonths(preExisting, byMonth, roster.rowByStudent);

	const schoolStartDateRow = await getDriver().queryOne<{ school_start_date: string | null }>(
		getSchoolStartDateSql
	);
	const schoolStartDate =
		parseIsoDate(schoolStartDateRow?.school_start_date ?? '') ?? defaultSchoolStartDate(schoolYear);

	const builds: MonthSheetBuild[] = [];
	const resolvedDays: (number | undefined)[] = [];
	for (const month of months) {
		const monthNumber = monthNumberOf(month.month);
		const existing = await findMonthTemplate(classId, schoolYear, month.month.toUpperCase());
		const firstSchoolDay = resolveFirstSchoolDay(
			schoolStartDate,
			monthNumber,
			month.reportYear,
			existing?.firstSchoolDayOverride,
			await legacyFirstSchoolDay(legacy.id, monthNumber)
		);
		resolvedDays.push(firstSchoolDay);
		builds.push({
			request: {
				// Provisional: the row may already exist under another id, and the real
				// one is read back by `recordMonth`.
				templateId: crypto.randomUUID(),
				reportMonth: month.month.toUpperCase(),
				reportYear: month.reportYear,
				firstSchoolDay: gridAnchorDay(firstSchoolDay, month.reportYear, monthNumber),
				header: {
					schoolId: legacy.schoolId,
					schoolName: legacy.schoolName,
					schoolYear: legacy.schoolYear,
					// Each worksheet carries **its own** month in the header. The retired
					// model wrote one report month to every sheet in the file, which under
					// twelve months would label eleven of them with the wrong one.
					reportMonth: sf2MonthName(monthNumber),
					gradeLevel: legacy.gradeLevel,
					section: legacy.section,
					adviserName: legacy.adviserName,
					schoolHeadName: legacy.schoolHeadName
				},
				learners: roster.writes,
				absences: byMonth.get(month.month.toUpperCase()) ?? [],
				// The roster is laid out the way a fresh bundled template is, so the same
				// row index means the same learner on all twelve sheets.
				sourceFemaleStartRow: 0
			},
			// The full build removes the template's sample sheets and any leftover
			// `__SF2_HIDDEN_*`; a single-month build leaves the other eleven alone.
			removeStaleSheets: true
		});
	}

	if (!(await getFileSystem().exists(workbookPath))) {
		// Nothing of the user's is here yet, so there is nothing to read and nothing
		// to lose: start from the bundled template.
		await writeBundledTemplateTo(workbookPath);
	}

	const blocked = [...unproven.values()][0];
	if (blocked !== undefined) {
		return mergeOutcome(
			null,
			blockedMonths(months, fileName, roster.writes.length, unproven),
			workbookPath,
			legacyCopy,
			outside.length
		);
	}

	const report = await buildSchoolYearWorkbook(workbookPath, builds, job.summaryCounts);

	const verified = report.verification.verified;
	const results: Sf2SplitMonthOutcome[] = [];
	for (const [index, month] of months.entries()) {
		const monthReport = report.months[index];
		const row: Sf2SplitMonthOutcome = {
			reportMonth: month.month.toUpperCase(),
			reportYear: month.reportYear,
			sheetName: monthReport.sheetName,
			fileName,
			status: verified ? 'verified' : 'needsAttention',
			xMarks: verified ? monthReport.writtenMarks : 0,
			learnerRows: roster.writes.length,
			detail: verified
				? null
				: `The workbook was not saved: ${mismatchReason(monthReport.verification) ?? 'the build did not verify'}. ` +
					'Nothing was changed, and the original workbook still holds every mark.'
		};
		if (verified) {
			try {
				await recordMonth(
					legacy,
					workbookPath,
					schoolYear,
					month,
					resolvedDays[index],
					monthReport,
					roster.mappings
				);
			} catch (error) {
				row.status = 'needsAttention';
				row.xMarks = 0;
				row.detail = error instanceof Error ? error.message : String(error);
				console.error(`workbook merge: ${row.reportMonth} ${row.reportYear} needs attention`);
			}
		}
		results.push(row);
	}

	// The completion stamp, and only once all twelve are fine.
	const stamp = verified && isMergeComplete(results) ? nowEpochSeconds() : null;
	if (stamp !== null) await setSplitCompletedAt(stamp);
	const lastReportMonth = canonicalMonthName(legacy.reportMonth);
	if (lastReportMonth !== '') await getDriver().execute(setLastReportMonthSql, [lastReportMonth]);

	return mergeOutcome(stamp, results, workbookPath, legacyCopy, outside.length);
}

/** The outcome of a merge that has nothing left to do. */
function alreadyMergedOutcome(
	completedAt: number | null,
	months: readonly SchoolYearMonth[],
	workbookPath: string,
	legacySourcePath: string | undefined
): Sf2SplitOutcome {
	const fileName = fileNameOf(workbookPath);
	const results: Sf2SplitMonthOutcome[] = months.map((month) => ({
		reportMonth: month.month.toUpperCase(),
		reportYear: month.reportYear,
		sheetName: monthSheetName(monthNumberOf(month.month), month.reportYear),
		fileName,
		status: 'alreadyMerged',
		xMarks: 0,
		learnerRows: 0,
		detail: null
	}));
	return mergeOutcome(completedAt, results, workbookPath, legacySourcePath ?? workbookPath, 0);
}

/**
 * Every month reported as blocked by the read-only comparison.
 *
 * A month with nothing to prove is `alreadyMerged` rather than `needsAttention`:
 * nothing about it is wrong, there is simply nothing this run needed to do.
 */
function blockedMonths(
	months: readonly SchoolYearMonth[],
	fileName: string,
	learnerRows: number,
	unproven: ReadonlyMap<string, string>
): Sf2SplitMonthOutcome[] {
	return months.map((month) => {
		const key = month.month.toUpperCase();
		const detail = unproven.get(key);
		return {
			reportMonth: key,
			reportYear: month.reportYear,
			sheetName: '',
			fileName,
			status: detail === undefined ? 'alreadyMerged' : 'needsAttention',
			xMarks: 0,
			learnerRows,
			detail: detail ?? null
		};
	});
}

function mergeOutcome(
	completedAt: number | null,
	months: Sf2SplitMonthOutcome[],
	workbookPath: string,
	legacyCopy: string,
	absencesOutsideSchoolYear: number
): Sf2SplitOutcome {
	let message = mergeSummaryMessage(completedAt, months, fileNameOf(legacyCopy));
	if (absencesOutsideSchoolYear > 0) {
		// Said out loud, because the alternative is a complete-looking workbook and a
		// teacher concluding those absences were never recorded.
		message +=
			` ${absencesOutsideSchoolYear} absence(s) the app holds fall in months this school year ` +
			'has no worksheet for; they are in the database and in the reports grid, but this ' +
			'workbook cannot show them.';
	}
	return {
		splitCompletedAt: completedAt,
		months,
		verifiedCount: verifiedCount(months),
		needsAttentionCount: months.filter((month) => !isAccountedFor(month)).length,
		workbookPath,
		legacyFilePath: legacyCopy,
		legacyBackupPath: null,
		absencesOutsideSchoolYear,
		message
	};
}

/**
 * `run_sf2_workbook_split` - the entry point the Settings screen calls, with
 * everything resolved from the app's own configuration.
 */
export async function mergeWorkbooks(options: { force?: boolean } = {}): Promise<Sf2SplitOutcome> {
	const workbookDir = await getSf2WorkbookDir();
	const legacy = await legacyTemplateRow();
	const identity: SplitWorkbook = {
		gradeLevel: legacy.gradeLevel,
		section: legacy.section,
		templateId: legacy.id
	};
	return mergeWorkbookYear({
		identity,
		workbookPath: singleWorkbookPath(workbookDir, identity),
		legacySourcePath: legacy.sourcePath,
		force: options.force,
		// The counts are the school's own enrolment movements; until they are read from
		// a roster import the summary block keeps the template's zeros rather than
		// inventing any.
		summaryCounts: {
			AR: { lateEnrolment: 0, droppedOut: 0, transferredOut: 0, transferredIn: 0 },
			AS: { lateEnrolment: 0, droppedOut: 0, transferredOut: 0, transferredIn: 0 },
			AT: { lateEnrolment: 0, droppedOut: 0, transferredOut: 0, transferredIn: 0 }
		}
	});
}
