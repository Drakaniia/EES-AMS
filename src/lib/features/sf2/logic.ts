/**
 * The small SF2 rules that the writer and the reader must agree on — the port of
 * `src-tauri/src/sf2/logic.rs`.
 *
 * Everything here is pure. The one thing worth reading twice is
 * {@link normalizeLearnerName}: it is what makes a learner row in a workbook and a
 * student row in the database the *same* learner, so a change to it silently
 * un-matches every roster in the app.
 */

import { isLearnerName } from '$lib/features/excel/roster';
import type { Sf2CellMark } from '$lib/features/excel/types';

/** The cell text that marks a learner absent. */
export const SF2_ABSENT_MARK = 'X';

/**
 * Present is a blank cell.
 *
 * The SF2 form is opt-out: a day column holds an `X` for the learners who were
 * absent and nothing at all for everyone else, so writing an explicit "present"
 * mark would change what the form prints.
 */
const SF2_PRESENT_MARK = '';

/**
 * Re-exported from `$lib/features/excel/roster`, which already owns the rule that
 * separates a learner row from the form's own `TOTAL Per Day` rows. Two copies of
 * that test would let a workbook import as a roster with no learners in it.
 */
export { isLearnerName };

/** One learner row in a workbook, as the roster sync records it. */
type Sf2StudentMapping = {
	studentId: string;
	sheetName: string;
	rowIndex: number;
};

/** One attendance record, reduced to what the day-level rules need. */
export type Sf2AttendanceEvent = {
	studentId: string;
	eventType: string;
};

/**
 * The identity of a learner across the workbook and the database.
 *
 * Runs of whitespace collapse, `", "` loses its space, and the result is
 * uppercased: the form prints `Last, First` while the database holds whatever the
 * teacher typed, and the two have to land on the same string.
 */
export function normalizeLearnerName(name: string): string {
	return name.trim().split(/\s+/).join(' ').replace(/, /g, ',').trim().toUpperCase();
}

/**
 * Generate Excel marks for a day's attendance.
 *
 * With explicit absent records, the X mark is written ONLY for students who have
 * an explicit absent event. Everyone else (recorded present or untouched) stays
 * blank — present by default, matching the SF2 opt-out model.
 */
export function attendanceMarksForDay(
	students: readonly Sf2StudentMapping[],
	absentStudentIds: ReadonlySet<string>,
	columnLetter: string
): Sf2CellMark[] {
	return students.map((student) => ({
		sheetName: student.sheetName,
		address: `${columnLetter}${student.rowIndex}`,
		value: absentStudentIds.has(student.studentId) ? SF2_ABSENT_MARK : SF2_PRESENT_MARK
	}));
}
