/**
 * The Settings → SF2 Workbook screen's logic, without the screen.
 *
 * Three claims are tested here because each is a way for the Settings page to
 * quietly lie to a teacher about their own attendance records:
 *
 * 1. A month file nobody has counted is **unmeasured**, not empty. Rendering it
 *    as "0 X marks" is how a list of real marks ends up looking like a school
 *    with no absences, and it is the same confusion §9.1's guard exists to
 *    prevent.
 * 2. A school year runs SEPTEMBER → AUGUST, so the list is sorted in that order.
 * 3. *Classes started on* has no default. There is no input for one, and no
 *    branch that invents one, because a guessed start date mis-dates every month
 *    file in the year.
 */
import { describe, it, expect } from 'vitest';
import {
	isSchoolStartDateValid,
	normalizeSchoolStartDate,
	sf2MonthWorkbookRows,
	sf2MonthWorkbookSummary,
	sf2SplitIsComplete,
	sf2SplitNeedsAttention,
	sf2SplitSummary,
	SF2_SCHOOL_YEAR_ORDER,
	type Sf2MonthWorkbookRow
} from './sf2-months';
import type { Sf2MonthPreview, Sf2SplitOutcome } from '$lib/types';

function monthRow(overrides: Partial<Sf2MonthPreview> & { month: string }): Sf2MonthPreview {
	return {
		reportYear: 2026,
		schoolYear: '2026-2027',
		fileName: `SF2-${overrides.month}-2026.xls`,
		fileExists: true,
		hasTemplate: true,
		firstSchoolDay: 1,
		firstSchoolDayOverridden: false,
		workbookXCount: 0,
		learnerCount: 40,
		mappedDateCount: 22,
		...overrides
	};
}

describe('the month workbooks list', () => {
	it('sorts SEPTEMBER → AUGUST, not alphabetically', () => {
		const rows = sf2MonthWorkbookRows([
			monthRow({ month: 'JUNE', reportYear: 2027 }),
			monthRow({ month: 'JANUARY', reportYear: 2027 }),
			monthRow({ month: 'SEPTEMBER', reportYear: 2026 }),
			monthRow({ month: 'APRIL', reportYear: 2027 })
		]);
		expect(rows.map((row) => row.month)).toEqual(['SEPTEMBER', 'JANUARY', 'APRIL', 'JUNE']);
	});

	it('keeps the year each month actually falls in', () => {
		// SEPTEMBER 2026 and JUNE 2027 are one school year. Printing the first
		// year's number on both is how a file gets filed under the wrong year.
		const [september, june] = sf2MonthWorkbookRows([
			monthRow({ month: 'SEPTEMBER', reportYear: 2026 }),
			monthRow({ month: 'JUNE', reportYear: 2027 })
		]);
		expect(september?.label).toBe('September 2026');
		expect(june?.label).toBe('June 2027');
	});

	it('shows a never-measured file as unmeasured, not as zero X marks', () => {
		const [row] = sf2MonthWorkbookRows([
			monthRow({ month: 'SEPTEMBER', workbookXCount: 0, workbookScannedAt: null })
		]);
		expect(row?.xCount).toBeNull();
		expect(row?.tone).toBe('ready');
		// The distinction that matters: the file is there and fine, it simply has
		// not been counted.
		expect(row?.fileState).toBe('Present');
	});

	it('shows the counted X marks once the file has been scanned', () => {
		const [row] = sf2MonthWorkbookRows([
			monthRow({
				month: 'SEPTEMBER',
				workbookXCount: 12,
				workbookScannedAt: 1_789_000_000,
				lastSyncedAt: 1_789_000_500
			})
		]);
		expect(row?.xCount).toBe(12);
		expect(row?.lastSynced).not.toBe('Never');
	});

	it('flags a stored row whose file is gone (edge case E4)', () => {
		const [row] = sf2MonthWorkbookRows([
			monthRow({ month: 'SEPTEMBER', fileExists: false, workbookXCount: 12 })
		]);
		expect(row?.tone).toBe('attention');
		expect(row?.fileState).toBe('Missing');
	});

	it('marks an unbuilt month pending rather than missing', () => {
		// `Sf2MonthPreview::without_template` stores `first_school_day: 0` - the
		// sentinel for "undated", never a guessed day. A month nobody has built
		// has no first day, and the list has to say so rather than print day 1.
		const [row] = sf2MonthWorkbookRows([
			monthRow({ month: 'SEPTEMBER', hasTemplate: false, fileExists: false, firstSchoolDay: 0 })
		]);
		expect(row?.fileState).toBe('Not set up');
		expect(row?.tone).toBe('pending');
		expect(row?.undated).toBe(true);
	});

	it('distinguishes a hand-typed first day from a derived one', () => {
		// Re-derivation never touches an override, so the list has to be able to
		// say which kind it is showing.
		const [derived, overridden] = sf2MonthWorkbookRows([
			monthRow({ month: 'SEPTEMBER', firstSchoolDay: 3, firstSchoolDayOverridden: false }),
			monthRow({ month: 'OCTOBER', firstSchoolDay: 10, firstSchoolDayOverridden: true })
		]);
		expect(derived?.undated).toBe(false);
		expect(derived?.firstSchoolDayOverridden).toBe(false);
		expect(overridden?.firstSchoolDayOverridden).toBe(true);
	});

	it('summarises a year with a missing file as needing attention', () => {
		const rows: Sf2MonthWorkbookRow[] = sf2MonthWorkbookRows([
			monthRow({ month: 'SEPTEMBER', workbookXCount: 12, workbookScannedAt: 1 }),
			monthRow({ month: 'OCTOBER', fileExists: false, workbookXCount: 4, workbookScannedAt: 1 })
		]);
		const summary = sf2MonthWorkbookSummary(rows);
		expect(summary).toContain('1 of 2 month files ready');
		expect(summary).toContain('1 missing from disk');
		expect(summary).toContain('16 X marks counted');
	});

	it('says nothing is measured when nothing has been scanned', () => {
		const rows = sf2MonthWorkbookRows([monthRow({ month: 'SEPTEMBER' })]);
		expect(sf2MonthWorkbookSummary(rows)).toContain('0 measured');
	});

	it('knows a school year is twelve months long', () => {
		expect(SF2_SCHOOL_YEAR_ORDER).toHaveLength(12);
		expect(SF2_SCHOOL_YEAR_ORDER[0]).toBe('SEPTEMBER');
		expect(SF2_SCHOOL_YEAR_ORDER[11]).toBe('AUGUST');
	});
});

describe('"Classes started on"', () => {
	it('accepts a real ISO date', () => {
		expect(normalizeSchoolStartDate('2026-08-03')).toBe('2026-08-03');
		expect(normalizeSchoolStartDate('  2026-08-03  ')).toBe('2026-08-03');
	});

	it('treats blank as unset, not as an error and not as a default', () => {
		expect(normalizeSchoolStartDate('')).toBeNull();
		expect(normalizeSchoolStartDate('   ')).toBeNull();
	});

	it('rejects a date that does not exist', () => {
		// The shape is right and the day is not. A regex cannot catch this; only
		// the round-trip can.
		expect(normalizeSchoolStartDate('2026-02-31')).toBeNull();
		expect(normalizeSchoolStartDate('2026-13-01')).toBeNull();
		expect(normalizeSchoolStartDate('2026-00-10')).toBeNull();
	});

	it('rejects anything that is not YYYY-MM-DD', () => {
		for (const value of ['08/03/2026', 'August 3 2026', '2026-8-3', '20260803', 'today']) {
			expect(normalizeSchoolStartDate(value), value).toBeNull();
			expect(isSchoolStartDateValid(value), value).toBe(false);
		}
	});

	it('calls an empty field valid, because clearing it is a legitimate answer', () => {
		expect(isSchoolStartDateValid('')).toBe(true);
		expect(isSchoolStartDateValid('2026-08-03')).toBe(true);
	});

	it('has no default to fall back on', () => {
		// The controller's ruling, as a test: a wrong default mis-dates every month
		// file in the school year, so the function that normalises the field has no
		// argument for "today" and no fallback branch.
		expect(normalizeSchoolStartDate.length).toBe(1);
		expect(normalizeSchoolStartDate.toString()).not.toMatch(/\?\?/);
	});
});

describe('the merge result', () => {
	const outcome = (overrides: Partial<Sf2SplitOutcome> = {}): Sf2SplitOutcome => ({
		splitCompletedAt: null,
		months: [],
		verifiedCount: 11,
		needsAttentionCount: 1,
		workbookPath: 'C:/appdata/sf2-workbooks/SF2-1-A-1a2b.xls',
		legacyFilePath: 'C:/appdata/sf2-workbooks/_legacy/SF2-1-A-1a2b.xls',
		legacyBackupPath: null,
		absencesOutsideSchoolYear: 0,
		message: '',
		...overrides
	});

	it('prefers the sentence the backend wrote', () => {
		const message =
			'Rebuilt one workbook with 12 month sheets. 11 verified, 1 needs attention (JUNE 2027).';
		expect(sf2SplitSummary(outcome({ message }))).toBe(message);
	});

	it('always says the original workbook was kept', () => {
		// Both wordings, because the guarantee is the point: the legacy file is
		// never deleted, and the teacher has to be able to rely on that without
		// having read the spec.
		expect(sf2SplitSummary(outcome())).toContain('_legacy');
		expect(sf2SplitSummary(outcome({ message: 'All twelve months are ready.' }))).toBe(
			'All twelve months are ready.'
		);
	});

	it('names only the months a human still has to do something about', () => {
		const summary = sf2SplitNeedsAttention(
			outcome({
				months: [
					{
						reportMonth: 'JUNE',
						reportYear: 2027,
						sheetName: 'JUNE 2027',
						fileName: 'one.xls',
						status: 'verified',
						xMarks: 0,
						learnerRows: 40,
						detail: null
					},
					{
						reportMonth: 'JULY',
						reportYear: 2027,
						sheetName: 'JULY 2027',
						fileName: 'one.xls',
						status: 'needsAttention',
						xMarks: 0,
						learnerRows: 40,
						detail: 'roster mismatch'
					},
					{
						reportMonth: 'AUGUST',
						reportYear: 2027,
						sheetName: 'AUGUST 2027',
						fileName: 'one.xls',
						status: 'alreadyMerged',
						xMarks: 3,
						learnerRows: 40,
						detail: null
					}
				]
			})
		);
		expect(summary).toEqual(['July 2027']);
	});

	it('is complete only once the backend recorded a completion', () => {
		expect(sf2SplitIsComplete(outcome({ splitCompletedAt: 1_789_000_000 }))).toBe(true);
		expect(sf2SplitIsComplete(outcome())).toBe(false);
	});
});
