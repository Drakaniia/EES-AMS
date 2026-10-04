import { describe, expect, it } from 'vitest';
import {
	averageDailyAttendance,
	blockAbsentTotal,
	blockPresentTotal,
	combinedAbsentTotal,
	combinedDailyPresentTotal,
	combinedPresentTotal,
	countAbsentMarks,
	dailyPresentTotal,
	learnerAbsentTotal,
	learnerPresentTotal,
	percentageOfAttendance,
	percentageOfEnrolment,
	registeredLearners
} from '../formulas';

/**
 * Both halves of every shape are asserted: the formula text has to stay byte-identical
 * to what the Rust wrote, and the cached value has to be what Excel would compute -
 * a cell with the right formula and a blank cache reads as zero until someone opens
 * the file (spec D7).
 */
describe('countAbsentMarks', () => {
	it('counts only the absent mark, case-insensitively', () => {
		expect(countAbsentMarks(['X', '', 'x', 'X ', 'present', undefined])).toBe(3);
	});

	it('counts an empty block as zero', () => {
		expect(countAbsentMarks([])).toBe(0);
	});
});

describe('learnerAbsentTotal', () => {
	it('writes COUNTIF over the whole day block', () => {
		expect(learnerAbsentTotal(8, ['X', '', 'X'])).toEqual({
			formula: 'COUNTIF(F8:AL8,"X")',
			value: 2
		});
	});

	it('is zero for a learner who was never absent', () => {
		expect(learnerAbsentTotal(43, ['', ''])).toEqual({
			formula: 'COUNTIF(F43:AL43,"X")',
			value: 0
		});
	});
});

describe('learnerPresentTotal', () => {
	it('subtracts absences from the TOTAL NO. OF DAYS cell', () => {
		expect(learnerPresentTotal(8, 11, 1)).toEqual({ formula: '$AW$5-AM8', value: 10 });
	});

	it('is the full day count when nothing is absent', () => {
		expect(learnerPresentTotal(43, 22, 0)).toEqual({ formula: '$AW$5-AM43', value: 22 });
	});
});

describe('blockAbsentTotal', () => {
	it('sums the AM column of a learner block', () => {
		expect(blockAbsentTotal(8, 28, [1, 0, 3, 1])).toEqual({
			formula: 'SUM(AM8:AN28)',
			value: 5
		});
	});

	it('is zero for a block with no absences', () => {
		expect(blockAbsentTotal(30, 48, [0, 0])).toEqual({ formula: 'SUM(AM30:AN48)', value: 0 });
	});
});

describe('blockPresentTotal', () => {
	it('multiplies the day count by the roster size, then subtracts absences', () => {
		expect(blockPresentTotal(29, 12, 11, 5)).toEqual({
			formula: '$AW$5*12-AM29',
			value: 127
		});
	});

	it('is zero when the whole block was absent on every day', () => {
		expect(blockPresentTotal(49, 14, 11, 154)).toEqual({
			formula: '$AW$5*14-AM49',
			value: 0
		});
	});
});

describe('combinedAbsentTotal / combinedPresentTotal', () => {
	it('adds the two gender subtotals', () => {
		expect(combinedAbsentTotal(29, 49, 5, 8)).toEqual({ formula: 'AM29+AM49', value: 13 });
		expect(combinedPresentTotal(29, 49, 127, 146)).toEqual({
			formula: 'AO29+AO49',
			value: 273
		});
	});
});

describe('dailyPresentTotal', () => {
	it('is the roster minus the X marks in that column', () => {
		expect(dailyPresentTotal(12, 'R', 8, 28, ['X', '', 'X', ''])).toEqual({
			formula: '12-COUNTIF(R8:R28,"X")',
			value: 10
		});
	});

	it('is the full roster on a day nobody was absent', () => {
		expect(dailyPresentTotal(14, 'AF', 30, 48, ['', '', ''])).toEqual({
			formula: '14-COUNTIF(AF30:AF48,"X")',
			value: 14
		});
	});

	it('never goes negative for an out-of-roster X left in an empty slot', () => {
		// Documented behaviour, not a hope: an X in a slot with no learner pushes the
		// count below the roster. Excel computes the same number.
		expect(dailyPresentTotal(12, 'AL', 8, 28, ['X']).value).toBe(11);
	});
});

describe('combinedDailyPresentTotal', () => {
	it('adds the two TOTAL Per Day cells of one column', () => {
		expect(combinedDailyPresentTotal('AB', 29, 49, 12, 12)).toEqual({
			formula: 'AB29+AB49',
			value: 24
		});
	});
});

describe('registeredLearners', () => {
	const counts = {
		enrolment: 26,
		lateEnrolment: 1,
		droppedOut: 2,
		transferredOut: 1,
		transferredIn: 2
	};

	it('writes the five-term form expression', () => {
		expect(registeredLearners('AR', counts)).toEqual({
			formula: 'AR53+AR55-AR67-AR69+AR71',
			value: 26
		});
	});

	it('is the same shape in every summary column', () => {
		expect(registeredLearners('AT', counts).formula).toBe('AT53+AT55-AT67-AT69+AT71');
	});
});

describe('percentageOfEnrolment', () => {
	it('divides registered by enrolment', () => {
		expect(percentageOfEnrolment('AR', 26, 26)).toEqual({
			formula: 'IF(AR53>0,AR59/AR53*100,0)',
			value: 100
		});
	});

	it('is zero rather than a division error for an empty class', () => {
		expect(percentageOfEnrolment('AS', 0, 0)).toEqual({
			formula: 'IF(AS53>0,AS59/AS53*100,0)',
			value: 0
		});
	});
});

describe('averageDailyAttendance', () => {
	it('averages only the numeric day cells', () => {
		// The 15 day totals JUNE 2025's MALE TOTAL row actually carries: 173.
		const dayTotals = [11, 11, 11, 11, 12, 12, 11, 11, 12, 12, 12, 12, 11, 12, 12];
		expect(averageDailyAttendance(29, dayTotals)).toEqual({
			formula: 'IFERROR(AVERAGE(F29:AL29),0)',
			value: 173 / 15
		});
	});

	it('is zero for a row with no numbers at all, which is what IFERROR guards', () => {
		expect(averageDailyAttendance(50, [])).toEqual({
			formula: 'IFERROR(AVERAGE(F50:AL50),0)',
			value: 0
		});
	});
});

describe('percentageOfAttendance', () => {
	it('divides average daily attendance by registered learners', () => {
		expect(percentageOfAttendance('AR', 26, 173 / 15)).toEqual({
			formula: 'IF(AR59>0,AR63/AR59*100,0)',
			value: (173 / 15 / 26) * 100
		});
	});

	it('is zero when nobody is registered', () => {
		expect(percentageOfAttendance('AT', 0, 24.8181818181818)).toEqual({
			formula: 'IF(AT59>0,AT63/AT59*100,0)',
			value: 0
		});
	});
});
