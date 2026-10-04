/**
 * Build the month worksheets of one SF2 workbook, and read the marks out of a
 * legacy workbook without ever touching it.
 *
 * Two things live here, and they are deliberately in one file because they are the
 * two halves of the same conversation with the file:
 *
 * - {@link buildSchoolYearWorkbook} - writes the month worksheets of the one file
 *   the class has, dates each one, writes the header, copies the roster, and
 *   writes the `X` marks from the absences it was handed. It **verifies before it
 *   saves**: a build where any month came back wrong is not written out, so the
 *   file on disk is left exactly as it was.
 * - {@link readLegacyMonths} - one read per learner row, for every month, of a
 *   workbook that is only ever opened for reading.
 *
 * ## Where the marks come from, and where they cannot come from
 *
 * From the `MonthBuildRequest.absences` the caller resolved out of `events`, and
 * never from a worksheet. The bundled DepEd template ships `X` marks of its own
 * on a roster of names that are not this teacher's students. Reading a month out
 * of a sheet is therefore the one operation this project must never perform: it
 * would turn the template's fiction into the user's attendance, and from then on
 * every mark count and every comparison would be measuring it.
 *
 * `readLegacyMonths` exists for the *opposite* purpose - it is how the merge job
 * reads what the user's own workbook holds so it can prove, before writing, that
 * the database holds at least as much - and it only ever matches a worksheet whose
 * name parses to a real month of the school year.
 *
 * ## What the COM layer bought, and what this port does not carry over
 *
 * The Rust verified each month with `Application.Evaluate` (`COUNTIF` over a
 * band, `COUNTA` over the name column) and read a band with one `TEXTJOIN`
 * formula per chunk, because ~3,300 COM round-trips per month is minutes of wall
 * clock during which Excel looks hung. There is no round-trip here: a band is a
 * loop over the cells ExcelJS already parsed. So the chunking, the formula
 * builders and the token parser have nothing left to do - verification counts the
 * addressable day columns of the learner rows it just wrote, which is exactly what
 * `COUNTIF` counted, once per merged pair rather than twice.
 *
 * Errors are thrown rather than returned. Every module under
 * `$lib/features/excel` throws, and a second convention in the layer would cost
 * more than it saves; the Rust `AppError` kinds are preserved in the messages.
 */

import type { Workbook, Worksheet } from 'exceljs';
import {
	bundledTemplateTotalRows,
	SF2_ATTENDANCE_FIRST_COLUMN,
	SF2_ATTENDANCE_LAST_COLUMN,
	SF2_DAY_ROW,
	SF2_FIRST_LEARNER_ROW,
	SF2_FRESH_FEMALE_SLOTS,
	SF2_FRESH_FEMALE_START_ROW,
	SF2_FRESH_MALE_SLOTS,
	SF2_ITEM_NUMBER_COLUMN,
	SF2_METADATA_CELLS,
	SF2_NAME_COLUMN,
	type Sf2SummaryCountsByColumn
} from '$lib/features/excel/constants';
import {
	cellAddress,
	cellText,
	columnLetter,
	columnNumber,
	getCellText,
	monthNumber,
	openWorkbook,
	readDayGrid,
	saveWorkbookAtomic,
	writableDayColumns,
	yearFromSheetName
} from '$lib/features/excel/workbook';
import { applyMarks, writeFormulaMarks, writeMarksForce } from '$lib/features/excel/marks';
import { countAbsentMarks, SF2_ABSENT_MARK } from '$lib/features/excel/formulas';
import {
	learnerAbsentPresentFormulaMarks,
	totalFormulaMarks,
	type Sf2DayColumn,
	type Sf2TotalRows
} from '$lib/features/excel/formula-marks';
import { hideEmptyLearnerRowsOnSheet, readLearnerRows } from '$lib/features/excel/roster';
import type { Sf2CellMark, Sf2LearnerRow } from '$lib/features/excel/types';
import {
	copyFormSheet,
	donorFormSheet,
	emptyMonthSheet,
	growRosterRows,
	helperSheetNames,
	isMonthSheetOf,
	makeMonthSheetsVisible,
	monthSheetName,
	prepareMonthSheet,
	removeNonMonthFormSheets,
	sheetEntries,
	totalRowsOnSheet,
	weekdaySlots,
	worksheetByName,
	writeTotalLabels,
	type MonthDaySlot,
	type PreparedMonthSheet
} from './workbook-sheets';
import { writeSummaryBlock } from './summary-block';

export type { MonthDaySlot };

/** The header block, copied from the legacy workbook's own template row. */
type MonthHeader = {
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
};

/** One learner row to write onto every one of the twelve worksheets. */
export type MonthLearnerWrite = {
	/** The `students.id` this row is, so a mark is never addressed by name. */
	studentId: string;
	rowIndex: number;
	name: string;
	/** The `No.` cell value, 1-based within its gender block. */
	itemNumber: number;
	/** `MALE` / `FEMALE`, which is what decides the band this row is read in. */
	genderBlock?: string;
};

/** One cell to copy verbatim into a month sheet's attendance grid. */
type MonthMarkWrite = {
	rowIndex: number;
	/** 1-based Excel column. */
	columnIndex: number;
	value: string;
};

/**
 * One absence the database holds, before it has been given a cell.
 *
 * Deliberately *not* a cell address. The address is `(student -> row) x (date ->
 * column)`, and both of those are only knowable once the sheet's roster and day
 * grid have been laid out - so the build resolves them, and the same arithmetic
 * that writes the grid resolves the marks. That is what makes it impossible for a
 * mark to land in a column the sheet's own `COUNTIF` formulas do not count.
 */
export type MonthAbsence = {
	studentId: string;
	/** `YYYY-MM-DD`. */
	date: string;
};

/**
 * One day column of one month, as stored in `sf2_month_date_mappings`.
 *
 * Declared here rather than imported so this module stands alone; it is the same
 * row `month/month.ts` reads, so the two are assignable to one another.
 */
type MonthDateMapping = {
	templateId: string;
	/** `YYYY-MM-DD`. */
	date: string;
	columnLetter: string;
	columnIndex: number;
	/** The worksheet this day is written to, e.g. `SEPTEMBER 2026`. */
	sheetName?: string;
};

/** Everything the build needs to write one month's worksheet. */
export type MonthBuildRequest = {
	/** The `sf2_month_templates.id` this month's grid is recorded against. */
	templateId: string;
	reportMonth: string;
	reportYear: number;
	firstSchoolDay: number;
	header: MonthHeader;
	learners: MonthLearnerWrite[];
	/** The `X` marks to write, resolved from `absences` against the grid laid out. */
	absences: MonthAbsence[];
	/**
	 * Where the female block starts in the source roster, so the sheet is grown to
	 * the roster's shape and the row indices above mean the same thing on all twelve
	 * sheets.
	 */
	sourceFemaleStartRow: number;
};

/** The result of comparing a freshly built month against its source. */
type MonthVerification =
	| { verified: true }
	| {
			verified: false;
			expectedX: number;
			foundX: number;
			expectedLearners: number;
			foundLearners: number;
	  };

/** Did the month come back exactly as the source had it? */
export function isVerified(verification: MonthVerification): boolean {
	return verification.verified;
}

/** One line naming the two numbers that disagreed, for the log and the UI. */
export function mismatchReason(verification: MonthVerification): string | undefined {
	if (verification.verified) return undefined;
	return (
		`copied ${verification.foundX} of ${verification.expectedX} X marks and ` +
		`${verification.foundLearners} of ${verification.expectedLearners} learner rows`
	);
}

/** A contiguous run of learner rows, read or written as one range. */
export type RowBand = {
	firstRow: number;
	lastRow: number;
	firstColumn: number;
	lastColumn: number;
};

/** What one month's worksheet ended up holding, and whether it matched. */
export type MonthBuildReport = {
	/** The worksheet the month was written to, named `"{MONTH} {year}"`. */
	sheetName: string;
	/**
	 * The grid the month was laid out over, ready for `sf2_month_date_mappings` -
	 * each row carrying the `sheetName` it is written to, which is what makes a
	 * twelve-sheet file addressable.
	 */
	dates: MonthDateMapping[];
	/** How many cells were written into the attendance grid. */
	writtenMarks: number;
	/** How many extra roster rows the sheet had to grow by. */
	extraRosterRows: number;
	/**
	 * Absences that named no learner on the roster, or no day column on the grid,
	 * and so were **not** written.
	 *
	 * Reported rather than thrown: a dropped absence makes the month fail
	 * verification on its own, which is what stops the file being saved. A build
	 * that threw would lose the other eleven months' work along with the report.
	 */
	unmappedAbsences: { students: number; dates: number };
	verification: MonthVerification;
};

/** One month of the single-file build. */
export type MonthSheetBuild = {
	/** What to write into this month's worksheet. */
	request: MonthBuildRequest;
	/**
	 * Delete every SF2-form worksheet in the file that is not one of the months
	 * being built.
	 *
	 * `true` for the twelve-month build, which is what removes a leftover
	 * `__SF2_HIDDEN_{n}` from the retired hide/rename/clear cycle and the bundled
	 * template's own sample sheets. `false` when only one month is being added to a
	 * workbook that already has its other months - the "create this month" path -
	 * where deleting the other eleven would be the data loss this whole project
	 * exists to prevent.
	 */
	removeStaleSheets: boolean;
};

/** What the build of one file ended up holding, and whether it may be saved. */
export type SchoolYearBuildReport = {
	/** One entry per month built, in the order they were built. */
	months: MonthBuildReport[];
	/** Worksheets removed because they were not one of the built months. */
	removedSheets: string[];
	/** Non-monthly worksheets still in the workbook. */
	keptHelperSheets: string[];
	/** Did every month come back exactly as it was written? */
	verification: MonthVerification;
};

/** The worksheet names this build wrote, in the order it wrote them. */
export function sheetNamesOf(report: SchoolYearBuildReport): string[] {
	return report.months.map((month) => month.sheetName);
}

/** Total `X` marks written across every month of the file. */
export function totalMarks(report: SchoolYearBuildReport): number {
	return report.months.reduce((total, month) => total + month.writtenMarks, 0);
}

// ── Pure geometry ────────────────────────────────────────────────────────────

const MS_PER_DAY = 86_400_000;

function utcDate(year: number, month: number, day: number): Date {
	return new Date(Date.UTC(year, month - 1, day));
}

function addDays(date: Date, days: number): Date {
	return new Date(date.getTime() + days * MS_PER_DAY);
}

/** Monday-Friday as 0-4, and `undefined` for the weekend. */
function weekdayIndex(date: Date): number | undefined {
	const index = (date.getUTCDay() + 6) % 7;
	return index < 5 ? index : undefined;
}

function daysInMonth(year: number, month: number): number {
	return new Date(Date.UTC(year, month, 0)).getUTCDate();
}

function isoDate(year: number, month: number, day: number): string {
	return `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
}

/**
 * The Monday of the week `date` falls in, whether or not `date` is a school day.
 *
 * The day grid counts school weeks *from* this date, so it has to be a Monday even
 * when the first day of the month is not one. A month that opens on a weekend -
 * November 2026 opens on a Sunday, say - has no weekday to step back from, and
 * taking the previous school day first is what keeps that month's grid from coming
 * out **empty**: every one of its day columns would otherwise be blank, and a
 * school term's worth of attendance with nowhere to record it.
 */
function mondayAnchorFor(date: Date): Date {
	const weekday = date.getUTCDay();
	const previousSchoolDay = addDays(date, weekday === 6 ? -1 : weekday === 0 ? -2 : 0);
	return addDays(previousSchoolDay, -((previousSchoolDay.getUTCDay() + 6) % 7));
}

/**
 * Which week of the form a date falls in, counting from the Monday on or before the
 * first attendance day.
 *
 * A date before the anchor answers negative, which no slot can match - the anchor is
 * the first attendance day itself or earlier, so this cannot happen in practice, and
 * a wrong day is far worse than no day.
 */
function weekNumber(date: Date, mondayAnchor: Date): number {
	return Math.trunc((date.getTime() - mondayAnchor.getTime()) / MS_PER_DAY / 7);
}

/**
 * The day number to print in each day column, or `undefined` for a column the month
 * does not reach.
 *
 * A column is left blank when the month has no school day in that (week, weekday)
 * cell - most of a month's first week, and all of its tail. A month needing more
 * days than the form has columns simply has its last days left off the sheet.
 */
export function dayNumbersForSlots(
	reportYear: number,
	reportMonth: number,
	firstSchoolDay: number,
	slots: readonly MonthDaySlot[]
): { column: number; day: number | undefined }[] {
	const lastDay = daysInMonth(reportYear, reportMonth);
	const anchor =
		firstSchoolDay >= 1 && firstSchoolDay <= lastDay
			? mondayAnchorFor(utcDate(reportYear, reportMonth, firstSchoolDay))
			: undefined;
	// One pass over the month's school days, keyed the way a slot asks for them.
	const placed = new Map<string, number>();
	if (anchor !== undefined) {
		for (let day = firstSchoolDay; day <= lastDay; day += 1) {
			const date = utcDate(reportYear, reportMonth, day);
			const weekday = weekdayIndex(date);
			if (weekday === undefined) continue;
			placed.set(`${weekNumber(date, anchor)}:${weekday}`, day);
		}
	}
	return slots.map((slot) => ({
		column: slot.column,
		day: placed.get(`${slot.weekIndex}:${slot.weekdayIndex}`)
	}));
}

/**
 * The school days of a month that the form has no day column for.
 *
 * The DepEd SF2 grid is **five weeks of Monday..Friday - 25 labelled day cells** -
 * and the ABSENT/PRESENT block begins in the very next column, so a sixth week has
 * nowhere to go. Read off the bundled template's own weekday header, the
 * unlabelled columns are the second halves of merged weekday pairs.
 *
 * That is 25 slots for a month that can hold at most **23** Monday-Friday days (a
 * 31-day month starting on a Monday is the maximum), so nothing is ever dropped.
 * The check is exported and swept over every month of a century because "it cannot
 * happen" is the weakest possible guard for the one failure that costs a user a
 * term of marks: a school day silently missing from the grid is a day the user
 * cannot record an absence on, and it looks like a holiday nobody took.
 */
export function daysWithoutASlot(
	reportYear: number,
	reportMonth: number,
	firstSchoolDay: number,
	slots: readonly MonthDaySlot[]
): number[] {
	const placed = new Set(
		dayNumbersForSlots(reportYear, reportMonth, firstSchoolDay, slots)
			.map((entry) => entry.day)
			.filter((day): day is number => day !== undefined)
	);
	const missing: number[] = [];
	for (let day = firstSchoolDay; day <= daysInMonth(reportYear, reportMonth); day += 1) {
		if (weekdayIndex(utcDate(reportYear, reportMonth, day)) === undefined) continue;
		if (!placed.has(day)) missing.push(day);
	}
	return missing;
}

/**
 * The `sf2_month_date_mappings` rows implied by a month sheet's day columns.
 *
 * Derived from the same numbers the sheet was written with, so the grid the database
 * records and the grid on the page are the same grid by construction - there is no
 * second read of the workbook that could disagree with the first.
 */
export function monthDateMappings(
	templateId: string,
	reportYear: number,
	reportMonth: number,
	firstSchoolDay: number,
	slots: readonly MonthDaySlot[]
): MonthDateMapping[] {
	const sheetName = monthSheetName(reportMonth, reportYear);
	const mappings: MonthDateMapping[] = [];
	const grid = dayNumbersForSlots(reportYear, reportMonth, firstSchoolDay, slots);
	for (const { column, day } of grid) {
		if (day === undefined) continue;
		mappings.push({
			templateId,
			date: isoDate(reportYear, reportMonth, day),
			columnLetter: columnLetter(column),
			columnIndex: column,
			sheetName
		});
	}
	return mappings;
}

/**
 * The roster shape this month must end up in, and where its TOTAL rows land.
 *
 * The Rust derived the TOTAL rows from the learner counts alone, which put them in
 * the wrong place whenever the `sourceFemaleStartRow` reason below grew the male
 * block - a sheet grown by four rows has its MALE TOTAL on row 33, not 32.
 */
function rosterTarget(
	maleCount: number,
	femaleCount: number,
	sourceFemaleStartRowValue: number
): { maleCapacity: number; femaleCapacity: number; totalRows: Sf2TotalRows } {
	const { extraMale, extraFemale } = rosterExpansionFor(
		maleCount,
		femaleCount,
		sourceFemaleStartRowValue
	);
	const maleCapacity = SF2_FRESH_MALE_SLOTS + extraMale;
	const femaleCapacity = SF2_FRESH_FEMALE_SLOTS + extraFemale;
	return {
		maleCapacity,
		femaleCapacity,
		totalRows: bundledTemplateTotalRows(maleCapacity, femaleCapacity)
	};
}

/**
 * Normalise a gender block onto the label the form's TOTAL rows use.
 *
 * A roster read off a worksheet carries `'M'` / `'F'` (the form has no gender
 * column, only the MALE/FEMALE TOTAL dividers), while a roster about to be
 * written carries the block label itself. Both answer the same question here.
 */
function bandLabel(gender: string | undefined): string {
	if (gender === 'M' || gender === 'MALE') return 'MALE';
	if (gender === 'F' || gender === 'FEMALE') return 'FEMALE';
	return '';
}

function bandsFrom(rows: readonly { row: number; gender?: string }[]): RowBand[] {
	const bands: RowBand[] = [];
	for (const block of ['MALE', 'FEMALE']) {
		const blockRows = rows
			.filter((entry) => bandLabel(entry.gender) === block)
			.map((entry) => entry.row);
		if (blockRows.length === 0) continue;
		bands.push({
			firstRow: Math.min(...blockRows),
			lastRow: Math.max(...blockRows),
			firstColumn: SF2_ATTENDANCE_FIRST_COLUMN,
			lastColumn: SF2_ATTENDANCE_LAST_COLUMN
		});
	}
	return bands;
}

/**
 * The learner rows of one month's sheet, as one band per gender block.
 *
 * Splitting by gender block is what keeps the MALE/FEMALE TOTAL rows - `SUM`
 * formulas, not marks - out of the read. A roster with no female learners produces a
 * single band.
 */
export function attendanceBands(learners: readonly Sf2LearnerRow[]): RowBand[] {
	return bandsFrom(learners);
}

/** The same bands, for a roster that is about to be written rather than read. */
export function attendanceBandsForWrites(learners: readonly MonthLearnerWrite[]): RowBand[] {
	return bandsFrom(
		learners.map((learner) => ({ row: learner.rowIndex, gender: learner.genderBlock }))
	);
}

/**
 * The first row of a source roster's female block, or a fresh template's own when the
 * roster has no female learners.
 */
export function sourceFemaleStartRow(learners: readonly Sf2LearnerRow[]): number {
	const female = learners.filter((learner) => learner.gender === 'F').map((row) => row.row);
	return female.length > 0 ? Math.min(...female) : SF2_FRESH_FEMALE_START_ROW;
}

/**
 * How many extra roster rows a month file must grow by to hold a source roster.
 *
 * Two independent reasons to grow, and both are honoured:
 *
 * * the source roster is longer than the form's own capacity, and
 * * the source's female block starts lower than a fresh template's, which means the
 *   legacy file was already expanded and its row indices have moved.
 *
 * The second reason is not cosmetic: without it, row 30 means a different learner in
 * the two files and every copied `X` would land on the wrong student.
 */
export function rosterExpansionFor(
	maleCount: number,
	femaleCount: number,
	sourceFemaleStartRowValue: number
): { extraMale: number; extraFemale: number } {
	return {
		extraMale: Math.max(
			0,
			maleCount - SF2_FRESH_MALE_SLOTS,
			sourceFemaleStartRowValue - SF2_FRESH_FEMALE_START_ROW
		),
		extraFemale: Math.max(0, femaleCount - SF2_FRESH_FEMALE_SLOTS)
	};
}

/**
 * How far the header rows below the roster move when the roster grows.
 *
 * An inserted row pushes everything below it down, and the adviser and school head
 * signature blocks sit below the roster. Without this the header would be written
 * over the wrong cells on any expanded workbook.
 */
export function headerRowShift(extraMale: number, extraFemale: number): number {
	return extraMale + extraFemale;
}

/**
 * The plain comparison behind a month's verification, with no workbook in it.
 *
 * Both counts must match exactly. There is no "close enough": a month that cannot
 * prove it holds the same marks as its source is not written.
 */
export function verifyMonthBuild(
	expectedX: number,
	foundX: number,
	expectedLearners: number,
	foundLearners: number
): MonthVerification {
	if (expectedX === foundX && expectedLearners === foundLearners) return { verified: true };
	return { verified: false, expectedX, foundX, expectedLearners, foundLearners };
}

/**
 * Combine the per-month verdicts into the file's verdict.
 *
 * `Mismatch` wins over `Verified` in every pairing: a file is only saved when
 * *every* month matched, and this is the function that decides that. The mismatching
 * months' own numbers are summed so the caller can name them.
 */
export function combineVerifications(
	verifications: readonly MonthVerification[]
): MonthVerification {
	if (verifications.every(isVerified)) return { verified: true };
	const totals = verifications.reduce(
		(sum, verification) =>
			verification.verified
				? sum
				: {
						...sum,
						expectedX: sum.expectedX + verification.expectedX,
						foundX: sum.foundX + verification.foundX,
						expectedLearners: sum.expectedLearners + verification.expectedLearners,
						foundLearners: sum.foundLearners + verification.foundLearners
					},
		{ expectedX: 0, foundX: 0, expectedLearners: 0, foundLearners: 0 }
	);
	return { verified: false, ...totals };
}

/** The cells a month's absences land in, and the ones no cell was found for. */
type ResolvedMarks = {
	marks: MonthMarkWrite[];
	/** Absences naming a learner the roster does not hold. */
	unmappedStudents: number;
	/** Absences falling on a day the grid has no column for. */
	unmappedDates: number;
};

/**
 * The cells a month's absences land in, given the grid and roster just written.
 *
 * An absence the roster does not know, or whose date the grid has no column for, is
 * **dropped and counted** rather than guessed at. Both cases are the shape of the bug
 * this project exists to fix: a mark in a column the `COUNTIF` does not count, or on
 * a row that is not a learner, is a mark Excel shows as absent and the database
 * cannot reproduce.
 */
export function resolveMarks(
	request: MonthBuildRequest,
	dates: readonly MonthDateMapping[],
	writableColumns: readonly number[]
): ResolvedMarks {
	const rowByStudent = new Map(request.learners.map((row) => [row.studentId, row.rowIndex]));
	const columnByDate = new Map(dates.map((date) => [date.date, date.columnIndex]));
	const marks: MonthMarkWrite[] = [];
	const seen = new Set<string>();
	let unmappedStudents = 0;
	let unmappedDates = 0;

	for (const absence of request.absences) {
		const rowIndex = rowByStudent.get(absence.studentId);
		if (rowIndex === undefined) {
			unmappedStudents += 1;
			continue;
		}
		const columnIndex = columnByDate.get(absence.date);
		if (columnIndex === undefined) {
			unmappedDates += 1;
			continue;
		}
		if (!writableColumns.includes(columnIndex)) continue;
		if (seen.has(`${rowIndex}:${columnIndex}`)) continue;
		seen.add(`${rowIndex}:${columnIndex}`);
		marks.push({ rowIndex, columnIndex, value: SF2_ABSENT_MARK });
	}

	marks.sort(
		(left, right) => left.rowIndex - right.rowIndex || left.columnIndex - right.columnIndex
	);
	return { marks, unmappedStudents, unmappedDates };
}

// ── Building the month worksheets ────────────────────────────────────────────

/** The metadata fields, paired with the header value each one carries. */
const HEADER_FIELDS: [keyof typeof SF2_METADATA_CELLS, keyof MonthHeader][] = [
	['schoolId', 'schoolId'],
	['schoolYear', 'schoolYear'],
	['reportMonth', 'reportMonth'],
	['schoolName', 'schoolName'],
	['gradeLevel', 'gradeLevel'],
	['section', 'section'],
	['adviserSignature', 'adviserName'],
	['adviserPrintedName', 'adviserName'],
	['schoolHeadPrintedName', 'schoolHeadName']
];

/**
 * Write this month's day numbers into the sheet's day row.
 *
 * Every labelled column is written, blank included: a month with no school day in a
 * slot must not inherit the previous occupant's day number.
 */
function dayNumberMarks(
	sheetName: string,
	request: MonthBuildRequest,
	reportMonthNumber: number,
	slots: readonly MonthDaySlot[]
): Sf2CellMark[] {
	const lastDay = daysInMonth(request.reportYear, reportMonthNumber);
	if (request.firstSchoolDay < 1 || request.firstSchoolDay > lastDay) {
		throw new Error(
			`the first attendance day of ${request.reportMonth} is not between 1 and ${lastDay}`
		);
	}
	return dayNumbersForSlots(
		request.reportYear,
		reportMonthNumber,
		request.firstSchoolDay,
		slots
	).map(
		({ column, day }): Sf2CellMark => ({
			sheetName,
			address: cellAddress(SF2_DAY_ROW, column),
			value: day === undefined ? '' : String(day)
		})
	);
}

/**
 * Write the header block, from the legacy workbook's own template row.
 *
 * A metadata cell *below* the roster shifts with the inserted rows; the school block
 * above it does not.
 */
function headerMarks(
	sheetName: string,
	header: MonthHeader,
	rosterBottomRow: number,
	rowShift: number
): Sf2CellMark[] {
	return HEADER_FIELDS.map(([field, value]) => {
		const { row, column } = SF2_METADATA_CELLS[field];
		const target = row > rosterBottomRow ? row + rowShift : row;
		return { sheetName, address: cellAddress(target, column), value: header[value] };
	});
}

/** Copy the roster into the learner rows: the `No.` cell and the name. */
function learnerMarks(sheetName: string, learners: readonly MonthLearnerWrite[]): Sf2CellMark[] {
	const marks: Sf2CellMark[] = [];
	for (const learner of learners) {
		marks.push(
			{
				sheetName,
				address: cellAddress(learner.rowIndex, SF2_ITEM_NUMBER_COLUMN),
				value: String(learner.itemNumber)
			},
			{
				sheetName,
				address: cellAddress(learner.rowIndex, SF2_NAME_COLUMN),
				value: learner.name.trim()
			}
		);
	}
	return marks;
}

/**
 * Read the built month back and count what it actually holds.
 *
 * Counted over the addressable day columns only - the second half of a merged weekday
 * pair is a slave, and counting it would count a mark twice - and over the learner
 * rows only, because the MALE TOTAL row sits between the two blocks and carries a
 * label.
 */
function verifySheet(
	sheet: Worksheet,
	bands: readonly RowBand[],
	learners: readonly MonthLearnerWrite[]
): { foundX: number; foundLearners: number } {
	const columns = writableDayColumns(sheet).map(columnNumber);
	let foundX = 0;
	for (const band of bands) {
		for (let row = band.firstRow; row <= band.lastRow; row += 1) {
			const cells = sheet.getRow(row);
			foundX += countAbsentMarks(columns.map((column) => cellText(cells.getCell(column))));
		}
	}
	const foundLearners = learners.filter(
		(learner) => getCellText(sheet, learner.rowIndex, SF2_NAME_COLUMN).trim() !== ''
	).length;
	return { foundX, foundLearners };
}

/** Empty one month worksheet and write everything into it. */
function populateMonthSheet(
	workbook: Workbook,
	donor: Worksheet,
	sheet: Worksheet,
	reportMonthNumber: number,
	request: MonthBuildRequest
): MonthBuildReport {
	const sheetName = sheet.name;

	// The form first, so the roster rows inserted below shift the pasted MALE/FEMALE
	// TOTAL rows and their formulas with them. Growing a blank sheet and copying
	// afterwards would leave the two in an order that only happens to come out right.
	copyFormSheet(donor, sheet);

	// Grow before clearing: the clear's row numbers are the post-insertion ones. The
	// roster is the same on all twelve sheets, so one shape covers the whole file and a
	// row index means the same learner everywhere. Growth is measured against the
	// sheet's own TOTAL labels, so re-running a build over a sheet a previous build
	// already grew inserts the *difference* rather than a second expansion.
	const femaleStart = Math.max(request.sourceFemaleStartRow, SF2_FRESH_FEMALE_START_ROW);
	const maleCount = request.learners.filter((learner) => learner.rowIndex < femaleStart).length;
	const femaleCount = request.learners.length - maleCount;
	const target = rosterTarget(maleCount, femaleCount, femaleStart);
	const current = totalRowsOnSheet(sheet);
	const insertMale = Math.max(
		0,
		target.maleCapacity - (current.maleTotalRow - SF2_FIRST_LEARNER_ROW)
	);
	const insertFemale = Math.max(
		0,
		target.femaleCapacity - (current.femaleTotalRow - current.maleTotalRow - 1)
	);
	growRosterRows(sheet, insertMale, insertFemale, current.maleTotalRow, current.femaleTotalRow);
	const totalRows = target.totalRows;

	emptyMonthSheet(sheet, totalRows.maleTotalRow, totalRows.femaleTotalRow);
	writeTotalLabels(
		sheet,
		totalRows.maleTotalRow,
		totalRows.femaleTotalRow,
		totalRows.combinedTotalRow
	);
	// The sheet was copied from the donor form, so its hidden flags are the
	// donor's - a roster that shrank since would leave empty rows showing and
	// filled rows hidden. The build owns the roster it just wrote, so it hides
	// what the roster does not claim.
	hideEmptyLearnerRowsOnSheet(
		sheet,
		totalRows.maleTotalRow,
		totalRows.femaleTotalRow,
		new Set(request.learners.map((learner) => learner.rowIndex))
	);

	const slots = weekdaySlots(sheet);
	if (slots.length === 0) {
		throw new Error('the SF2 month sheet has no weekday header, so its days cannot be laid out');
	}
	const dates = monthDateMappings(
		request.templateId,
		request.reportYear,
		reportMonthNumber,
		request.firstSchoolDay,
		slots
	);

	// One pass over every literal the month needs. `textFormat` matches the Rust's
	// `set_sf2_cell(.., force_text)`, so a school id keeps its leading digits.
	const writableColumns = writableDayColumns(sheet).map(columnNumber);
	const { marks, unmappedStudents, unmappedDates } = resolveMarks(request, dates, writableColumns);
	applyMarks(
		workbook,
		[
			...dayNumberMarks(sheetName, request, reportMonthNumber, slots),
			...headerMarks(
				sheetName,
				request.header,
				totalRows.femaleTotalRow,
				headerRowShift(insertMale, insertFemale)
			),
			...learnerMarks(sheetName, request.learners),
			...marks.map(
				(mark): Sf2CellMark => ({
					sheetName,
					address: cellAddress(mark.rowIndex, mark.columnIndex),
					value: mark.value
				})
			)
		],
		{ textFormat: true }
	);

	// The ABSENT / PRESENT and TOTAL formulas that make Excel count what was just
	// written, each with the value Excel caches for it. Without them the form shows the
	// marks but every total is blank, and `emptyMonthSheet` has just cleared the ones the
	// template shipped.
	const grid = readDayGrid(sheet, SF2_FIRST_LEARNER_ROW, totalRows.combinedTotalRow);
	const gridFor = () => grid;
	const days: Sf2DayColumn[] = dates.map((date) => ({ sheetName, column: date.columnLetter }));
	const totals = totalFormulaMarks(days, maleCount, femaleCount, totalRows, gridFor);
	const absentPresent = learnerAbsentPresentFormulaMarks(
		[sheetName],
		request.learners.map((learner) => ({ row: learner.rowIndex })),
		maleCount,
		femaleCount,
		dates.length,
		totalRows,
		gridFor
	);
	writeFormulaMarks(workbook, [...totals, ...absentPresent.formulaMarks]);
	writeMarksForce(workbook, absentPresent.staticMarks);

	const found = verifySheet(sheet, attendanceBandsForWrites(request.learners), request.learners);
	return {
		sheetName,
		dates,
		writtenMarks: marks.length,
		extraRosterRows: insertMale + insertFemale,
		unmappedAbsences: { students: unmappedStudents, dates: unmappedDates },
		verification: verifyMonthBuild(
			request.absences.length,
			found.foundX,
			request.learners.length,
			found.foundLearners
		)
	};
}

/**
 * Build the month worksheets of one workbook, and save it only if all verify.
 *
 * The order is fixed and it is the order the safety depends on:
 *
 * 1. **Create every month's worksheet.**
 * 2. **Make all twelve visible.** There is no hide path anywhere in this module: a
 *    month is selected by reading the database, not by uncovering a tab.
 * 3. **Populate each one**: reset it to the donor form, grow the roster, empty the
 *    roster block, write the day numbers, the header block, the roster and the `X`
 *    marks, then the ABSENT/PRESENT and TOTAL formulas.
 * 4. **Delete** every SF2-form worksheet that is not one of them - but only when the
 *    caller asked for it, and only *after* all twelve months are written, because the
 *    donor the month sheets were copied from is itself one of the sheets being removed.
 * 5. **Save**, or leave the file on disk exactly as it was.
 */
export async function buildSchoolYearWorkbook(
	path: string,
	builds: readonly MonthSheetBuild[],
	summaryCounts?: Sf2SummaryCountsByColumn
): Promise<SchoolYearBuildReport> {
	if (builds.length === 0) {
		throw new Error('a workbook build was asked for with no months in it');
	}
	const workbook = await openWorkbook(path);
	const donor = donorFormSheet(workbook);

	const wanted: PreparedMonthSheet[] = [];
	const reportMonths: number[] = [];
	for (const build of builds) {
		const reportMonthNumber = monthNumber(build.request.reportMonth);
		if (reportMonthNumber === 0) {
			throw new Error(`\`${build.request.reportMonth}\` is not a month this workbook can hold`);
		}
		reportMonths.push(reportMonthNumber);
		wanted.push(prepareMonthSheet(workbook, reportMonthNumber, build.request.reportYear));
	}
	makeMonthSheetsVisible(workbook, wanted);

	const months: MonthBuildReport[] = [];
	for (const [index, build] of builds.entries()) {
		months.push(
			populateMonthSheet(
				workbook,
				donor,
				worksheetByName(workbook, wanted[index].name),
				reportMonths[index],
				build.request
			)
		);
	}

	const removedSheets = builds[0].removeStaleSheets
		? removeNonMonthFormSheets(
				workbook,
				wanted.map((sheet) => sheet.name)
			)
		: [];
	const verification = combineVerifications(months.map((month) => month.verification));

	// A month that came back wrong is not saved. The file on disk is untouched, so the
	// caller can retry the month that failed instead of inheriting a workbook in which
	// September has been rebuilt and October still holds a template's sample roster.
	if (isVerified(verification)) {
		// Inside this pass, not after another parse of the file: the counts are computed
		// from the TOTAL Per Day rows just written, and those rows are already here.
		if (summaryCounts !== undefined) writeSummaryBlock(workbook, builds, summaryCounts);
		await saveWorkbookAtomic(workbook, path);
	}

	return { months, removedSheets, keptHelperSheets: helperSheetNames(workbook), verification };
}

// ── Reading the legacy workbook ──────────────────────────────────────────────

/**
 * One month of a legacy workbook, read without writing to it.
 *
 * The workbook is opened for reading and never saved, so reading the original can
 * never modify it - which is the promise that lets the split keep the file as the
 * fallback authority for any month it cannot prove.
 */
export type LegacyMonthSnapshot = {
	/** The legacy sheet the month was read from. */
	sheetName: string;
	reportMonth: string;
	reportYear: number;
	learners: Sf2LearnerRow[];
	/** Every non-empty cell of the learner rows, with its absolute position. */
	marks: MonthMarkWrite[];
	/** How many `X` marks the month holds. */
	xCount: number;
	/**
	 * The day number the sheet prints in each labelled day column, read out of its own
	 * day row.
	 *
	 * Needed to turn a mark's *column* back into a *date*, which is what makes the "the
	 * workbook holds a mark the database cannot produce" check exact rather than a bare
	 * count comparison. Read from the sheet rather than recomputed from the calendar,
	 * because the number in the cell is the day whoever wrote the sheet meant.
	 */
	dayByColumn: Map<number, number>;
	/** Where the source's female block starts, so the month file grows to match. */
	femaleStartRow: number;
};

/**
 * The `YYYY-MM-DD` a mark in `columnIndex` stands for, or `undefined` when the sheet
 * prints no day there.
 *
 * `undefined` is the honest answer for the second half of a merged weekday pair and
 * for every column past the month's last school day. A mark there cannot be matched
 * against a date, so the comparison ignores it rather than inventing one - and a mark
 * in a sub-cell is a duplicate of its pair's primary anyway, which *is* checked.
 */
export function dateInColumn(
	snapshot: LegacyMonthSnapshot,
	columnIndex: number
): string | undefined {
	const day = snapshot.dayByColumn.get(columnIndex);
	const month = monthNumber(snapshot.reportMonth);
	if (day === undefined || month === 0) return undefined;
	if (day < 1 || day > daysInMonth(snapshot.reportYear, month)) return undefined;
	return isoDate(snapshot.reportYear, month, day);
}

/** One month of a bulk read, successful or not. */
type LegacyMonthRead = {
	reportMonth: string;
	reportYear: number;
	/** Absent when the legacy file has no readable sheet for this month. */
	snapshot?: LegacyMonthSnapshot;
	/** Why this month could not be read, when it could not. */
	error?: string;
};

/**
 * Read every requested month of a legacy twelve-tab workbook.
 *
 * A month is a separate outcome rather than a separate call because a failure has to
 * stay *inside* the month: the legacy file may simply have no sheet for one month, and
 * that must not cost the other eleven their read.
 */
export async function readLegacyMonths(
	path: string,
	months: readonly { reportMonth: string; reportYear: number }[]
): Promise<LegacyMonthRead[]> {
	const workbook = await openWorkbook(path);
	return months.map(({ reportMonth, reportYear }) => {
		const upper = reportMonth.toUpperCase();
		try {
			return {
				reportMonth: upper,
				reportYear,
				snapshot: readLegacyMonth(workbook, upper, reportYear)
			};
		} catch (thrown) {
			return {
				reportMonth: upper,
				reportYear,
				error: thrown instanceof Error ? thrown.message : String(thrown)
			};
		}
	});
}

function readLegacyMonth(
	workbook: Workbook,
	reportMonth: string,
	reportYear: number
): LegacyMonthSnapshot {
	const month = monthNumber(reportMonth);
	// Prefer the sheet whose year is the one being split, so a leftover renamed tab
	// from an earlier school year cannot be read as this month.
	const chosen = sheetEntries(workbook)
		.filter((entry) => isMonthSheetOf(entry.name, month))
		.sort(
			(left, right) =>
				Number(yearFromSheetName(left.name) !== reportYear) -
					Number(yearFromSheetName(right.name) !== reportYear) ||
				left.name.localeCompare(right.name)
		)[0];
	if (!chosen) throw new Error(`the original workbook has no ${reportMonth} sheet to split from`);

	const sheet = worksheetByName(workbook, chosen.name);
	const learners = readLearnerRows(sheet);
	const lastRow = learners.reduce(
		(last, learner) => Math.max(last, learner.row),
		SF2_FIRST_LEARNER_ROW
	);
	const grid = readDayGrid(sheet, SF2_FIRST_LEARNER_ROW, lastRow);

	const marks: MonthMarkWrite[] = [];
	for (const learner of learners) {
		for (const [column, value] of grid.marksByRow.get(learner.row) ?? []) {
			marks.push({ rowIndex: learner.row, columnIndex: columnNumber(column), value });
		}
	}

	const dayByColumn = new Map<number, number>();
	for (const slot of weekdaySlots(sheet)) {
		const day = Number.parseInt(getCellText(sheet, SF2_DAY_ROW, slot.column).trim(), 10);
		if (Number.isInteger(day) && day >= 1 && day <= 31) dayByColumn.set(slot.column, day);
	}

	return {
		sheetName: chosen.name,
		reportMonth,
		reportYear,
		learners,
		marks,
		xCount: countAbsentMarks(marks.map((mark) => mark.value)),
		dayByColumn,
		femaleStartRow: sourceFemaleStartRow(learners)
	};
}
