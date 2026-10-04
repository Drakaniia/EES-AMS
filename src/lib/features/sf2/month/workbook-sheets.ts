/**
 * The worksheets of the single-file, twelve-sheet SF2 workbook, and the naming
 * rules that decide which of them is a month.
 *
 * Ported from `src-tauri/src/sf2/month/workbook_sheets.rs`. The DepEd School
 * Form 2 is one worksheet per month on one file, and this module owns the two
 * things that must never disagree about that file: the form's own geometry -
 * which row a learner is on, where the weekday header sits, where a MALE TOTAL
 * row lands - because the code that *empties* a worksheet and the code that
 * *fills* one both read it from here; and which worksheet name counts as a month,
 * because that single predicate is what makes "never import the bundled
 * template's sample data" structural rather than a promise.
 *
 * ## Why the naming rule is the safety property
 *
 * The bundled `TEMPLATE_AUTOMATED_SF2.xlsx` ships five month worksheets and a
 * sixth copy of the form called `COMPLETE DAYS`, carrying `X` marks on a roster
 * of names that are not this teacher's students. The old per-month-file design
 * also left `__SF2_HIDDEN_{n}` worksheets behind whenever it hid and renamed a
 * tab.
 *
 * `isSf2MonthlySheetName` (in `$lib/features/excel/workbook`) accepts a name only
 * if it parses to a real month **and** a four-digit year of this century.
 * `JUNE`, `__SF2_HIDDEN_1` and `COMPLETE DAYS` all fail it, so no read path in
 * this project can reach a worksheet that is not a month of the school year.
 *
 * ## What "emptying" a worksheet means
 *
 * A month worksheet is made by **copying the form** and then clearing the roster
 * block of the copy - never by clearing a worksheet the user may have filled in.
 * {@link emptyMonthSheet} keeps the copy's merges and formats and drops only its
 * contents, so the DepEd layout - borders, the `C:E` name merge, the day-column
 * fills - survives while not one mark, name or number of the donor's sample
 * class does. The Rust had to unmerge, clear, then re-paste formats because
 * Excel's `PasteSpecial` could not target a merge's interior; ExcelJS writes the
 * merged model out as-is, so dropping the values is enough.
 */

import type { Workbook, Worksheet } from 'exceljs';
import { clearRange } from '$lib/features/excel/marks';
import {
	SF2_ATTENDANCE_LAST_COLUMN,
	SF2_FIRST_LEARNER_ROW,
	SF2_FORM_TITLE,
	SF2_FRESH_FEMALE_TOTAL_ROW,
	SF2_FRESH_MALE_TOTAL_ROW,
	SF2_NAME_COLUMN,
	SF2_WEEKDAY_ROW
} from '$lib/features/excel/constants';
import {
	cellAddress,
	cellText,
	columnNumber,
	eachPopulatedCell,
	formulaOf,
	formulaResult,
	getSheet,
	isSf2MonthlySheetName,
	materialiseSharedFormulas,
	monthNumber,
	openWorkbook,
	parseAddress,
	writableDayColumns
} from '$lib/features/excel/workbook';
import { countAbsentMarks } from '$lib/features/excel/formulas';
import { readLearnerRows } from '$lib/features/excel/roster';
import type { Sf2LearnerRow } from '$lib/features/excel/types';

/**
 * The prefix of the retired hide/rename cycle's worksheets.
 *
 * The old per-month-file design hid a tab and renamed it to `__SF2_HIDDEN_{n}`
 * while it rebuilt the month. Those leftovers are recognised here so they can be
 * *removed* - and, because they are not month names, so they can never be *read*
 * as one.
 */
export const HIDDEN_SHEET_PREFIX = '__SF2_HIDDEN_';

/** Excel refuses a worksheet name longer than this. */
export const MONTH_SHEET_NAME_MAX = 31;

/**
 * One day column of a month file, resolved against the sheet's own weekday
 * header rather than against a hardcoded calendar.
 */
export type MonthDaySlot = {
	/** 1-based Excel column of the day cell. */
	column: number;
	/** 0-based week, counting from the first week the month touches. */
	weekIndex: number;
	/** 0 = Monday .. 4 = Friday. */
	weekdayIndex: number;
};

/** One worksheet's name, position and visibility. */
export type SheetEntry = {
	/** 1-based index of the worksheet in the workbook, matching Excel. */
	index: number;
	name: string;
	visible: boolean;
	/** Whether the worksheet's `A1` carries the DepEd School Form 2 title. */
	isSf2Form: boolean;
};

/**
 * A month worksheet that exists, ready to be filled.
 *
 * The row fields start at zero: a worksheet that has only just been created has
 * no roster of its own yet, and the build fills them in once the roster's shape
 * is known.
 */
export type PreparedMonthSheet = {
	/** `"{MONTH} {year}"`, the canonical worksheet name. */
	name: string;
	maleTotalRow: number;
	femaleTotalRow: number;
	combinedTotalRow: number;
	/** The last learner row of the sheet, which is the FEMALE TOTAL row. */
	lastRosterRow: number;
};

/** The MALE TOTAL row's label, which is how the female block is found. */
const MALE_TOTAL_LABEL = 'MALE TOTAL';

/** The FEMALE total row's label. */
const FEMALE_TOTAL_LABEL = 'FEMALE TOTAL';

// ── Naming ───────────────────────────────────────────────────────────────────

/**
 * The canonical worksheet name for a month: `"{MONTH} {year}"`.
 *
 * The full month name, upper case, because that is the spelling every other
 * spelling is canonicalised to. Both must produce the same string, or a write
 * addressed by a stored `sheet_name` would not find its worksheet.
 */
export function monthSheetName(month: number, year: number): string {
	const name = monthNumberToFullName(month);
	return `${name} ${year}`;
}

function monthNumberToFullName(month: number): string {
	const full = [
		'',
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
	][month];
	if (!full) throw new Error(`${month} is not a month this workbook can hold`);
	return full;
}

/**
 * Whether a worksheet name is the given month, in any year.
 *
 * The year is not checked here, because a caller that already knows which school
 * year it is reading compares the month and then prefers the year itself
 * afterwards - {@link readLegacyMonths} reads a leftover tab from an earlier
 * school year only as a last resort.
 */
export function isMonthSheetOf(sheetName: string, month: number): boolean {
	return monthNumber(sheetName) === month;
}

/**
 * Whether a worksheet name is a leftover of the retired hide/rename cycle.
 *
 * The name came from a loop, not from a user, so case and padding are ignored.
 */
export function isHiddenSheetName(sheetName: string): boolean {
	return sheetName.trim().toUpperCase().startsWith(HIDDEN_SHEET_PREFIX);
}

/**
 * A weekday header cell's text, as a Monday-to-Friday index.
 *
 * `TH` is Thursday and `T` is Tuesday, so the two-letter spellings are matched
 * before the one-letter ones. Anything that is not a weekday - `ABSENT`,
 * `PRESENT`, the empty cell at the right-hand edge of the grid - is `undefined`,
 * because reading one as a weekday would add a 26th day cell and shift every mark
 * after it.
 */
export function parseWeekdayLabel(label: string): number | undefined {
	switch (label.trim().toUpperCase()) {
		case 'M':
		case 'MON':
		case 'MONDAY':
			return 0;
		case 'T':
		case 'TUE':
		case 'TUES':
		case 'TUESDAY':
			return 1;
		case 'W':
		case 'WED':
		case 'WEDNESDAY':
			return 2;
		case 'TH':
		case 'THU':
		case 'THUR':
		case 'THURS':
		case 'THURSDAY':
			return 3;
		case 'F':
		case 'FRI':
		case 'FRIDAY':
			return 4;
		default:
			return undefined;
	}
}

// ── Finding worksheets ───────────────────────────────────────────────────────

/**
 * Whether a worksheet's `A1` carries the DepEd School Form 2 title.
 *
 * Matched as a substring so a school's own variant - the same title with a
 * different suffix - is still recognised as a form worksheet. Visibility is
 * deliberately not part of the test: the retire path has to recognise a form the
 * hide/rename cycle left hidden, or `__SF2_HIDDEN_1` would survive every split.
 * (`isSf2FormSheet` in `$lib/features/excel/analysis` is the visible-only
 * variant, and is what the write paths use.)
 */
export function carriesFormTitle(sheet: Worksheet): boolean {
	return cellText(sheet.getRow(1).getCell(1)).trimStart().includes(SF2_FORM_TITLE);
}

/** Every worksheet in the workbook, in tab order, with its visibility. */
export function sheetEntries(workbook: Workbook): SheetEntry[] {
	return workbook.worksheets.map((sheet, position) => ({
		index: position + 1,
		name: sheet.name,
		visible: sheet.state === 'visible',
		isSf2Form: carriesFormTitle(sheet)
	}));
}

/**
 * One worksheet, by name.
 *
 * Fails rather than falling back to "the first month-shaped tab", because a write
 * that lands on the wrong worksheet is a teacher's marks on a month the school
 * never opened.
 */
export function worksheetByName(workbook: Workbook, name: string): Worksheet {
	return getSheet(workbook, name);
}

/** A month worksheet whose roster rows are not laid out yet. */
function blankPreparedMonthSheet(name: string): PreparedMonthSheet {
	return {
		name,
		maleTotalRow: 0,
		femaleTotalRow: 0,
		combinedTotalRow: 0,
		lastRosterRow: 0
	};
}

/**
 * Create the month worksheet if it is not there yet, and return it.
 *
 * Reusing an existing worksheet of the right name is deliberate: a rebuild of a
 * school year must be able to run against a workbook that already holds eleven of
 * the twelve months, and a build that failed on the tenth month would otherwise
 * have to be thrown away whole.
 */
export function prepareMonthSheet(
	workbook: Workbook,
	month: number,
	year: number
): PreparedMonthSheet {
	const name = monthSheetName(month, year);
	if (name.length > MONTH_SHEET_NAME_MAX) {
		throw new Error(
			`\`${name}\` is longer than the ${MONTH_SHEET_NAME_MAX} characters Excel allows a worksheet name to be`
		);
	}
	if (workbook.getWorksheet(name)) return blankPreparedMonthSheet(name);
	workbook.addWorksheet(name);
	return blankPreparedMonthSheet(name);
}

/**
 * The one worksheet every month sheet is copied from.
 *
 * Chosen by *form*, not by name: a previous build may have renamed it
 * `__SF2_HIDDEN_1`, and a month sheet the previous merge wrote is also a form
 * sheet but is a month this build is about to rewrite. So the first worksheet
 * whose `A1` carries the form's own title wins, and its position in the file is
 * not consulted.
 */
export function donorFormSheet(workbook: Workbook): Worksheet {
	for (const entry of sheetEntries(workbook)) {
		if (entry.isSf2Form) return getSheet(workbook, entry.name);
	}
	throw new Error(
		'the workbook has no School Form 2 worksheet, so the month worksheets cannot be built from it'
	);
}

/**
 * The day columns of a worksheet, from its weekday header.
 *
 * Read off row 7 rather than assumed, so a school's form is measured rather than
 * presumed - and so a worksheet with no weekday header at all returns empty
 * instead of 25 invented columns.
 *
 * Restricted to the columns that are also *writable* (see
 * {@link writableDayColumns}). The two row sets are not the same on the DepEd
 * form: the weekday header is not merged exactly the way the day-number row is,
 * so a column can carry a weekday label and still be the second half of a merged
 * day cell. Writing there lands on the pair's master, which would put one day's
 * `X` on top of the previous day's - so such a column is not a slot at all.
 */
export function weekdaySlots(sheet: Worksheet): MonthDaySlot[] {
	const writable = new Set(writableDayColumns(sheet).map(columnNumber));
	const slots: MonthDaySlot[] = [];
	for (const column of [...writable].sort((left, right) => left - right)) {
		const weekdayIndex = parseWeekdayLabel(cellText(sheet.getRow(SF2_WEEKDAY_ROW).getCell(column)));
		if (weekdayIndex === undefined) continue;
		// The form is five weeks of Monday..Friday, so every fifth labelled column
		// starts a new week. Deriving the week from the count rather than from a
		// hard-coded column list means a form that grew or lost a day column still
		// gets its weeks numbered in order.
		slots.push({ column, weekdayIndex, weekIndex: Math.floor(slots.length / 5) });
	}
	return slots;
}

// ── Shaping a month worksheet ────────────────────────────────────────────────

/**
 * Overwrite a worksheet with the donor form's cells, formats, merges and geometry.
 *
 * Excel's `UsedRange.Copy` pastes values and formats but neither column widths
 * nor row heights, which left a month sheet built by copying alone with the
 * donor's default grid - every day column one character wide and the DepEd table
 * unreadable - so the geometry is copied alongside the cells here.
 *
 * A row the donor has no height for is left alone rather than failed on: a
 * default height is already the right answer.
 *
 * Shared formulas are materialised *first*, because copying a clone's shared
 * reference onto another sheet starts a second shared group whose master is not
 * on that sheet, and the serialiser then refuses to write the file at all.
 */
export function copyFormSheet(donor: Worksheet, sheet: Worksheet): void {
	materialiseSharedFormulas(donor);

	for (let column = 1; column <= donor.columnCount; column += 1) {
		sheet.getColumn(column).width = donor.getColumn(column).width;
	}
	for (let row = 1; row <= donor.rowCount; row += 1) {
		const source = donor.getRow(row);
		if (source.height !== undefined) sheet.getRow(row).height = source.height;
	}

	eachPopulatedCell(donor, (cell, row) => {
		const target = sheet.getRow(row).getCell(Number(cell.col));
		target.style = cell.style;
		const formula = formulaOf(cell);
		target.value = formula === undefined ? cell.value : { formula, result: formulaResult(cell) };
	});

	// Merges last: the values above went onto the top-left cells, which is the only
	// cell of each merged region that can hold one. Every merge is unmerged first, one
	// by one - `unMergeCells()` with no argument only clears the ones covering `A1`
	// (ExcelJS reads the empty range as `A1:A1`), so a month rebuilt onto its own
	// last build kept all 681 and the re-merge threw. A sheet that is its own donor is
	// left alone: unmerging it would empty the very list being copied.
	if (donor === sheet) return;
	for (const range of sheet.model.merges) sheet.unMergeCells(range);
	for (const range of donor.model.merges) sheet.mergeCells(range);
}

/**
 * A merged range's A1 pair with its rows moved down by `shift` rows.
 *
 * ExcelJS stores a merge as a range string (`"A64:X67"`), so shifting one is
 * re-addressing its two ends - the columns are untouched by a row insertion.
 */
function shiftRange(range: string, shift: number): string {
	const [from, to] = range.split(':');
	const start = parseAddress(from);
	const end = parseAddress(to ?? from);
	const moved = (address: { row: number; column: number }) =>
		cellAddress(address.row + shift, address.column);
	return shift === 0 ? range : `${moved(start)}:${moved(end)}`;
}

/**
 * Grow one worksheet's roster to hold `extraMale` more males and `extraFemale`
 * more females.
 *
 * Rows are **inserted**, not appended, so the blocks that follow the roster - the
 * MALE TOTAL row, the female block, the signature rows - move down intact.
 * Appending instead would put the twenty-second male on top of the MALE TOTAL
 * row. `maleTotalRow` / `femaleTotalRow` are the *current* positions; pass
 * nothing for a fresh template.
 *
 * The merges come off first and go back on afterwards, because ExcelJS's
 * `spliceRows` cannot be trusted with a merged worksheet: it assigns the rows by
 * `rDst.values = rSrc.values`, and a merge slave's value *is* its master's, so
 * the master text lands in every cell of the merge's rectangle; and it re-points
 * the slaves with `cell.merge(...)` without touching the sheet's merge index, so
 * `model.merges` keeps reporting the ranges while the serialiser never writes
 * them. Both losses are silent - the file opens, and the roster block below the
 * MALE TOTAL row is a plain grid with the guidelines smeared down four rows.
 * Unmerged, only literal values move, and the ranges are re-applied below on the
 * rows the insertion pushed them to.
 *
 * A merge that straddles the insertion row keeps its top row and grows by the
 * insert count. That is what a total block spanning a boundary wants anyway:
 * its content belongs to the block below.
 *
 * Shared formulas are materialised first, because moving a shared formula's
 * master without its clones leaves the workbook unserialisable.
 */
export function growRosterRows(
	sheet: Worksheet,
	extraMale: number,
	extraFemale: number,
	maleTotalRow = SF2_FRESH_MALE_TOTAL_ROW,
	femaleTotalRow = SF2_FRESH_FEMALE_TOTAL_ROW
): void {
	if (extraMale <= 0 && extraFemale <= 0) return;
	materialiseSharedFormulas(sheet);

	const merges = [...sheet.model.merges];
	for (const range of merges) sheet.unMergeCells(range);

	// Males first: the female divider then sits `extraMale` rows lower, and the
	// female insert does not move the male block a second time.
	for (let index = 0; index < extraMale; index += 1) sheet.spliceRows(maleTotalRow, 0, []);
	for (let index = 0; index < extraFemale; index += 1)
		sheet.spliceRows(femaleTotalRow + extraMale, 0, []);

	for (const range of merges) {
		const shift =
			parseAddress(range.split(':')[0]).row >= femaleTotalRow
				? extraMale + extraFemale
				: parseAddress(range.split(':')[0]).row >= maleTotalRow
					? extraMale
					: 0;
		sheet.mergeCells(shiftRange(range, shift));
	}
}

/**
 * Where the MALE and FEMALE TOTAL rows currently sit on a worksheet.
 *
 * Read off the labels rather than assumed, so growing a worksheet that a previous
 * build already grew inserts the right number of rows instead of stacking a
 * second expansion on top of the first. A worksheet whose roster has not been
 * written yet answers with the fresh template's own positions.
 *
 * `FEMALE` is tested first because it contains `MALE`.
 */
export function totalRowsOnSheet(sheet: Worksheet): {
	maleTotalRow: number;
	femaleTotalRow: number;
} {
	const found = { maleTotalRow: 0, femaleTotalRow: 0 };
	for (let row = SF2_FIRST_LEARNER_ROW; row <= sheet.rowCount; row += 1) {
		const label = cellText(sheet.getRow(row).getCell(SF2_NAME_COLUMN)).toUpperCase();
		if (!label.includes('TOTAL')) continue;
		if (label.includes('FEMALE')) found.femaleTotalRow = row;
		else if (label.includes('MALE')) found.maleTotalRow = row;
	}
	return {
		maleTotalRow: found.maleTotalRow || SF2_FRESH_MALE_TOTAL_ROW,
		femaleTotalRow: found.femaleTotalRow || SF2_FRESH_FEMALE_TOTAL_ROW
	};
}

/**
 * Empty the roster block of a worksheet copied from the donor, keeping its merges
 * and its formats.
 *
 * This is the step that makes the bundled template's sample class impossible to
 * carry over: the sample names, the sample item numbers, the sample `X` marks and
 * the sample `SUM` formulas all go, while the borders, the alignment and the
 * day-column fills are still the DepEd layout.
 */
export function emptyMonthSheet(
	sheet: Worksheet,
	maleTotalRow: number,
	femaleTotalRow: number
): void {
	clearRange(
		sheet,
		SF2_FIRST_LEARNER_ROW,
		Math.max(femaleTotalRow, maleTotalRow),
		1,
		SF2_ATTENDANCE_LAST_COLUMN
	);
}

/** Relabel the three TOTAL rows, which {@link emptyMonthSheet} cleared with the rest. */
export function writeTotalLabels(
	sheet: Worksheet,
	maleTotalRow: number,
	femaleTotalRow: number,
	combinedTotalRow: number
): void {
	for (const [row, label] of [
		[maleTotalRow, MALE_TOTAL_LABEL],
		[femaleTotalRow, FEMALE_TOTAL_LABEL],
		[combinedTotalRow, 'COMBINED TOTAL']
	] as const) {
		const cell = sheet.getRow(row).getCell(SF2_NAME_COLUMN);
		cell.value = label;
		cell.numFmt = '@';
	}
}

/**
 * Make every month worksheet visible.
 *
 * There is no hide path anywhere in this module: a month is selected by reading
 * the database, not by uncovering a tab, so a month the user cannot click to
 * would be a month that looks like it is missing.
 */
export function makeMonthSheetsVisible(
	workbook: Workbook,
	sheets: readonly PreparedMonthSheet[]
): void {
	for (const prepared of sheets) getSheet(workbook, prepared.name).state = 'visible';
}

/**
 * Remove the SF2 form worksheets that are not in `keep`, and report their names.
 *
 * Only worksheets that carry the form's own title are considered, so a school's
 * own working sheet is never touched. Must be called only after every month sheet
 * exists, or the last form sheet would leave the workbook with no visible sheet.
 */
export function removeNonMonthFormSheets(workbook: Workbook, keep: readonly string[]): string[] {
	const doomed = sheetEntries(workbook).filter(
		(entry) => entry.isSf2Form && !keep.includes(entry.name)
	);
	for (const entry of doomed) workbook.removeWorksheet(getSheet(workbook, entry.name).id);
	return doomed.map((entry) => entry.name);
}

/** The non-monthly worksheets still in the workbook, for the build report. */
export function helperSheetNames(workbook: Workbook): string[] {
	return sheetEntries(workbook)
		.filter((entry) => !entry.isSf2Form)
		.map((entry) => entry.name);
}

// ── Reading a worksheet back ─────────────────────────────────────────────────

/** Every worksheet in the file at `path`, with whether it is visible. */
export async function readSheetNames(path: string): Promise<{ name: string; visible: boolean }[]> {
	return sheetEntries(await openWorkbook(path)).map((entry) => ({
		name: entry.name,
		visible: entry.visible
	}));
}

/**
 * The roster on one worksheet, with each learner's own row and gender block.
 *
 * This is what a build needs - a learner is a *row*, and the twelve worksheets
 * only work because the same name is on the same row on all of them - so it is
 * the shape the merge job reads a class's roster in.
 */
export async function learnersOnSheet(path: string, sheetName: string): Promise<Sf2LearnerRow[]> {
	return readLearnerRows(getSheet(await openWorkbook(path), sheetName));
}

/**
 * The learner names on one worksheet, in row order.
 *
 * Used to prove that all twelve worksheets carry the **same** roster on the
 * **same** rows, which is what makes one `sf2_month_student_mappings` row set
 * valid for all twelve months.
 */
export async function learnerNamesOnSheet(path: string, sheetName: string): Promise<string[]> {
	return (await learnersOnSheet(path, sheetName)).map((learner) => learner.name.trim());
}

/**
 * How many `X` marks the named worksheet's learner rows hold.
 *
 * Counted cell by cell over the learner rows and the addressable day columns,
 * rather than by `COUNTIF` over the whole sheet, because what has to be proven is
 * that the marks are on *this* worksheet and in the *learner* rows: a count over
 * the whole sheet would be satisfied by a mark in a total row, and one over the
 * full grid would be satisfied by a mark left in a column the month does not use.
 */
export async function countAbsentMarksOnSheet(path: string, sheetName: string): Promise<number> {
	const sheet = getSheet(await openWorkbook(path), sheetName);
	const columns = writableDayColumns(sheet).map(columnNumber);
	let total = 0;
	for (const learner of readLearnerRows(sheet)) {
		const row = sheet.getRow(learner.row);
		total += countAbsentMarks(columns.map((column) => cellText(row.getCell(column))));
	}
	return total;
}

/** Re-exported so a reader of the month modules has the one true predicate. */
export { isSf2MonthlySheetName };
