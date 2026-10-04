/**
 * The one read-only pass over a workbook that the diagnostic needs: what worksheets
 * exist, what day grid each one carries, which cells hold `X`, and who each learner
 * row is — the port of `src-tauri/src/sf2/diagnose/workbook_probe.rs`.
 *
 * ## Read-only, and structurally so
 *
 * `openWorkbook` parses the `.xlsx` into an in-memory model and this module holds no
 * writer: nothing here calls `setCellText`, `saveWorkbookAtomic` or any `worksheet`
 * mutation, so there is no branch that *can* write.
 *
 * ## What did not survive the port
 *
 * The Rust probe flattened each band through a `TEXTJOIN` formula evaluated by
 * `Application.Evaluate`, because COM hands a range read back as a `SAFEARRAY` this
 * layer could not marshal. It also had to validate the token count of every read, to
 * detect an Excel error variant arriving as an empty answer - a worksheet that reads
 * as "no marks" because Excel refused to flatten it. `TEXTJOIN`, `quoteSheetName`,
 * `bandAddress` and the whole two-strategy band reader have no reason to exist once
 * ExcelJS hands over a real grid, and the token-count validation they existed to
 * justify goes with them: a grid read here is the file's grid.
 *
 * The separator constant that came with it is likewise gone. `|` was safe in the
 * attendance block and `\u{1}` was needed for the `NAME` column because the DepEd
 * form's own subtotal rows read `<=== MALE | TOTAL Per Day ===>`; with cells read
 * individually there is nothing to separate.
 *
 * ## Merge pairs are the trap here
 *
 * The template merges day columns in pairs, and ExcelJS answers for a merge *slave*
 * with its *master's* text — `cellText(G6)` is the day number printed in `F6`. Read
 * naively that yields two columns for one day, so both the day grid and the `X` scan
 * below walk merge **masters** only. That is also the set `writableDayColumns()`
 * hands the writer, so the diagnose read and the write read cannot disagree about
 * which columns hold a day. `cellText` is used rather than `cell.text` because
 * `cell.text` throws on a merge cell whose master holds no value.
 */

import type { Worksheet } from 'exceljs';
import { SF2_DAY_ROW, SF2_METADATA_CELLS, SF2_NAME_COLUMN } from '$lib/features/excel/constants';
import { SF2_ABSENT_MARK, isLearnerName } from '$lib/features/sf2/logic';
import { sf2MonthNumber } from '$lib/features/sf2/calendar';
import {
	cellText,
	columnNumber,
	openWorkbook,
	writableDayColumns
} from '$lib/features/excel/workbook';
import type { RawMark } from './compare';
import type { RosterRow } from './model';

/**
 * Last row of the block read.
 *
 * The roster ends well before this (row 48) and the form's adviser/signature block
 * starts at 76, whose cells sit right of the day grid, so nothing an `X` could
 * legitimately be in is left out.
 */
const LAST_BLOCK_ROW = 80;

/** A worksheet, as the workbook has it, before any month has been assigned. */
export interface RawSheet {
	sheetName: string;
	visible: boolean;
	/** The month the sheet's **name** says it is, when the name says one. */
	monthFromName?: number;
	/** The year the sheet's name says, when it says one. */
	yearFromName?: number;
	/**
	 * The sheet's own `report_month` header cell (`AA3`), verbatim.
	 *
	 * Reported, never trusted. `configureSf2Calendar` rewrites this cell on the one
	 * sheet it makes visible and leaves the rest holding whatever the bundled template
	 * shipped, so on a file that has been through that cycle every hidden sheet can
	 * claim to be the same month. Trusting it would read four sample-data worksheets
	 * as September and count them twice.
	 */
	headerMonthLabel: string;
	/** `{ column, day }` for every day column that carries a day. */
	dayNumbers: { column: number; day: number }[];
	/** Every `X` cell in the block, in row-then-column order. */
	marks: RawMark[];
	/** The worksheet's own learner names, by row. */
	rosterNames: RosterRow[];
}

/**
 * How many `X` cells the worksheet holds, over every row and column of the block.
 * The roster-free count: what is on the sheet, regardless of whether the database can
 * place any of it.
 */
export function markCount(sheet: RawSheet): number {
	return sheet.marks.length;
}

/** The name of the learner in `rowIndex`, when this sheet names one. */
export function nameAt(sheet: RawSheet, rowIndex: number): string | undefined {
	return sheet.rosterNames.find((row) => row.rowIndex === rowIndex)?.workbookName;
}

/** A workbook, as the diagnostic found it. */
export interface RawWorkbook {
	path: string;
	sheets: RawSheet[];
}

/**
 * The first sheet that names `month`, by name then by tab order.
 *
 * Name first, so a workbook that keeps all twelve months visible resolves each month
 * to its own sheet rather than to whichever comes first.
 */
export function sheetForMonth(workbook: RawWorkbook, month: number): RawSheet | undefined {
	return workbook.sheets.find((sheet) => sheet.monthFromName === month);
}

/**
 * Read every worksheet of `path` into a raw grid, read-only.
 *
 * Hidden worksheets are included. `analyzeWorkbook` skips any sheet that is not
 * visible, and on a file the pre-split calendar cycle has been through that is eleven
 * of the twelve months - so a probe that skipped them would report every hidden month
 * as having no sheet, which is the one answer that must never be invented.
 */
export async function probeWorkbook(path: string): Promise<RawWorkbook> {
	const workbook = await openWorkbook(path);
	return { path, sheets: workbook.worksheets.map(probeSheet) };
}

function probeSheet(sheet: Worksheet): RawSheet {
	const dayColumns = addressableDayColumns(sheet);
	return {
		sheetName: sheet.name,
		visible: sheet.state === 'visible',
		monthFromName: sf2MonthNumber(sheet.name),
		yearFromName: yearFromSheetName(sheet.name),
		headerMonthLabel: headerMonthLabel(sheet),
		dayNumbers: dayNumbers(sheet, dayColumns),
		marks: marksIn(sheet, dayColumns),
		rosterNames: rosterNames(sheet)
	};
}

function headerMonthLabel(sheet: Worksheet): string {
	const { row, column } = SF2_METADATA_CELLS.reportMonth;
	return cellText(sheet.getRow(row).getCell(column)).trim();
}

/**
 * The day-grid columns a mark can actually be addressed in, as numbers: every column
 * `F`..`AL` that is not the right half of a merged pair.
 *
 * {@link writableDayColumns} is the one existing answer to that question and is what
 * the month writer uses, so reusing it is what keeps the diagnose read and the write
 * read from disagreeing about which columns hold a day. Reading the slave as well
 * would give every day two columns, and the second would address the same cell.
 */
function addressableDayColumns(sheet: Worksheet): number[] {
	return writableDayColumns(sheet).map(columnNumber);
}

/**
 * The worksheet's own day-number row, as `{ column, day }` pairs.
 *
 * Blank, non-numeric and out-of-range cells are not day columns and are skipped - the
 * same acceptance `sf2WeekdaySlots` and the calendar writers apply, so the diagnose
 * read cannot disagree with them about which columns hold days.
 */
function dayNumbers(
	sheet: Worksheet,
	columns: readonly number[]
): { column: number; day: number }[] {
	const found: { column: number; day: number }[] = [];
	for (const column of columns) {
		const text = cellText(sheet.getRow(SF2_DAY_ROW).getCell(column)).trim();
		if (!/^\d+$/.test(text)) continue;
		const day = Number(text);
		if (day < 1 || day > 31) continue;
		found.push({ column, day });
	}
	return found;
}

/** Every `X` in the block, in row-then-column order. */
function marksIn(sheet: Worksheet, columns: readonly number[]): RawMark[] {
	const marks: RawMark[] = [];
	for (let rowIndex = 1; rowIndex <= LAST_BLOCK_ROW; rowIndex += 1) {
		const row = sheet.getRow(rowIndex);
		for (const column of columns) {
			if (cellText(row.getCell(column)).trim().toUpperCase() !== SF2_ABSENT_MARK) continue;
			marks.push({ rowIndex, columnIndex: column });
		}
	}
	return marks;
}

/** The worksheet's own learner names, by row. */
function rosterNames(sheet: Worksheet): RosterRow[] {
	const rows: RosterRow[] = [];
	for (let rowIndex = 1; rowIndex <= LAST_BLOCK_ROW; rowIndex += 1) {
		const name = cellText(sheet.getRow(rowIndex).getCell(SF2_NAME_COLUMN)).trim();
		if (!isLearnerName(name)) continue;
		rows.push({ studentId: '', workbookName: name, rowIndex });
	}
	return rows;
}

/**
 * A year a sheet's name actually carries; `undefined` when it carries none.
 *
 * The first four-digit run that starts with `20`, so `__SF2_HIDDEN_1` and
 * `COMPLETE DAYS` resolve to nothing rather than to a year from some other number.
 */
export function yearFromSheetName(name: string): number | undefined {
	const year = name.split(/[^0-9]/).find((part) => part.length === 4 && part.startsWith('20'));
	return year === undefined ? undefined : Number(year);
}
