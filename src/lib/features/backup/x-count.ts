/**
 * Counting the `"X"` marks a workbook holds, without modifying it.
 *
 * A port of `x_count.rs`. Its one job is to be the evidence in the restore
 * preview: a backup whose workbooks hold more X marks than its database holds
 * absences is pairing a database with its own future, and the app will re-import
 * the difference the next time SF2 is opened. The teacher has to be told that
 * before they click restore, not after.
 *
 * The Rust version let Excel do the counting with one `COUNTIF` per sheet over
 * the learner block. There is no Excel to ask any more (spec D2), so this reads
 * the day grid through `$lib/features/excel` — the same `readDayGrid` the marks
 * writer derives its totals from, over the same learner rows, so the count
 * covers exactly the cells the app treats as attendance marks.
 */

import type { Worksheet } from 'exceljs';
import { readLearnerRows } from '$lib/features/excel/roster';
import { openWorkbook, readDayGrid } from '$lib/features/excel/workbook';

/** The mark the SF2 workbook uses for an absence (`sf2/logic.rs:9`). */
export const ABSENT_MARK = 'X';

/**
 * The number of absences the workbook records.
 *
 * 0 means the workbook has no learner rows, so there is nothing that could hold a
 * mark. A workbook that *cannot be read* — locked, truncated, not a workbook —
 * rejects instead, because reporting 0 there would read as "this file holds no
 * absences" and quietly disable the restore guard. `collectWorkbooks` is the
 * layer that turns a rejection into a recorded 0.
 */
export async function countXmarks(path: string): Promise<number> {
	let total = 0;
	for (const sheet of (await openWorkbook(path)).worksheets) {
		if (sheet.state !== 'visible') continue;
		total += countSheetMarks(sheet);
	}
	return total;
}

function countSheetMarks(sheet: Worksheet): number {
	const rows = readLearnerRows(sheet).map((learner) => learner.row);
	if (rows.length === 0) return 0;

	const grid = readDayGrid(sheet, Math.min(...rows), Math.max(...rows));
	let total = 0;
	for (const marks of grid.marksByRow.values()) {
		for (const mark of marks.values()) {
			if (mark.trim().toUpperCase() === ABSENT_MARK) total += 1;
		}
	}
	return total;
}
