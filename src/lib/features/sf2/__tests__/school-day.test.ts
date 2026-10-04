/**
 * School-day derivation and the first attendance day of a month.
 *
 * A wrong day here silently mis-dates a month, so the cases below are the ones the
 * Rust module documented as its own edge cases: an unset start date (no default,
 * ever), a class that starts mid-month, a class that starts on a weekend, and a
 * month that classes have not started yet at all (spec E2).
 */
import { describe, expect, it } from 'vitest';
import {
	FIRST_SCHOOL_DAY_UNDETERMINED,
	effectiveFirstSchoolDay,
	gridAnchorDay,
	isSchoolDay,
	knownFirstSchoolDay,
	lastDayOfMonth,
	naiveDate,
	parseIsoDate,
	deriveFirstSchoolDay,
	formatIsoDate,
	needsSchoolStartDatePrompt,
	reportYearForSchoolMonth,
	resolveFirstSchoolDay,
	schoolYearStartYear,
	schoolYearYears
} from '../first-school-day';
import {
	defaultSf2FirstSchoolDay,
	firstSchoolDayForReportMonth,
	sf2MonthNumber,
	sf2ReportYear,
	validateFirstSchoolDay
} from '../calendar';

function appErrorOf(run: () => unknown): { kind: string; detail: string } {
	try {
		run();
	} catch (thrown) {
		return thrown as { kind: string; detail: string };
	}
	throw new Error('expected an AppError, nothing was thrown');
}

const start = (value: string): Date => parseIsoDate(value)!;

describe('dates', () => {
	it('rejects a day the month does not have', () => {
		expect(naiveDate(2026, 2, 29)).toBeUndefined();
		expect(naiveDate(2024, 2, 29)?.getUTCDate()).toBe(29);
		expect(naiveDate(2026, 13, 1)).toBeUndefined();
	});

	it('reads only the padded ISO form', () => {
		expect(parseIsoDate('2026-06-03')?.getUTCDay()).toBe(3);
		expect(parseIsoDate('2026-6-3')).toBeUndefined();
		expect(parseIsoDate('')).toBeUndefined();
		expect(formatIsoDate(start('2026-06-03'))).toBe('2026-06-03');
	});

	it('knows the length of every month', () => {
		expect(lastDayOfMonth(2026, 2)).toBe(28);
		expect(lastDayOfMonth(2024, 2)).toBe(29);
		expect(lastDayOfMonth(2026, 12)).toBe(31);
		expect(lastDayOfMonth(2026, 4)).toBe(30);
	});
});

describe('the school year', () => {
	it('wraps at September', () => {
		expect(reportYearForSchoolMonth('2026-2027', 9, 2030)).toBe(2026);
		expect(reportYearForSchoolMonth('2026-2027', 12, 2030)).toBe(2026);
		expect(reportYearForSchoolMonth('2026-2027', 1, 2030)).toBe(2027);
		expect(reportYearForSchoolMonth('2026-2027', 8, 2030)).toBe(2027);
	});

	it('reads both spellings of one school year the same way', () => {
		expect(schoolYearYears('2026 - 2027')).toEqual([2026, 2027]);
		expect(schoolYearYears('SY 2026-2027')).toEqual([2026, 2027]);
		expect(schoolYearStartYear('2026')).toBe(2026);
		expect(schoolYearStartYear('no year here')).toBeUndefined();
	});

	it('falls back to the caller year when the label has none', () => {
		expect(reportYearForSchoolMonth('no year here', 9, 2030)).toBe(2030);
	});
});

describe('isSchoolDay', () => {
	it('is Monday to Friday and never a weekend', () => {
		expect(isSchoolDay(start('2026-09-07'))).toBe(true);
		expect(isSchoolDay(start('2026-09-11'))).toBe(true);
		expect(isSchoolDay(start('2026-09-12'))).toBe(false);
		expect(isSchoolDay(start('2026-09-13'))).toBe(false);
	});
});

describe('deriveFirstSchoolDay', () => {
	it('is undefined while the start date is unset - there is no default', () => {
		expect(deriveFirstSchoolDay(undefined, 9, 2026)).toBeUndefined();
		expect(needsSchoolStartDatePrompt(undefined)).toBe(true);
		expect(needsSchoolStartDatePrompt(start('2026-09-07'))).toBe(false);
	});

	it('dates a month the class started before from its own first school day', () => {
		// October 2026 opens on a Thursday.
		expect(deriveFirstSchoolDay(start('2026-09-07'), 10, 2026)).toBe(1);
	});

	it('dates a month the class started in from the start date', () => {
		expect(deriveFirstSchoolDay(start('2026-09-07'), 9, 2026)).toBe(7);
	});

	it('steps off a weekend start', () => {
		// 5 September 2026 is a Saturday.
		expect(deriveFirstSchoolDay(start('2026-09-05'), 9, 2026)).toBe(7);
	});

	it('is undefined for a month classes have not started yet (spec E2)', () => {
		expect(deriveFirstSchoolDay(start('2026-09-07'), 8, 2026)).toBeUndefined();
	});
});

describe('the not-derived-yet sentinel', () => {
	it('is 0, and 0 is never a day', () => {
		expect(FIRST_SCHOOL_DAY_UNDETERMINED).toBe(0);
		expect(knownFirstSchoolDay(0)).toBeUndefined();
		expect(knownFirstSchoolDay(7)).toBe(7);
		expect(knownFirstSchoolDay(undefined)).toBeUndefined();
	});

	it('never wins over a derived day', () => {
		expect(effectiveFirstSchoolDay(0, 5)).toBe(5);
		expect(effectiveFirstSchoolDay(3, 5)).toBe(3);
		expect(effectiveFirstSchoolDay(0, undefined)).toBeUndefined();
	});

	it('is not read as a day by the resolver, nor is it written as one', () => {
		const derived = deriveFirstSchoolDay(start('2026-09-07'), 9, 2026);
		expect(resolveFirstSchoolDay(start('2026-09-07'), 9, 2026, 0, 4)).toBe(derived);
		expect(resolveFirstSchoolDay(undefined, 9, 2026, 0, 4)).toBe(4);
		expect(resolveFirstSchoolDay(undefined, 9, 2026, 0, 0)).toBeUndefined();
	});

	it('anchors the grid on day 1 rather than claiming the class started then', () => {
		expect(gridAnchorDay(undefined, 2026, 9)).toBe(1);
		expect(gridAnchorDay(0, 2026, 9)).toBe(1);
		expect(gridAnchorDay(7, 2026, 9)).toBe(7);
		expect(gridAnchorDay(31, 2026, 9)).toBe(1);
	});
});

describe('month names and the report year', () => {
	it('resolves a month from any of the spellings a form uses', () => {
		expect(sf2MonthNumber('SEPT.')).toBe(9);
		expect(sf2MonthNumber(' june ')).toBe(6);
		expect(sf2MonthNumber('COMPLETE DAYS')).toBeUndefined();
	});

	it('wraps the legacy template year at June', () => {
		expect(sf2ReportYear('2025-2026', 6, 2030)).toBe(2025);
		expect(sf2ReportYear('2025-2026', 5, 2030)).toBe(2026);
		expect(sf2ReportYear('nonsense', 6, 2030)).toBe(2030);
	});
});

describe('the first attendance day of a report month', () => {
	it('defaults to the first Monday-Friday of the month', () => {
		// June 2025 opens on a Sunday; September 2025 opens on a Monday.
		expect(defaultSf2FirstSchoolDay('JUNE', '2025-2026')).toBe(2);
		expect(defaultSf2FirstSchoolDay('SEPTEMBER', '2025-2026')).toBe(1);
	});

	it('prefers the earliest day the workbook already recorded', () => {
		expect(firstSchoolDayForReportMonth('JUNE', '2025-2026', ['2025-06-09', '2025-06-02'])).toBe(2);
	});

	it('skips a recorded day that is not a school day', () => {
		// 7 June 2025 is a Saturday, and 31 May is not even in June.
		expect(firstSchoolDayForReportMonth('JUNE', '2025-2026', ['2025-05-31', '2025-06-07'])).toBe(2);
	});

	it('refuses a report month that names no month', () => {
		expect(appErrorOf(() => defaultSf2FirstSchoolDay('smiling', '2025-2026'))).toEqual({
			kind: 'InvalidInput',
			detail: 'Report Month must be a valid month name'
		});
	});
});

describe('validateFirstSchoolDay', () => {
	it('accepts a school day inside the month', () => {
		expect(() => validateFirstSchoolDay(2, 'JUNE', '2025-2026')).not.toThrow();
	});

	it('rejects a weekend', () => {
		expect(appErrorOf(() => validateFirstSchoolDay(1, 'JUNE', '2025-2026'))).toEqual({
			kind: 'InvalidInput',
			detail: 'First attendance day must be a Monday-Friday school day'
		});
	});

	it('rejects a day the month does not have, naming the last day it does', () => {
		expect(appErrorOf(() => validateFirstSchoolDay(31, 'JUNE', '2025-2026'))).toEqual({
			kind: 'InvalidInput',
			detail: 'First attendance day must be between 1 and 30 for this report month'
		});
	});
});
