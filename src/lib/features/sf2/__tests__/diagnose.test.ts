/**
 * The SF2 mark diagnostic — the test the brief names.
 *
 * The bulk of the coverage lives beside the code it covers:
 *
 * - `diagnose/__tests__/rules.test.ts` — the verdict rules, the false-zero rule and the
 *   database-side join, with no workbook and no database in them;
 * - `diagnose/__tests__/probe.test.ts` — the read-only pass over a workbook, against the
 *   real converted DepEd template;
 * - `diagnose/__tests__/db-read.test.ts` — every database read, and the no-write check;
 * - `diagnose/__tests__/diagnose-e2e.test.ts` — whole runs against a realistic install.
 *
 * What belongs here is the shape a caller sees: that the module exports one entry point,
 * that the answer it returns is the answer a teacher can act on, and that the parts hang
 * together into that answer. Kept separate so this file stays the thing to read first.
 */

import { describe, expect, it } from 'vitest';
import * as diagnose from '../diagnose/index';
import { diagnoseSf2Marks } from '../diagnose/diagnose';
import {
	incomparableSummary,
	permitsWrite,
	verdictFor,
	type MonthMarkComparison
} from '../diagnose/model';

/** The twelve months in school-year order, all readable, none with a missing mark. */
function schoolYear(): MonthMarkComparison[] {
	return [
		'JANUARY',
		'FEBRUARY',
		'MARCH',
		'APRIL',
		'MAY',
		'JUNE',
		'JULY',
		'AUGUST',
		'SEPTEMBER',
		'OCTOBER',
		'NOVEMBER',
		'DECEMBER'
	].map((reportMonth) => ({
		reportMonth,
		reportYear: reportMonth === 'JUNE' || reportMonth === 'SEPTEMBER' ? 2026 : 2027,
		counts: { workbookXCount: 4, dbAbsentCount: 4, dbMappedAbsentCount: 4, cellsScanned: 60 },
		cellsOnlyInWorkbook: [],
		cellsOnlyInDatabase: [],
		sourceStatus: 'Comparable' as const,
		reason: 'measured',
		mappingSource: 'perMonthTables' as const,
		rosterResolution: 'databaseRowMappings' as const,
		rosterRows: 3,
		dayColumns: 20
	}));
}

describe('the module boundary', () => {
	it('exports one entry point for a caller', () => {
		expect(typeof diagnoseSf2Marks).toBe('function');
		expect(diagnose.diagnoseSf2Marks).toBe(diagnoseSf2Marks);
	});

	it('exports everything the diagnostic answer is made of', () => {
		// A UI that renders this needs the verdict rules and the two lists that keep the
		// verdict honest. If one of these disappears, the view starts crying wolf.
		for (const name of [
			'verdictFor',
			'verdictReason',
			'incomparableSummary',
			'isComparable',
			'permitsWrite',
			'unplacedSheets',
			'workbookReports',
			'unmeasuredMonth'
		]) {
			expect(typeof diagnose[name as keyof typeof diagnose], name).toBe('function');
		}
	});

	it('exposes the measurement helpers a differential harness needs', () => {
		// Spec risk 4 asks for a harness that runs the old and the new code over the same
		// fixtures. That needs the pure comparison exported, which it is.
		for (const name of [
			'buildDateMappings',
			'buildStudentMappings',
			'databaseXCells',
			'workbookXCellsInScope',
			'diffCells',
			'absentCountInMonth',
			'attendanceScopeCells',
			'monthFromMeasurement'
		]) {
			expect(typeof diagnose[name as keyof typeof diagnose], name).toBe('function');
		}
	});

	it('keeps the read statements greppable from the module boundary', () => {
		// The no-write check in `db-read.test.ts` walks this list. Exporting it is what
		// makes that check about the whole read side rather than one file's worth of it.
		expect(diagnose.DIAGNOSTIC_STATEMENTS.length).toBeGreaterThanOrEqual(12);
	});
});

describe('the answer a teacher reads', () => {
	it('reads as one sentence when the workbook holds what the database lacks', () => {
		const months = schoolYear();
		months[8] = {
			...months[8]!,
			cellsOnlyInWorkbook: [
				{
					studentName: 'ALVARADO, ZYRON JAY  E.',
					date: '2026-09-01',
					sheetName: 'SEPTEMBER 2026',
					cellAddress: 'F8'
				}
			]
		};

		const verdict = verdictFor(months);

		expect(verdict).toBe('WorkbookIsSourceOfTruth');
		expect(diagnose.verdictReason(verdict, months, 0)).toBe(
			'1 X mark(s) in the workbook have no record in the database. The workbook is the only ' +
				'copy of those marks: import them before anything is written.'
		);
	});

	it('reads as one sentence when the database holds everything', () => {
		const months = schoolYear();
		const verdict = verdictFor(months);

		expect(verdict).toBe('DatabaseIsSourceOfTruth');
		expect(verdict).toBeTruthy();
		expect(permitsWrite(verdict)).toBe(true);
		expect(diagnose.verdictReason(verdict, months, 0)).toBe(
			'All 12 months were measurable and the database holds every mark the workbook shows, on ' +
				'the same cells.'
		);
	});

	it('refuses to conclude, and says so, when a month could not be read', () => {
		const months = schoolYear();
		const refused = diagnose.unmeasuredMonth(
			'AUGUST',
			2026,
			'NoSheet',
			'No worksheet names AUGUST 2026.'
		);
		months[7] = refused;

		const verdict = verdictFor(months);

		expect(verdict).toBe('Incomparable');
		expect(permitsWrite(verdict)).toBe(false);
		expect(diagnose.verdictReason(verdict, months, 0)).toContain(
			'1 of 12 month(s) could not be measured: AUGUST 2026 (NoSheet)'
		);
		expect(diagnose.verdictReason(verdict, months, 0)).toContain('No conclusion is drawn');
		expect(incomparableSummary(refused)).toBe(
			'AUGUST 2026: NoSheet - No worksheet names AUGUST 2026.'
		);
	});

	it('refuses to conclude for an empty school year rather than reassuring', () => {
		expect(verdictFor([])).toBe('Incomparable');
		expect(diagnose.verdictReason('Incomparable', [], 0)).toBe('No month could be measured.');
	});
});
