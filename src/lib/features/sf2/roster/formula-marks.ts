import type { Workbook } from 'exceljs';
import { SF2_FIRST_LEARNER_ROW } from '$lib/features/excel/constants';
import { summaryFormulaMarks, totalFormulaMarks } from '$lib/features/excel/formula-marks';
import { getSheet, readDayGrid } from '$lib/features/excel/workbook';
import type { Sf2DayGrid } from '$lib/features/excel/workbook';
import type { Sf2TotalRows } from '$lib/features/excel/formula-marks';
import type { Sf2CellMark } from '$lib/features/excel/types';
import type { Sf2DateMapping } from '../metadata';
import type { Sf2SummaryCountsByColumn } from '$lib/features/excel/constants';

/**
 * The formulas a roster sync writes after it has re-assigned the learners.
 *
 * A roster change moves every TOTAL Per Day count: `COUNTIF` over a row range that no
 * longer holds the same learners, and an enrolment summary that no longer counts the
 * same class. Without this step the workbook keeps the counts of the roster it used
 * to have, which is the stale-count bug this module exists to close.
 */

/**
 * Late enrolment, dropouts and transfers, per summary column.
 *
 * All zero, because a roster sync knows the enrolment and nothing else: rows 55, 67,
 * 69 and 71 are the teacher's own entries in the form. The *formula text* is
 * identical either way, so a recalculation in Excel replaces the cached value with
 * the real one the moment the teacher fills those rows in (migration spec D7).
 */
export const NO_MOVEMENT: Sf2SummaryCountsByColumn = {
	AR: { lateEnrolment: 0, droppedOut: 0, transferredOut: 0, transferredIn: 0 },
	AS: { lateEnrolment: 0, droppedOut: 0, transferredOut: 0, transferredIn: 0 },
	AT: { lateEnrolment: 0, droppedOut: 0, transferredOut: 0, transferredIn: 0 }
};

/** Marks for a roster sync, split by how each kind has to be written. */
export interface RosterSyncFormulaMarks {
	totalMarks: Sf2CellMark[];
	summaryFormulaMarks: Sf2CellMark[];
	summaryStaticMarks: Sf2CellMark[];
}

/**
 * One grid read per sheet, over the learner rows and the three TOTAL rows.
 *
 * Both mark builders count the `X` marks already in the grid, so the cached value and
 * the formula text come from the same numbers (migration spec D7). The grid is read
 * once and shared, because a school year is twelve sheets and reading it per mark
 * would be the slowest thing in the sync.
 */
function gridSource(workbook: Workbook, rows: Sf2TotalRows): (sheetName: string) => Sf2DayGrid {
	const grids = new Map<string, Sf2DayGrid>();
	return (sheetName) => {
		let grid = grids.get(sheetName);
		if (grid === undefined) {
			grid = readDayGrid(
				getSheet(workbook, sheetName),
				SF2_FIRST_LEARNER_ROW,
				rows.combinedTotalRow
			);
			grids.set(sheetName, grid);
		}
		return grid;
	};
}

/** The sheets a set of date mappings touches, in first-appearance order. */
export function mappedSheetNames(dateMappings: readonly Sf2DateMapping[]): string[] {
	const names: string[] = [];
	for (const mapping of dateMappings) {
		if (!names.includes(mapping.sheetName)) names.push(mapping.sheetName);
	}
	return names;
}

/**
 * The TOTAL Per Day and Enrolment summary marks a roster sync should write.
 *
 * A mapping with no date cannot be addressed, so it is skipped: it is the shape an
 * Excel read returns when a day column was cleared, and writing to it would put a
 * count in a column that holds no date.
 */
export function rosterSyncFormulaMarks(
	workbook: Workbook,
	maleCount: number,
	femaleCount: number,
	rows: Sf2TotalRows,
	dateMappings: readonly Sf2DateMapping[]
): RosterSyncFormulaMarks {
	const usable = dateMappings.filter((mapping) => mapping.date.trim() !== '');
	const gridFor = gridSource(workbook, rows);

	const totalMarks = totalFormulaMarks(
		usable.map((mapping) => ({ sheetName: mapping.sheetName, column: mapping.columnLetter })),
		maleCount,
		femaleCount,
		rows,
		gridFor
	);

	const sheetNames = mappedSheetNames(usable);
	const summary = summaryFormulaMarks(
		sheetNames,
		maleCount,
		femaleCount,
		rows,
		gridFor,
		NO_MOVEMENT
	);

	return {
		totalMarks,
		summaryFormulaMarks: summary.formulaMarks,
		summaryStaticMarks: summary.staticMarks
	};
}

/**
 * Only the Enrolment summary marks, for a caller that has written the TOTAL Per Day
 * and per-learner formulas itself.
 *
 * The summary block counts the whole class rather than one month's grid, so a
 * twelve-month roster sync writes it once per workbook instead of once per month.
 */
export function summaryMarksForRoster(
	workbook: Workbook,
	maleCount: number,
	femaleCount: number,
	rows: Sf2TotalRows,
	sheetNames: readonly string[]
): { formulaMarks: Sf2CellMark[]; staticMarks: Sf2CellMark[] } {
	return summaryFormulaMarks(
		sheetNames,
		maleCount,
		femaleCount,
		rows,
		gridSource(workbook, rows),
		NO_MOVEMENT
	);
}
