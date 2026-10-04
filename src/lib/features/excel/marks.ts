/**
 * Writing attendance marks and the metadata block into a workbook.
 *
 * Replaces `write_marks`, `write_formulas`, `write_marks_force`,
 * `write_metadata` and `clear_total_rows`. Each formula mark is written as a
 * formula *and* its cached value, because ExcelJS cannot evaluate and the app
 * compares counts without opening Excel (migration spec D7).
 *
 * The `Sf2CellMark` lists themselves are built in `formula-marks.ts`.
 */

import ExcelJS from 'exceljs';
import type { Workbook, Worksheet } from 'exceljs';
import {
	SF2_ATTENDANCE_FIRST_COLUMN,
	SF2_ATTENDANCE_LAST_COLUMN,
	SF2_METADATA_CELLS
} from './constants';
import { cellAddress, getSheet, hasFormula, writableCell } from './workbook';
import { isSf2FormSheet } from './analysis';
import type { Sf2TotalRows } from './formula-marks';
import type { Sf2CellMark, Sf2WorkbookMetadata } from './types';

export type { Sf2TotalRows };

/** How `applyMarks` should treat a cell that already holds a formula. */
export type ApplyMarksOptions = {
	/**
	 * Whether a literal mark may replace an existing formula.
	 *
	 * The Rust `set_sf2_mark` refused and `set_sf2_mark_force` did not; the force
	 * variant existed because the TOTAL Per Day cells ship with `SUM` formulas
	 * that have to be replaced with computed numbers.
	 */
	allowFormulaOverwrite?: boolean;
	/** Force the cell to Excel's text format (`NumberFormat = "@"`). */
	textFormat?: boolean;
};

function cellValueFor(mark: Sf2CellMark): ExcelJS.CellValue {
	if (mark.formula !== undefined) {
		return { formula: mark.formula, result: mark.cachedValue ?? 0 };
	}
	if (mark.value === '') return null;
	// `set_sf2_mark_force` stored a parseable count as a number so the formulas
	// that add it up could calculate; anything else stays text.
	const numeric = Number(mark.value);
	return Number.isInteger(numeric) && mark.value.trim() !== '' ? numeric : mark.value;
}

/**
 * Apply marks to a workbook, writing each one as a literal, or as a formula plus
 * its cached value.
 *
 * Sheets are looked up once per name rather than per mark, because a month of
 * attendance is tens of thousands of marks into a handful of sheets.
 */
export function applyMarks(
	workbook: Workbook,
	marks: readonly Sf2CellMark[],
	options: ApplyMarksOptions = {}
): void {
	const sheets = new Map<string, Worksheet>();
	// Two marks that resolve to the same merged cell are two answers for one
	// question, and the second would silently replace the first. That is the shape
	// of bug this file exists to prevent, so it is refused rather than resolved.
	const written = new Map<string, string>();
	for (const entry of marks) {
		let sheet = sheets.get(entry.sheetName);
		if (!sheet) {
			sheet = getSheet(workbook, entry.sheetName);
			sheets.set(entry.sheetName, sheet);
		}
		const key = `${sheet.name}!${writableCell(sheet.getCell(entry.address)).address}`;
		const content = entry.formula ?? entry.value;
		const previous = written.get(key);
		if (previous !== undefined && previous !== content) {
			throw new Error(
				`Marks disagree about ${key}: \`${previous}\` and \`${content}\` both land on it, ` +
					'because the form merges that cell with its neighbour'
			);
		}
		written.set(key, content);
		writeMark(sheet, entry, options);
	}
}

function writeMark(sheet: Worksheet, entry: Sf2CellMark, options: ApplyMarksOptions): void {
	const cell = writableCell(sheet.getCell(entry.address));
	if (entry.formula === undefined && hasFormula(cell) && options.allowFormulaOverwrite !== true) {
		throw new Error(`Refusing to overwrite formula cell ${sheet.name}!${cell.address}`);
	}
	if (options.textFormat === true) cell.numFmt = '@';
	cell.value = cellValueFor(entry);
	if (entry.style === 'bold') cell.font = { ...cell.font, bold: true };
	else if (entry.style === 'header') cell.font = { ...cell.font, bold: true, size: 11 };
}

/** Write literal marks, refusing to clobber a formula cell. */
export function writeMarks(workbook: Workbook, marks: readonly Sf2CellMark[]): void {
	applyMarks(workbook, marks);
}

/** Write literal marks over formula cells - the TOTAL Per Day counts. */
export function writeMarksForce(workbook: Workbook, marks: readonly Sf2CellMark[]): void {
	applyMarks(workbook, marks, { allowFormulaOverwrite: true });
}

/** Write formula marks, each carrying the value Excel caches for it. */
export function writeFormulaMarks(workbook: Workbook, marks: readonly Sf2CellMark[]): void {
	applyMarks(workbook, marks, { allowFormulaOverwrite: true });
}

// ── The metadata block ───────────────────────────────────────────────────────

/** Every metadata cell, in the order the form reads them. */
function metadataEntries(
	metadata: Sf2WorkbookMetadata
): [keyof typeof SF2_METADATA_CELLS, string][] {
	return [
		['schoolId', metadata.schoolId],
		['schoolYear', metadata.schoolYear],
		['reportMonth', metadata.reportMonth],
		['schoolName', metadata.schoolName],
		['gradeLevel', metadata.gradeLevel],
		['section', metadata.section],
		['adviserSignature', metadata.adviserName],
		['adviserPrintedName', metadata.adviserName],
		['schoolHeadPrintedName', metadata.schoolHeadName]
	];
}

/**
 * Write the school / class / adviser block onto every SF2 form sheet.
 *
 * Only sheets carrying the form title are touched: a school's own working sheet
 * in the same workbook is none of our business. Returns how many sheets changed.
 */
export function writeMetadata(workbook: Workbook, metadata: Sf2WorkbookMetadata): number {
	const marks: Sf2CellMark[] = [];
	for (const sheet of workbook.worksheets) {
		if (!isSf2FormSheet(sheet)) continue;
		for (const [field, value] of metadataEntries(metadata)) {
			const { row, column } = SF2_METADATA_CELLS[field];
			marks.push({ sheetName: sheet.name, address: cellAddress(row, column), value });
		}
	}
	applyMarks(workbook, marks, { textFormat: true });
	return new Set(marks.map((entry) => entry.sheetName)).size;
}

// ── Clearing ─────────────────────────────────────────────────────────────────

/**
 * Empty a rectangular range of cells.
 *
 * One pass over the coordinates rather than per-cell clearing, because the
 * attendance grid is the largest area the app touches.
 */
export function clearRange(
	sheet: Worksheet,
	startRow: number,
	endRow: number,
	startColumn: number,
	endColumn: number
): void {
	if (startRow > endRow) return;
	for (let row = startRow; row <= endRow; row += 1) {
		for (let column = startColumn; column <= endColumn; column += 1) {
			sheet.getRow(row).getCell(column).value = null;
		}
	}
}

/**
 * Empty the MALE, FEMALE and Combined TOTAL rows across every weekday column.
 *
 * These rows ship with `SUM` and `+` formulas that must be gone before the
 * generated marks are written, or a stale formula would still be sitting in a
 * cell the app is about to fill.
 */
export function clearTotalRows(workbook: Workbook, sheetName: string, rows: Sf2TotalRows): void {
	const sheet = getSheet(workbook, sheetName);
	for (const row of [rows.maleTotalRow, rows.femaleTotalRow, rows.combinedTotalRow]) {
		clearRange(sheet, row, row, SF2_ATTENDANCE_FIRST_COLUMN, SF2_ATTENDANCE_LAST_COLUMN);
	}
}
