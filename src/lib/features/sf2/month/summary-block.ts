/**
 * The X counts, the TOTAL Per Day rows and the summary block across `AR` / `AS` / `AT`.
 *
 * This used to be a second full pass over the file: `buildSchoolYearWorkbook` parsed
 * the twelve-month workbook, populated every month and saved it, and then
 * `finishMonthSheets` parsed that same file again to write these numbers and save it
 * again. ExcelJS needs seconds per parse and per serialize of a file this size, and
 * the webview main thread does the work, so those four extra round trips were most of
 * why creating or importing a workbook looked like it would never finish.
 *
 * The block is computed from the TOTAL Per Day rows the build just wrote, and those
 * rows are already in memory - so the only reason it needed its own pass was the file
 * round trip, and there is no longer one. It is written into the build's own open
 * workbook, and the file is saved once.
 */

import ExcelJS from 'exceljs';
import type { Workbook, Worksheet } from 'exceljs';
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
import {
	cellText,
	columnLetter,
	monthNumber,
	numericCellValue,
	type Sf2DayGrid
} from '$lib/features/excel/workbook';
import { dayNumbersForSlots } from '$lib/features/sf2/calendar';
import { monthSheetName, weekdaySlots } from './workbook-sheets';
import type { MonthSheetBuild } from './workbook-builder';

const FEMALE = 'FEMALE';

/**
 * The day grid as Excel will read it: one entry per addressable day column.
 *
 * `readDayGrid` answers every column in the block, and a merge slave answers with
 * its master's value - so a mark written into the left cell of a merged pair is
 * reported twice. Excel's own `COUNTIF(F8:AL8,"X")` counts it once, so a count
 * taken from the unfiltered grid is a number the workbook disagrees with the moment
 * the teacher opens it. Slaves are skipped here for exactly that reason.
 */
function primaryDayGrid(sheet: Worksheet, firstRow: number, lastRow: number): Sf2DayGrid {
	const grid: Sf2DayGrid = { marksByRow: new Map(), numbersByRow: new Map() };
	for (let row = firstRow; row <= lastRow; row += 1) {
		const marks = new Map<string, string>();
		const numbers: number[] = [];
		for (
			let column = SF2_ATTENDANCE_FIRST_COLUMN;
			column <= SF2_ATTENDANCE_LAST_COLUMN;
			column += 1
		) {
			const cell = sheet.getRow(row).getCell(column);
			if (cell.type === ExcelJS.ValueType.Merge) continue;
			const number = numericCellValue(cell);
			if (number !== undefined) {
				numbers.push(number);
				continue;
			}
			const text = cellText(cell);
			if (text !== '') marks.set(columnLetter(column), text);
		}
		if (marks.size > 0) grid.marksByRow.set(row, marks);
		if (numbers.length > 0) grid.numbersByRow.set(row, numbers);
	}
	return grid;
}

/**
 * Write the counts and the summary block for every month the build produced.
 *
 * `counts` supplies rows 55, 67, 69 and 71 per column: a girl who transfers out
 * leaves the boys' figure untouched, so one shared set would silently over-count the
 * combined column.
 *
 * Saves nothing. The caller already holds this workbook open and saves it once.
 */
export function writeSummaryBlock(
	workbook: Workbook,
	builds: readonly MonthSheetBuild[],
	counts: Sf2SummaryCountsByColumn
): void {
	for (const build of builds) {
		const reportMonth = monthNumber(build.request.reportMonth);
		const sheet = workbook.getWorksheet(monthSheetName(reportMonth, build.request.reportYear));
		if (sheet === undefined) continue;

		const maleCount = build.request.learners.filter(
			(learner) => learner.genderBlock !== FEMALE
		).length;
		const femaleCount = build.request.learners.length - maleCount;
		const totals = bundledTemplateTotalRows(maleCount, femaleCount);
		const slots = weekdaySlots(sheet);
		const days: Sf2DayColumn[] = dayNumbersForSlots(
			build.request.reportYear,
			reportMonth,
			build.request.firstSchoolDay,
			slots
		)
			.filter((entry) => entry.day !== undefined)
			.map((entry) => ({ sheetName: sheet.name, column: columnLetter(entry.column) }));

		const grid = primaryDayGrid(sheet, SF2_FIRST_LEARNER_ROW, totals.combinedTotalRow);
		const absentPresent = learnerAbsentPresentFormulaMarks(
			[sheet.name],
			build.request.learners.map((learner) => ({ row: learner.rowIndex })),
			maleCount,
			femaleCount,
			days.length,
			totals,
			() => grid
		);
		writeFormulaMarks(workbook, [
			...totalFormulaMarks(days, maleCount, femaleCount, totals, () => grid),
			...absentPresent.formulaMarks
		]);
		writeMarksForce(workbook, absentPresent.staticMarks);

		// The summary averages the TOTAL Per Day rows just written, so it is computed
		// from a second read rather than the one above.
		const summary = summaryFormulaMarks(
			[sheet.name],
			maleCount,
			femaleCount,
			totals,
			() => primaryDayGrid(sheet, SF2_FIRST_LEARNER_ROW, totals.combinedTotalRow),
			counts
		);
		writeFormulaMarks(workbook, summary.formulaMarks);
		writeMarksForce(workbook, summary.staticMarks);
	}
}
