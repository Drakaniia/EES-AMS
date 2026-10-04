/**
 * Workbook introspection - replaces `analyze_workbook`.
 *
 * The Rust version drove Excel to read sheet visibility, `UsedRange`, merges and
 * a handful of metadata cells. All of that is already in the `.xlsx` we just
 * parsed, so the answer is a read of the model rather than a round-trip.
 */

import type { Workbook, Worksheet } from 'exceljs';
import { getFileSystem } from '$lib/platform/fs';
import { SF2_FORM_TITLE } from './constants';
import { cellText, formulaOf, openWorkbook, sheetHasProtection } from './workbook';
import type { Sf2SampleCell, Sf2SheetAnalysis, Sf2WorkbookAnalysis } from './types';

/** How many cells one sheet's analysis carries. */
export const SAMPLE_CELL_LIMIT = 12;

/**
 * The first cells of a sheet worth sampling, formula cells first.
 *
 * Formula cells come first: a count that is wrong is almost always a formula
 * pointing at the wrong range, and the plain text of the form tells the teacher
 * nothing. A sheet with no formulas falls back to its first populated cells.
 */
function sampleCells(sheet: Worksheet): Sf2SampleCell[] {
	const withFormula: Sf2SampleCell[] = [];
	const plain: Sf2SampleCell[] = [];
	sheet.eachRow({ includeEmpty: false }, (row) => {
		row.eachCell({ includeEmpty: false }, (cell) => {
			if (withFormula.length >= SAMPLE_CELL_LIMIT) return;
			const text = cellText(cell);
			if (text === '') return;
			const sample: Sf2SampleCell = {
				address: cell.address,
				// ExcelJS types `Cell.row`/`Cell.col` as strings; they are numbers.
				row: Number(cell.row),
				column: Number(cell.col),
				value: text
			};
			const formula = formulaOf(cell);
			if (formula === undefined) {
				if (plain.length < SAMPLE_CELL_LIMIT) plain.push(sample);
				return;
			}
			withFormula.push({ ...sample, formula });
		});
	});
	return withFormula.length > 0 ? withFormula : plain;
}

/** Sheet inventory for one worksheet. */
function analyzeSheet(sheet: Worksheet, index: number): Sf2SheetAnalysis {
	return {
		name: sheet.name,
		index,
		rowCount: sheet.rowCount,
		columnCount: sheet.columnCount,
		mergedRanges: [...sheet.model.merges],
		sampleCells: sampleCells(sheet),
		hasProtection: sheetHasProtection(sheet)
	};
}

/**
 * Whether a worksheet is an SF2 form.
 *
 * The form title in `A1` is the test, on a visible sheet only. That is also why
 * `COMPLETE DAYS` counts as a form: it is a full copy of one.
 */
export function isSf2FormSheet(sheet: Worksheet): boolean {
	if (sheet.state !== 'visible') return false;
	const title = cellText(sheet.getRow(1).getCell(1)).toUpperCase();
	return title.includes(SF2_FORM_TITLE.toUpperCase());
}

/** Whether a workbook holds the worksheets an SF2 month needs. */
export function looksLikeSf2(workbook: Workbook): boolean {
	return workbook.worksheets.some(isSf2FormSheet);
}

/** Every worksheet of an already-open workbook. */
export function analyzeWorkbook(workbook: Workbook): Sf2WorkbookAnalysis {
	return {
		exists: true,
		sheets: workbook.worksheets.map((sheet, position) => analyzeSheet(sheet, position + 1)),
		looksLikeSf2: looksLikeSf2(workbook)
	};
}

/**
 * Analyse the workbook at `path`, reporting a missing file rather than throwing.
 *
 * A workbook that is not there yet is a normal state - one is about to be
 * created from the template - so callers get `exists: false` instead of an
 * error they would only turn back into the same answer.
 */
export async function analyzeWorkbookFile(path: string): Promise<Sf2WorkbookAnalysis> {
	const missing: Sf2WorkbookAnalysis = { exists: false, sheets: [], looksLikeSf2: false };
	if (!(await getFileSystem().exists(path))) return missing;
	return analyzeWorkbook(await openWorkbook(path));
}
