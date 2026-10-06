/**
 * Opening, saving and addressing an SF2 workbook.
 *
 * There is no Excel process and no COM object graph: a workbook is a plain
 * `ExcelJS.Workbook` value and a save is a temp-file write plus a rename
 * (migration spec D15).
 */

import ExcelJS from 'exceljs';
import type { Cell, Row, Workbook, Worksheet, WorksheetProtection } from 'exceljs';
import { getFileSystem } from '$lib/platform/fs';
import { SF2_ATTENDANCE_FIRST_COLUMN, SF2_ATTENDANCE_LAST_COLUMN, SF2_DAY_ROW } from './constants';

type ExcelBuffer = Parameters<Workbook['xlsx']['load']>[0];

/**
 * ExcelJS types `load`/`writeBuffer` against Node's `Buffer`, which the webview
 * does not have. At runtime both accept any binary view, so the single
 * unavoidable bridge lives here instead of being an `any` at each call site.
 */
function asExcelBuffer(bytes: Uint8Array): ExcelBuffer {
	return bytes as unknown as ExcelBuffer;
}

/** ExcelJS carries sheet protection at runtime but omits it from `index.d.ts`. */
type ProtectedWorksheet = Worksheet & { sheetProtection?: WorksheetProtection | null };

/** Whether a worksheet is protected against editing. */
export function sheetHasProtection(sheet: Worksheet): boolean {
	return Boolean((sheet as ProtectedWorksheet).sheetProtection);
}

/** Convert a 1-based column number to its Excel letters (1 -> A, 27 -> AA). */
export function columnLetter(column: number): string {
	let letter = '';
	let remaining = column;
	while (remaining > 0) {
		const modulo = (remaining - 1) % 26;
		letter = String.fromCharCode(65 + modulo) + letter;
		remaining = Math.floor((remaining - modulo) / 26);
	}
	return letter;
}

/** Convert Excel column letters to their 1-based number (`AL` -> 38). */
export function columnNumber(letters: string): number {
	let value = 0;
	for (const character of letters.toUpperCase()) {
		value = value * 26 + (character.charCodeAt(0) - 64);
	}
	return value;
}

/** The A1 address for a 1-based row and column. */
export function cellAddress(row: number, column: number): string {
	return `${columnLetter(column)}${row}`;
}

/** Parse an A1 address into its 1-based row and column. */
function parseAddress(address: string): { row: number; column: number } {
	const match = /^([A-Za-z]+)(\d+)$/.exec(address);
	if (!match) throw new Error(`Not an A1 address: ${address}`);
	return { column: columnNumber(match[1]), row: Number(match[2]) };
}

/**
 * Read a workbook from `path` through the bound file system.
 *
 * Nothing is written back, so a read cannot modify the teacher's file.
 */
export async function openWorkbook(path: string): Promise<Workbook> {
	const bytes = await getFileSystem().readFile(path);
	const workbook = new ExcelJS.Workbook();
	await workbook.xlsx.load(asExcelBuffer(bytes));
	return workbook;
}

/**
 * Write `workbook` to `path` without ever truncating the file already there.
 *
 * The bytes go to a sibling temp file and are renamed over the target, so a
 * failure part-way through leaves the previous good workbook intact.
 *
 * ExcelJS invents `fitToWidth`/`fitToHeight` (both `1`) when it parses a sheet
 * whose `<pageSetup>` has neither, and writes them straight back - so every
 * save would shrink the form onto one page even though the template never
 * asked for fit-to-page and nothing in the app sets it. They are cleared here,
 * at the one choke point every workbook save goes through.
 */
export async function saveWorkbookAtomic(workbook: Workbook, path: string): Promise<void> {
	for (const sheet of workbook.worksheets) {
		sheet.pageSetup.fitToWidth = undefined;
		sheet.pageSetup.fitToHeight = undefined;
	}
	const buffer = await workbook.xlsx.writeBuffer();
	await getFileSystem().writeFileAtomic(path, new Uint8Array(buffer));
}

/** Serialize a workbook to bytes, for callers that write somewhere else. */
export async function workbookToBytes(workbook: Workbook): Promise<Uint8Array> {
	const buffer = await workbook.xlsx.writeBuffer();
	return new Uint8Array(buffer);
}

/**
 * Make one worksheet the workbook's active tab, so the file reopens on it.
 *
 * Under the one-file-twelve-months model the landing tab is otherwise whatever
 * the last write happened to leave in front. A worksheet the file does not have
 * is not an error - the caller is asking for a nicer landing tab, and a workbook
 * missing one is still a workbook the teacher can click through - so it is
 * reported rather than thrown.
 */
export function activateSheet(workbook: Workbook, sheetName: string): boolean {
	const index = workbook.worksheets.findIndex((sheet) => sheet.name === sheetName);
	if (index < 0) return false;
	const [view] = workbook.views;
	if (view) view.activeTab = index;
	else workbook.views = [{ activeTab: index }] as Workbook['views'];
	return true;
}

// ── Sheet-name rules ─────────────────────────────────────────────────────────

const MONTH_ABBREVIATIONS = [
	['JAN', 1],
	['FEB', 2],
	['MAR', 3],
	['APR', 4],
	['MAY', 5],
	['JUN', 6],
	['JUL', 7],
	['AUG', 8],
	['SEP', 9],
	['OCT', 10],
	['NOV', 11],
	['DEC', 12]
] as const;

/**
 * Which month (1-12) a sheet name represents, or 0 if it names none.
 *
 * `SEPT. 2025`, `JUNE2025` and `September 2026` all resolve; `COMPLETE DAYS`
 * and `__SF2_HIDDEN_1` do not.
 */
export function monthNumber(name: string): number {
	const upper = name.toUpperCase();
	for (const [abbreviation, month] of MONTH_ABBREVIATIONS) {
		if (upper.includes(abbreviation)) return month;
	}
	return 0;
}

/** The full uppercase English name of a month, or `''` when out of range. */
export function monthName(month: number): string {
	return (
		[
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
		][month] ?? ''
	);
}

/** A 4-digit year of this century found in a sheet name, or 0. */
export function yearFromSheetName(name: string): number {
	for (const part of name.split(/\D+/)) {
		if (part.length === 4 && part.startsWith('20')) return Number(part);
	}
	return 0;
}

/** Whether a worksheet name is a month of a school year. */
export function isSf2MonthlySheetName(name: string): boolean {
	return monthNumber(name) > 0 && yearFromSheetName(name) > 0;
}

/** Every visible monthly sheet - the only ones a write may touch. */
export function sf2MonthlySheets(workbook: Workbook): Worksheet[] {
	return workbook.worksheets.filter(
		(sheet) => sheet.state === 'visible' && isSf2MonthlySheetName(sheet.name)
	);
}

/**
 * The day columns of the attendance block that can actually be written.
 *
 * The form merges consecutive day columns into pairs (`F8:G8`, `R29:S29`, …) so
 * that a wide weekend or non-school day occupies as much width as a school one.
 * Only the left column of each pair holds the day number and only the left column
 * can be addressed - writing the right one lands on its master and overwrites it.
 */
export function writableDayColumns(sheet: Worksheet): string[] {
	const columns: string[] = [];
	for (
		let column = SF2_ATTENDANCE_FIRST_COLUMN;
		column <= SF2_ATTENDANCE_LAST_COLUMN;
		column += 1
	) {
		if (sheet.getRow(SF2_DAY_ROW).getCell(column).type !== ExcelJS.ValueType.Merge) {
			columns.push(columnLetter(column));
		}
	}
	return columns;
}

// ── Cell access ──────────────────────────────────────────────────────────────

/** The worksheet of that name, or a thrown error naming what is missing. */
export function getSheet(workbook: Workbook, sheetName: string): Worksheet {
	const sheet = workbook.getWorksheet(sheetName);
	if (!sheet) throw new Error(`SF2 workbook has no worksheet named \`${sheetName}\``);
	return sheet;
}

/**
 * The cell a write lands on.
 *
 * Every SF2 field and total sits in a merged cell, and only the top-left cell of
 * a merge holds the value - writing anywhere else is silently dropped by Excel.
 */
export function writableCell(cell: Cell): Cell {
	return cell.type === ExcelJS.ValueType.Merge ? cell.master : cell;
}

/** Whether a cell carries a formula. */
export function hasFormula(cell: Cell): boolean {
	return cell.type === ExcelJS.ValueType.Formula;
}

/** The formula text of a cell, whether it stands alone or is shared. */
export function formulaOf(cell: Cell): string | undefined {
	const { formula, sharedFormula } = cell.model;
	return formula ?? sharedFormula;
}

/**
 * The value Excel caches for a formula cell.
 *
 * Read from `cell.model` rather than `cell.value`: ExcelJS's value getter copies
 * the model through a truthiness check, so a cached result of `0` - a day with
 * nobody absent - comes back as `undefined` and reads as "not calculated".
 */
export function formulaResult(cell: Cell): number | string | boolean | undefined {
	if (!hasFormula(cell)) return undefined;
	const { result } = cell.model;
	if (typeof result === 'number' || typeof result === 'string' || typeof result === 'boolean') {
		return result;
	}
	return undefined;
}

/** The number a cell holds, whether it is a literal or a formula's cached result. */
export function numericCellValue(cell: Cell): number | undefined {
	if (hasFormula(cell)) {
		const result = formulaResult(cell);
		return typeof result === 'number' ? result : undefined;
	}
	return typeof cell.value === 'number' ? cell.value : undefined;
}

/**
 * The cell's displayed text, matching what Excel's `Range.Text` returned.
 *
 * Blank for a cell holding nothing, which is what the caller checks to decide
 * whether a row or a TOTAL slot is empty. Two ExcelJS quirks are handled here,
 * both of which a read cannot be allowed to inherit:
 *
 * - a merge slave delegates to its master, and throws rather than returning blank
 *   once that master has been cleared - which clearing a TOTAL row does;
 * - a formula whose cached result is `0` renders as blank text, while Excel shows
 *   `0`, and a day with nobody absent must not read as an empty cell.
 */
export function cellText(cell: Cell): string {
	if (cell.type === ExcelJS.ValueType.Merge) {
		return cell.master === cell ? '' : cellText(cell.master);
	}
	if (cell.text !== '') return cell.text;
	const result = formulaResult(cell);
	return typeof result === 'number' ? String(result) : '';
}

/** Displayed text of one cell, addressed by row and column. */
export function getCellText(sheet: Worksheet, row: number, column: number): string {
	return cellText(sheet.getRow(row).getCell(column));
}

/** Displayed text of one cell, addressed by A1. */
export function getCellTextAt(sheet: Worksheet, address: string): string {
	return cellText(sheet.getCell(address));
}

/** Write a value to one cell, addressed by row and column. */
export function setCellText(sheet: Worksheet, row: number, column: number, value: string): void {
	const cell = writableCell(sheet.getRow(row).getCell(column));
	cell.value = value === '' ? null : value;
}

/**
 * Iterate the cells that hold something, row by row.
 *
 * Skips empty cells so a 656-row month workbook costs one pass over the rows
 * that matter rather than one lookup per coordinate.
 */
export function eachPopulatedCell(
	sheet: Worksheet,
	visit: (cell: Cell, row: number) => void
): void {
	sheet.eachRow({ includeEmpty: false }, (row: Row) => {
		row.eachCell({ includeEmpty: false }, (cell: Cell) => visit(cell, row.number));
	});
}

/**
 * Rewrite every shared formula as a formula of its own.
 *
 * ExcelJS stores a repeated formula once and points each clone at its master's
 * address. Inserting a row moves the master without rewriting the clones, and the
 * serialiser then refuses to write the file at all ("Shared Formula master must
 * exist above and or left of clone"). Reading each clone's already-translated
 * formula first turns the group back into independent formulas, so a row insert
 * cannot leave the workbook unwritable.
 */
export function materialiseSharedFormulas(sheet: Worksheet): void {
	eachPopulatedCell(sheet, (cell) => {
		if (!hasFormula(cell) || cell.model.formula !== undefined) return;
		const formula = cell.formula;
		if (formula === undefined) return;
		cell.value = { formula, result: formulaResult(cell) };
	});
}

/**
 * Insert `count` blank rows at `at`, keeping merged cells intact.
 *
 * ExcelJS's own `spliceRows` moves merged slaves by copying their master's
 * text into every cell (`rDst.values = rSrc.values`) and re-points them with
 * `cell.merge()`, which never reaches the sheet's merge index — the file then
 * opens with smeared duplicate text and silently dropped merges. So the merges
 * come off first and go back on shifted afterwards, while no cell holds a
 * merge value.
 *
 * A merge below the insertion shifts by `count`; one above it stays; one
 * straddling it keeps its top row and extends by `count`. Inserted rows copy
 * the style and height of `templateRow` (default: the row above the
 * insertion), because an unformatted learner row is a row the form stops
 * drawing.
 *
 * The merges go back on with `mergeCellsWithoutStyle` and each cell's own style
 * is put back by hand. ExcelJS's `mergeCells` would give every slave cell the
 * master's style (`Cell.merge` does `this.style = master.style`), so inserting
 * one row at the bottom of the form would restyle every merge above it - the day
 * grid's inner vertical rules would all become the master's border. See
 * {@link copyFormSheet} for the same trap on the copy path.
 */
export function spliceRowsPreservingMerges(
	sheet: Worksheet,
	at: number,
	count: number,
	templateRow?: number
): void {
	if (count <= 0) return;
	const merges = [...sheet.model.merges];

	// Read every merged cell's style before the merges come off: `unmerge()` resets
	// a slave's style to its row and column defaults, so the styles above have to be
	// remembered across the round trip rather than re-derived from the master.
	const mergedStyles = new Map<string, Cell['style']>();
	for (const range of merges) {
		const [from, to] = range.split(':');
		const start = parseAddress(from);
		const end = parseAddress(to ?? from);
		for (let row = start.row; row <= end.row; row += 1) {
			for (let column = start.column; column <= end.column; column += 1) {
				mergedStyles.set(`${row}:${column}`, sheet.getRow(row).getCell(column).style);
			}
		}
	}

	for (const range of merges) sheet.unMergeCells(range);
	sheet.spliceRows(at, 0, ...Array.from({ length: count }, () => []));
	for (const range of merges) {
		const [from, to] = range.split(':');
		const start = parseAddress(from);
		const end = parseAddress(to ?? from);
		const moved = (row: number): number => (row >= at ? row + count : row);
		const newEnd = start.row < at && end.row >= at ? end.row + count : moved(end.row);
		sheet.mergeCellsWithoutStyle(
			`${cellAddress(moved(start.row), start.column)}:${cellAddress(newEnd, end.column)}`
		);
		// The rows at and below the insertion moved down by `count`, so each cell's
		// remembered style goes back on at its new address. The rows that were just
		// inserted are not in this map and are styled from `templateRow` below.
		for (let row = start.row; row <= end.row; row += 1) {
			for (let column = start.column; column <= end.column; column += 1) {
				const style = mergedStyles.get(`${row}:${column}`);
				if (style !== undefined) sheet.getRow(moved(row)).getCell(column).style = style;
			}
		}
	}
	const template = sheet.getRow(templateRow ?? at - 1);
	for (let row = at; row < at + count; row += 1) {
		const inserted = sheet.getRow(row);
		inserted.height = template.height;
		template.eachCell({ includeEmpty: true }, (cell, column) => {
			inserted.getCell(column).style = cell.style;
		});
	}
}

/** The `F`–`AL` day grid of one sheet, read once. */
export type Sf2DayGrid = {
	/** The mark in each day column of one row, keyed by row then column letter. */
	marksByRow: Map<number, Map<string, string>>;
	/** The numeric cells of one row across the day block - what `AVERAGE` reads. */
	numbersByRow: Map<number, number[]>;
};

/**
 * Read the day grid once, so the formula text and the cached value it needs are
 * derived from the same numbers.
 *
 * `numbersByRow` holds only the numeric cells, because `AVERAGE` over the TOTAL
 * rows skips blanks rather than counting them as zero.
 */
export function readDayGrid(sheet: Worksheet, firstRow: number, lastRow: number): Sf2DayGrid {
	const marksByRow = new Map<number, Map<string, string>>();
	const numbersByRow = new Map<number, number[]>();
	for (let row = firstRow; row <= lastRow; row += 1) {
		const marks = new Map<string, string>();
		const numbers: number[] = [];
		for (
			let column = SF2_ATTENDANCE_FIRST_COLUMN;
			column <= SF2_ATTENDANCE_LAST_COLUMN;
			column += 1
		) {
			const cell = sheet.getRow(row).getCell(column);
			// Day columns are merged in pairs, so the right half reports its
			// master's value. Counting both would double every mark written on a
			// merged pair, which is how a learner absent once came to cache AM8=2.
			if (cell.type === ExcelJS.ValueType.Merge) continue;
			// A TOTAL Per Day cell holds a formula whose cached result is the count,
			// so the number has to come from the formula, not only from literals.
			const number = numericCellValue(cell);
			if (number !== undefined) {
				numbers.push(number);
				continue;
			}
			const text = cellText(cell);
			if (text !== '') marks.set(columnLetter(column), text);
		}
		if (marks.size > 0) marksByRow.set(row, marks);
		if (numbers.length > 0) numbersByRow.set(row, numbers);
	}
	return { marksByRow, numbersByRow };
}

/** The X marks of one row across the day block. */
export function rowDayMarks(grid: Sf2DayGrid, row: number): (string | undefined)[] {
	const marks = grid.marksByRow.get(row);
	if (!marks) return [];
	return [...marks.values()];
}

/** The X marks in one day column, over the learner rows of a block. */
export function columnDayMarks(
	grid: Sf2DayGrid,
	column: string,
	firstRow: number,
	lastRow: number
): (string | undefined)[] {
	const marks: (string | undefined)[] = [];
	for (let row = firstRow; row <= lastRow; row += 1)
		marks.push(grid.marksByRow.get(row)?.get(column));
	return marks;
}

/** The numeric day cells of one row, which is what `AVERAGE` sees. */
export function rowDayNumbers(grid: Sf2DayGrid, row: number): number[] {
	return grid.numbersByRow.get(row) ?? [];
}
