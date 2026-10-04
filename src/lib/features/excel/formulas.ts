/**
 * The SF2 formula subset, computed in TypeScript.
 *
 * ExcelJS writes formulas but never evaluates them, and the app compares X
 * counts programmatically without opening Excel. So a cell that carries a
 * formula must also carry the value Excel would have cached for it (migration
 * spec D7). Every function here returns that pair: the formula text to write
 * and the number it evaluates to.
 *
 * The formula strings are ports of
 * `src-tauri/src/sf2/attendance/attendance_marks.rs`. A new formula shape in
 * the template means a new function here and a test.
 */

import {
	SF2_ATTENDANCE_FIRST_COLUMN,
	SF2_ATTENDANCE_LAST_COLUMN,
	SF2_SUMMARY_ROWS,
	SF2_TOTAL_DAYS_REF
} from './constants';
import { columnLetter } from './workbook';

/** A formula to write into a cell together with the value Excel caches for it. */
export type Sf2Formula = {
	/** Formula text *without* the leading `=`, the way ExcelJS stores it. */
	formula: string;
	/** The result Excel caches, so the cell is never blank before a recalculation. */
	value: number;
};

/** The mark a learner leaves in the day grid: `X` means absent. */
export const SF2_ABSENT_MARK = 'X';

/** `F` - the first day column of the attendance block. */
const DAY_FIRST_LETTER = columnLetter(SF2_ATTENDANCE_FIRST_COLUMN);

/** `AL` - the last day column of the attendance block. */
const DAY_LAST_LETTER = columnLetter(SF2_ATTENDANCE_LAST_COLUMN);

/**
 * `COUNTIF` semantics: how many of `marks` are the absent mark.
 *
 * Excel text criteria are case-insensitive, so a lowercase `x` counts too.
 */
export function countAbsentMarks(marks: Iterable<string | undefined>): number {
	let count = 0;
	for (const mark of marks) {
		if (mark && mark.trim().toUpperCase() === SF2_ABSENT_MARK) count += 1;
	}
	return count;
}

// ── Learner row: ABSENT / PRESENT for the month ──────────────────────────────

/**
 * `AM{row}` - the learner's absent days for the month.
 *
 * `COUNTIF(F8:AL8,"X")` over the whole day block.
 */
export function learnerAbsentTotal(
	row: number,
	dayMarks: Iterable<string | undefined>
): Sf2Formula {
	return {
		formula: `COUNTIF(${DAY_FIRST_LETTER}${row}:${DAY_LAST_LETTER}${row},"${SF2_ABSENT_MARK}")`,
		value: countAbsentMarks(dayMarks)
	};
}

/**
 * `AO{row}` - the learner's present days for the month.
 *
 * `$AW$5-AM{row}`: total school days minus absences. `AW5` is the form's
 * `TOTAL NO. OF DAYS`, which the app rewrites to the real mapped day count.
 */
export function learnerPresentTotal(
	row: number,
	totalDays: number,
	absentDays: number
): Sf2Formula {
	return {
		formula: `${SF2_TOTAL_DAYS_REF}-AM${row}`,
		value: totalDays - absentDays
	};
}

// ── Block subtotals: MALE / FEMALE / Combined ────────────────────────────────

/**
 * `AM{totalRow}` - total absent days across a block of learner rows.
 *
 * `SUM(AM8:AN28)` - the range deliberately runs to `AN`, the far side of the
 * merged `AM:AN` ABSENT cell, exactly as the bundled template writes it.
 */
export function blockAbsentTotal(
	firstRow: number,
	lastRow: number,
	absentTotals: readonly number[]
): Sf2Formula {
	return {
		formula: `SUM(AM${firstRow}:AN${lastRow})`,
		value: absentTotals.reduce((total, value) => total + value, 0)
	};
}

/**
 * `AO{totalRow}` - total present days across a block of learner rows.
 *
 * `$AW$5*{learnerCount}-AM{totalRow}`. Derived from the roster size rather than
 * summed from the rows, because an unassigned slot would contribute a spurious
 * zero present day.
 */
export function blockPresentTotal(
	totalRow: number,
	learnerCount: number,
	totalDays: number,
	blockAbsent: number
): Sf2Formula {
	return {
		formula: `${SF2_TOTAL_DAYS_REF}*${learnerCount}-AM${totalRow}`,
		value: totalDays * learnerCount - blockAbsent
	};
}

/** `AM{combined}` - `AM{maleTotal}+AM{femaleTotal}`. */
export function combinedAbsentTotal(
	maleTotalRow: number,
	femaleTotalRow: number,
	maleAbsent: number,
	femaleAbsent: number
): Sf2Formula {
	return {
		formula: `AM${maleTotalRow}+AM${femaleTotalRow}`,
		value: maleAbsent + femaleAbsent
	};
}

/** `AO{combined}` - `AO{maleTotal}+AO{femaleTotal}`. */
export function combinedPresentTotal(
	maleTotalRow: number,
	femaleTotalRow: number,
	malePresent: number,
	femalePresent: number
): Sf2Formula {
	return {
		formula: `AO${maleTotalRow}+AO${femaleTotalRow}`,
		value: malePresent + femalePresent
	};
}

// ── Daily present counts: the TOTAL Per Day rows ─────────────────────────────

/**
 * `{column}{totalRow}` - how many learners were present on one day.
 *
 * `{learnerCount}-COUNTIF(F8:F28,"X")`. The `X` marks are absences, so present
 * is the roster minus the absences. Empty template slots never hold an `X`, so
 * they cannot affect the count.
 */
export function dailyPresentTotal(
	learnerCount: number,
	column: string,
	firstRow: number,
	lastRow: number,
	absentMarks: Iterable<string | undefined>
): Sf2Formula {
	return {
		formula: `${learnerCount}-COUNTIF(${column}${firstRow}:${column}${lastRow},"${SF2_ABSENT_MARK}")`,
		value: learnerCount - countAbsentMarks(absentMarks)
	};
}

/** `{column}{combinedTotalRow}` - `{column}{maleTotal}+{column}{femaleTotal}`. */
export function combinedDailyPresentTotal(
	column: string,
	maleTotalRow: number,
	femaleTotalRow: number,
	malePresent: number,
	femalePresent: number
): Sf2Formula {
	return {
		formula: `${column}${maleTotalRow}+${column}${femaleTotalRow}`,
		value: malePresent + femalePresent
	};
}

// ── Summary block: rows 53-65, columns AR/AS/AT ───────────────────────────────

/**
 * The five cells row 59 reads, for one summary column.
 *
 * `enrolment` is row 53, which the caller writes as a static mark and which
 * differs per column - boys, girls, combined - so it is passed alongside the
 * counts rather than assumed.
 */
type Sf2RegisteredLearnerInputs = {
	/** Row 53, enrolment. */
	enrolment: number;
	/** Row 55, late enrolment. */
	lateEnrolment: number;
	/** Row 67, dropped out (NLS). */
	droppedOut: number;
	/** Row 69, transferred out. */
	transferredOut: number;
	/** Row 71, transferred in. */
	transferredIn: number;
};

/**
 * `{column}59` - `Registered Learners as of end of the month`.
 *
 * `enrolment + lateEnrolment - droppedOut - transferredOut + transferredIn`.
 * The counts are per summary column: a girl who transfers out leaves the boys'
 * figure untouched.
 */
export function registeredLearners(column: string, inputs: Sf2RegisteredLearnerInputs): Sf2Formula {
	const rows = SF2_SUMMARY_ROWS;
	return {
		formula:
			`${column}${rows.enrolment}+${column}${rows.lateEnrolment}` +
			`-${column}${rows.droppedOut}-${column}${rows.transferredOut}` +
			`+${column}${rows.transferredIn}`,
		value:
			inputs.enrolment +
			inputs.lateEnrolment -
			inputs.droppedOut -
			inputs.transferredOut +
			inputs.transferredIn
	};
}

/** `{column}61` - `Percentage of Enrolment`, as 0 when nobody is enrolled. */
export function percentageOfEnrolment(
	column: string,
	enrolment: number,
	registered: number
): Sf2Formula {
	const rows = SF2_SUMMARY_ROWS;
	return {
		formula: `IF(${column}${rows.enrolment}>0,${column}${rows.registeredLearners}/${column}${rows.enrolment}*100,0)`,
		value: enrolment > 0 ? (registered / enrolment) * 100 : 0
	};
}

/**
 * `{column}63` - `Average Daily Attendance`.
 *
 * `IFERROR(AVERAGE(F{totalRow}:AL{totalRow}),0)`: the mean of the *numeric*
 * TOTAL Per Day cells, so a day with no total written yet is skipped rather
 * than counted as a zero. `dayCounts` must therefore be only the numbers the
 * row actually holds.
 */
export function averageDailyAttendance(totalRow: number, dayCounts: readonly number[]): Sf2Formula {
	const total = dayCounts.reduce((sum, value) => sum + value, 0);
	return {
		formula: `IFERROR(AVERAGE(${DAY_FIRST_LETTER}${totalRow}:${DAY_LAST_LETTER}${totalRow}),0)`,
		value: dayCounts.length === 0 ? 0 : total / dayCounts.length
	};
}

/** `{column}65` - `Percentage of Attendance`, as 0 when nobody is registered. */
export function percentageOfAttendance(
	column: string,
	registered: number,
	averageDaily: number
): Sf2Formula {
	const rows = SF2_SUMMARY_ROWS;
	return {
		formula: `IF(${column}${rows.registeredLearners}>0,${column}${rows.averageDailyAttendance}/${column}${rows.registeredLearners}*100,0)`,
		value: registered > 0 ? (averageDaily / registered) * 100 : 0
	};
}
