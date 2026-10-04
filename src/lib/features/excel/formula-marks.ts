/**
 * Building `Sf2CellMark` lists out of a sheet's day grid.
 *
 * Where `formulas.ts` computes one shape and `marks.ts` writes marks to a
 * workbook, this pairs them: it decides *which* cells of the SF2 form need a
 * formula and hands `formulas.ts` the inputs, so every mark leaves with both the
 * formula text and the value Excel caches for it (migration spec D7).
 */

import {
	SF2_FIRST_LEARNER_ROW,
	SF2_SUMMARY_COLUMNS,
	SF2_SUMMARY_ROWS,
	SF2_TOTAL_DAYS_CELL,
	type Sf2SummaryCountsByColumn
} from './constants';
import {
	averageDailyAttendance,
	blockAbsentTotal,
	blockPresentTotal,
	combinedAbsentTotal,
	combinedDailyPresentTotal,
	combinedPresentTotal,
	dailyPresentTotal,
	learnerAbsentTotal,
	learnerPresentTotal,
	percentageOfAttendance,
	percentageOfEnrolment,
	registeredLearners,
	type Sf2Formula
} from './formulas';
import { columnDayMarks, rowDayMarks, rowDayNumbers, type Sf2DayGrid } from './workbook';
import type { Sf2CellMark } from './types';

/** The three TOTAL rows of one sheet, as the formulas address them. */
export type Sf2TotalRows = {
	maleTotalRow: number;
	femaleTotalRow: number;
	combinedTotalRow: number;
};

/** One mapped day column of one sheet: `("SEPT. 2025", "F")`. */
export type Sf2DayColumn = { sheetName: string; column: string };

/** A learner row the roster mapped onto the workbook. */
type Sf2MappedRow = { row: number };

/** The day grid of each sheet the marks will be written to. */
export type Sf2GridSource = (sheetName: string) => Sf2DayGrid;

/** A formula mark: the formula text, and the value Excel will cache for it. */
function mark(sheetName: string, address: string, computed: Sf2Formula): Sf2CellMark {
	return {
		sheetName,
		address,
		value: String(computed.value),
		formula: computed.formula,
		cachedValue: computed.value
	};
}

function staticMark(sheetName: string, address: string, value: number): Sf2CellMark {
	return { sheetName, address, value: String(value), cachedValue: value };
}

/** The absent totals of a row range, zero-filling the slots no learner claims. */
function rangeValues(values: Map<number, number>, firstRow: number, lastRow: number): number[] {
	const slice: number[] = [];
	for (let row = firstRow; row <= lastRow; row += 1) slice.push(values.get(row) ?? 0);
	return slice;
}

/**
 * Formulas for the three TOTAL Per Day rows.
 *
 * Per day column: male present, female present, combined present. Empty template
 * slots never hold an `X`, so they cannot pull the present count down.
 *
 * `days` must hold only addressable columns - the form merges consecutive day
 * columns into pairs, and a mark addressed to the right cell of a pair lands on
 * its master and overwrites it. `writableDayColumns` gives the right list.
 */
export function totalFormulaMarks(
	days: readonly Sf2DayColumn[],
	maleCount: number,
	femaleCount: number,
	rows: Sf2TotalRows,
	gridFor: Sf2GridSource
): Sf2CellMark[] {
	const maleFirst = SF2_FIRST_LEARNER_ROW;
	const maleLast = rows.maleTotalRow - 1;
	const femaleFirst = rows.maleTotalRow + 1;
	const femaleLast = rows.femaleTotalRow - 1;
	const grids = new Map<string, Sf2DayGrid>();
	const marks: Sf2CellMark[] = [];

	for (const day of days) {
		let grid = grids.get(day.sheetName);
		if (!grid) {
			grid = gridFor(day.sheetName);
			grids.set(day.sheetName, grid);
		}
		const male = dailyPresentTotal(
			maleCount,
			day.column,
			maleFirst,
			maleLast,
			columnDayMarks(grid, day.column, maleFirst, maleLast)
		);
		const female = dailyPresentTotal(
			femaleCount,
			day.column,
			femaleFirst,
			femaleLast,
			columnDayMarks(grid, day.column, femaleFirst, femaleLast)
		);
		marks.push(
			mark(day.sheetName, `${day.column}${rows.maleTotalRow}`, male),
			mark(day.sheetName, `${day.column}${rows.femaleTotalRow}`, female),
			mark(
				day.sheetName,
				`${day.column}${rows.combinedTotalRow}`,
				combinedDailyPresentTotal(
					day.column,
					rows.maleTotalRow,
					rows.femaleTotalRow,
					male.value,
					female.value
				)
			)
		);
	}
	return marks;
}

/**
 * Formulas for the ABSENT / PRESENT columns, per learner row and per TOTAL row.
 *
 * The bundled template ships these formulas on most rows but not all - some
 * learner rows and the subtotal rows are missing them - and its cached values are
 * stale, which would make every PRESENT count wrong. So the whole block is
 * regenerated, and `AW5` is corrected to the real mapped day count.
 */
export function learnerAbsentPresentFormulaMarks(
	sheetNames: readonly string[],
	learnerRows: readonly Sf2MappedRow[],
	maleCount: number,
	femaleCount: number,
	dayCount: number,
	rows: Sf2TotalRows,
	gridFor: Sf2GridSource
): { formulaMarks: Sf2CellMark[]; staticMarks: Sf2CellMark[] } {
	const formulaMarks: Sf2CellMark[] = [];
	const staticMarks: Sf2CellMark[] = [];
	const maleLast = rows.maleTotalRow - 1;
	const femaleFirst = rows.maleTotalRow + 1;
	const femaleLast = rows.femaleTotalRow - 1;
	const grids = new Map<string, Sf2DayGrid>();

	for (const sheetName of sheetNames) {
		let grid = grids.get(sheetName);
		if (!grid) {
			grid = gridFor(sheetName);
			grids.set(sheetName, grid);
		}
		staticMarks.push(staticMark(sheetName, SF2_TOTAL_DAYS_CELL, dayCount));

		const absentByRow = new Map<number, number>();
		for (const learner of learnerRows) {
			if (learner.row === 0) continue;
			const absent = learnerAbsentTotal(learner.row, rowDayMarks(grid, learner.row));
			absentByRow.set(learner.row, absent.value);
			formulaMarks.push(
				mark(sheetName, `AM${learner.row}`, absent),
				mark(
					sheetName,
					`AO${learner.row}`,
					learnerPresentTotal(learner.row, dayCount, absent.value)
				)
			);
		}

		const maleAbsent = blockAbsentTotal(
			SF2_FIRST_LEARNER_ROW,
			maleLast,
			rangeValues(absentByRow, SF2_FIRST_LEARNER_ROW, maleLast)
		);
		const femaleAbsent = blockAbsentTotal(
			femaleFirst,
			femaleLast,
			rangeValues(absentByRow, femaleFirst, femaleLast)
		);
		formulaMarks.push(
			mark(sheetName, `AM${rows.maleTotalRow}`, maleAbsent),
			mark(
				sheetName,
				`AO${rows.maleTotalRow}`,
				blockPresentTotal(rows.maleTotalRow, maleCount, dayCount, maleAbsent.value)
			),
			mark(sheetName, `AM${rows.femaleTotalRow}`, femaleAbsent),
			mark(
				sheetName,
				`AO${rows.femaleTotalRow}`,
				blockPresentTotal(rows.femaleTotalRow, femaleCount, dayCount, femaleAbsent.value)
			),
			mark(
				sheetName,
				`AM${rows.combinedTotalRow}`,
				combinedAbsentTotal(
					rows.maleTotalRow,
					rows.femaleTotalRow,
					maleAbsent.value,
					femaleAbsent.value
				)
			),
			mark(
				sheetName,
				`AO${rows.combinedTotalRow}`,
				combinedPresentTotal(
					rows.maleTotalRow,
					rows.femaleTotalRow,
					dayCount * maleCount - maleAbsent.value,
					dayCount * femaleCount - femaleAbsent.value
				)
			)
		);
	}
	return { formulaMarks, staticMarks };
}

/**
 * Formulas for the summary block (rows 53-65) across the three summary columns.
 *
 * Returns the formulas and the three enrolment counts on row 53 they are seeded
 * from. `counts` supplies rows 55, 67, 69 and 71 per column - a girl who
 * transfers out leaves the boys' figure untouched, so one shared set would
 * silently over-count the combined column.
 */
export function summaryFormulaMarks(
	sheetNames: readonly string[],
	maleCount: number,
	femaleCount: number,
	rows: Sf2TotalRows,
	gridFor: Sf2GridSource,
	counts: Sf2SummaryCountsByColumn
): { formulaMarks: Sf2CellMark[]; staticMarks: Sf2CellMark[] } {
	const formulaMarks: Sf2CellMark[] = [];
	const staticMarks: Sf2CellMark[] = [];
	const enrolments = [maleCount, femaleCount, maleCount + femaleCount];
	const totalRows = [rows.maleTotalRow, rows.femaleTotalRow, rows.combinedTotalRow];

	for (const sheetName of sheetNames) {
		const grid = gridFor(sheetName);
		SF2_SUMMARY_COLUMNS.forEach((column, position) => {
			const registered = registeredLearners(column, {
				enrolment: enrolments[position],
				...counts[column]
			});
			const averageDaily = averageDailyAttendance(
				totalRows[position],
				rowDayNumbers(grid, totalRows[position])
			);

			staticMarks.push(
				staticMark(sheetName, `${column}${SF2_SUMMARY_ROWS.enrolment}`, enrolments[position])
			);
			formulaMarks.push(
				mark(sheetName, `${column}${SF2_SUMMARY_ROWS.registeredLearners}`, registered),
				mark(
					sheetName,
					`${column}${SF2_SUMMARY_ROWS.percentageOfEnrolment}`,
					percentageOfEnrolment(column, enrolments[position], registered.value)
				),
				mark(sheetName, `${column}${SF2_SUMMARY_ROWS.averageDailyAttendance}`, averageDaily),
				mark(
					sheetName,
					`${column}${SF2_SUMMARY_ROWS.percentageOfAttendance}`,
					percentageOfAttendance(column, registered.value, averageDaily.value)
				)
			);
		});
	}
	return { formulaMarks, staticMarks };
}
