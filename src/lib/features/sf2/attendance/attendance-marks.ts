/**
 * Which cells of an SF2 attendance grid a sync is allowed to write — the port of
 * `src-tauri/src/sf2/attendance/attendance_marks.rs`.
 *
 * ## The rule this file exists to hold
 *
 * A sync writes two lists: the `X` marks the database proves, and the blanks that
 * withdraw marks the database no longer holds. The blanks are the dangerous half,
 * and they are computed here as a **set difference** rather than as "empty the
 * grid":
 *
 * ```text
 * to_clear = { cells in this month's scope that hold an X }
 *          - { cells the database says should hold an X }
 * ```
 *
 * So a cell holding an `X` the database still proves is never touched, a cell
 * outside the mapped scope is never touched, and an already-blank cell is never
 * written to at all. The result is structurally a subset of the workbook's own
 * `X` marks, which is what makes an over-broad clear inexpressible rather than
 * merely guarded.
 */

import type { Worksheet } from 'exceljs';
import { resolvedSheetName } from '$lib/features/sf2/month/templates';
import type { Sf2MonthDateMappingRecord } from '$lib/features/sf2/month/templates';
import type { Sf2MonthStudentMapping } from '$lib/features/sf2/month/students';
import { cellText } from '$lib/features/excel/workbook';
import type { Sf2CellMark } from '$lib/features/excel/types';
import { SF2_ABSENT_MARK } from '$lib/features/sf2/logic';

/**
 * One cell of the attendance block, addressed the way Excel addresses it.
 *
 * Both halves of a sync speak this vocabulary — what the database holds, what the
 * workbook holds, what may be cleared — so a comparison can never mix two shapes
 * of data.
 */
export type Sf2GridCell = {
	sheetName: string;
	columnLetter: string;
	rowIndex: number;
};

/** The A1 address, e.g. `AL47`. */
export function gridCellAddress(cell: Sf2GridCell): string {
	return `${cell.columnLetter}${cell.rowIndex}`;
}

/** The stable identity of a cell: its worksheet and its address. */
export function gridCellKey(cell: Sf2GridCell): string {
	return `${cell.sheetName}!${gridCellAddress(cell)}`;
}

/**
 * Recover a cell from a generated mark, or `undefined` for anything that does not
 * end in a row number — which is never a cell of the attendance block.
 */
export function gridCellFromMark(mark: Sf2CellMark): Sf2GridCell | undefined {
	const columnLength = mark.address.search(/[^A-Za-z]/);
	if (columnLength <= 0) return undefined;
	const rowIndex = Number(mark.address.slice(columnLength));
	if (!Number.isInteger(rowIndex)) return undefined;
	return {
		sheetName: mark.sheetName,
		columnLetter: mark.address.slice(0, columnLength),
		rowIndex
	};
}

/**
 * The learner rows the roster mapped, sorted and de-duplicated.
 *
 * Row `0` is the "not linked to a workbook row" placeholder: it has no cell in any
 * sheet and must never be scanned or written.
 */
export function mappedAttendanceRows(rows: Iterable<number>): number[] {
	return [...new Set([...rows].filter((row) => row > 0))].sort((left, right) => left - right);
}

/**
 * Every cell a sync is responsible for: the mapped learner rows × the mapped day
 * columns of the report month.
 *
 * This is the ceiling on what any sync may blank. A month with no mappings yields
 * an empty scope, so a degenerate month produces an empty clear set rather than a
 * wiped grid.
 */
export function attendanceScopeCells(
	studentMappings: readonly Sf2MonthStudentMapping[],
	dateMappings: readonly Sf2MonthDateMappingRecord[]
): Sf2GridCell[] {
	const rows = mappedAttendanceRows(studentMappings.map((mapping) => mapping.rowIndex));
	const cells: Sf2GridCell[] = [];
	for (const date of dateMappings) {
		const sheetName = resolvedSheetName(date);
		for (const rowIndex of rows) {
			cells.push({ sheetName, columnLetter: date.columnLetter, rowIndex });
		}
	}
	return cells;
}

/**
 * The marks this sync may write as blanks, narrowed to the cells that really hold
 * an `X` right now.
 *
 * See the module docs for the subtraction. The result is sorted by worksheet then
 * address so two runs over the same state produce byte-identical writes.
 */
export function differentialClearMarks(
	scope: readonly Sf2GridCell[],
	dbXCells: readonly Sf2GridCell[],
	workbookXCells: readonly Sf2GridCell[]
): Sf2CellMark[] {
	const scopeKeys = new Set(scope.map(gridCellKey));
	const dbXKeys = new Set(dbXCells.map(gridCellKey));
	const cleared = new Set<string>();

	for (const cell of workbookXCells) {
		const key = gridCellKey(cell);
		if (scopeKeys.has(key) && !dbXKeys.has(key)) cleared.add(key);
	}

	return [...cleared].sort().map((key) => {
		const separator = key.lastIndexOf('!');
		return {
			sheetName: key.slice(0, separator),
			address: key.slice(separator + 1),
			value: ''
		};
	});
}

/**
 * The `X` cells a worksheet holds, over the given scope.
 *
 * Read off the sheet rather than from a count: a count cannot say *which* cells,
 * and the whole point of the differential clear is that the answer is per cell.
 */
export function workbookXCells(sheet: Worksheet, scope: readonly Sf2GridCell[]): Sf2GridCell[] {
	return scope.filter(
		(cell) =>
			cell.sheetName === sheet.name &&
			cellText(sheet.getCell(gridCellAddress(cell)))
				.trim()
				.toUpperCase() === SF2_ABSENT_MARK
	);
}
