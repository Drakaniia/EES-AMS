import { describe, expect, it } from 'vitest';
import { isComparable, permitsWrite, verdictFor, verdictReason } from '../model';
import type { MarkCell, MonthMarkComparison } from '../model';

/**
 * The rules, with no workbook and no database in them.
 *
 * Two families live here and the second is the one this engine exists for: the verdict
 * rules, and the false-zero rule - every other outcome is a refusal with a reason, and
 * never a count of `0`.
 */

// ── fixtures ──────────────────────────────────────────────────────────────

function markCell(index: number): MarkCell {
	return {
		studentName: `Learner ${index}`,
		date: `2026-09-${String(index + 1).padStart(2, '0')}`,
		sheetName: 'SEPTEMBER 2026',
		cellAddress: `H${index + 8}`
	};
}

/** A month the diagnostic actually measured. */
function measured(workbookOnly: number, databaseOnly: number): MonthMarkComparison {
	return {
		reportMonth: 'SEPTEMBER',
		reportYear: 2026,
		sourceStatus: 'Comparable',
		reason: 'measured',
		counts: {
			workbookXCount: workbookOnly + 4,
			dbAbsentCount: databaseOnly + 4,
			dbMappedAbsentCount: databaseOnly + 4,
			cellsScanned: 600
		},
		cellsOnlyInWorkbook: Array.from({ length: workbookOnly }, (_, index) => markCell(index)),
		cellsOnlyInDatabase: Array.from({ length: databaseOnly }, (_, index) => markCell(index)),
		mappingSource: 'perMonthTables',
		rosterResolution: 'databaseRowMappings',
		rosterRows: 3,
		dayColumns: 22
	};
}

/** A month the diagnostic could not measure. */
function unmeasured(
	month: string,
	status: MonthMarkComparison['sourceStatus']
): MonthMarkComparison {
	return {
		...unmeasuredMonthFor(month, status)
	};
}

function unmeasuredMonthFor(month: string, status: MonthMarkComparison['sourceStatus']) {
	return {
		reportMonth: month,
		reportYear: 2026,
		counts: {},
		cellsOnlyInWorkbook: [],
		cellsOnlyInDatabase: [],
		sourceStatus: status,
		reason: 'no worksheet for this month',
		mappingSource: 'none' as const,
		rosterResolution: 'unresolved' as const,
		rosterRows: 0,
		dayColumns: 0
	};
}

function schoolYear(): MonthMarkComparison[] {
	return [
		'JUNE',
		'JULY',
		'AUGUST',
		'SEPTEMBER',
		'OCTOBER',
		'NOVEMBER',
		'DECEMBER',
		'JANUARY',
		'FEBRUARY',
		'MARCH',
		'APRIL',
		'MAY'
	].map((month) => ({ ...measured(0, 0), reportMonth: month }));
}

// ── the verdict rules ─────────────────────────────────────────────────────

describe('the verdict rules', () => {
	it('lets a workbook mark the database lacks outrank every other answer', () => {
		const months = schoolYear();
		months[3] = measured(2, 0);

		expect(verdictFor(months)).toBe('WorkbookIsSourceOfTruth');
	});

	it('does not let an unreadable month hide a missing mark', () => {
		const months = schoolYear();
		months[3] = measured(1, 0);
		months[0] = unmeasured('JUNE', 'ExcelUnavailable');

		expect(verdictFor(months)).toBe('WorkbookIsSourceOfTruth');
	});

	it('calls the database the source of truth only when every month was read', () => {
		const months = schoolYear();
		months[5] = unmeasured('NOVEMBER', 'NoSheet');

		expect(verdictFor(months)).toBe('Incomparable');
	});

	it('does not warn about absences only the database has', () => {
		const months = schoolYear();
		months[7] = measured(0, 3);

		expect(verdictFor(months)).toBe('DatabaseIsSourceOfTruth');
	});

	it('calls a school year with no measurable month incomparable', () => {
		const months = schoolYear().map((month) => ({
			...month,
			sourceStatus: 'WorkbookMissing' as const
		}));

		expect(verdictFor(months)).toBe('Incomparable');
	});

	it('calls an empty school year incomparable rather than reassuring', () => {
		expect(verdictFor([])).toBe('Incomparable');
	});

	it('permits a write only for the database being authoritative', () => {
		expect(permitsWrite('DatabaseIsSourceOfTruth')).toBe(true);
		expect(permitsWrite('WorkbookIsSourceOfTruth')).toBe(false);
		expect(permitsWrite('Incomparable')).toBe(false);
	});

	it('names the number of missing marks in the reason for a stale verdict', () => {
		const months = schoolYear();
		months[3] = measured(7, 0);

		const reason = verdictReason(verdictFor(months), months, 0);

		expect(reason).toContain('7 X mark(s)');
	});
});

describe('verdictReason', () => {
	it('names the months it could not read', () => {
		const months = schoolYear();
		months[2] = unmeasured('AUGUST', 'NoSheet');
		months[5] = unmeasured('NOVEMBER', 'WorkbookMissing');

		const reason = verdictReason(verdictFor(months), months, 0);

		expect(reason).toContain('2 of 12 month(s)');
		expect(reason).toContain('AUGUST 2026 (NoSheet)');
		expect(reason).toContain('NOVEMBER 2026 (WorkbookMissing)');
		expect(reason).toContain('No conclusion is drawn');
	});

	it('reports the unplaceable cells too', () => {
		const months = schoolYear();

		expect(verdictReason('Incomparable', months, 36)).toContain('A further 36 X cell(s)');
	});

	it('says nothing was measured when every month was measured but the verdict is incomparable', () => {
		expect(verdictReason('Incomparable', schoolYear(), 0)).toBe('No month could be measured.');
	});
});

// ── the false-zero rule ───────────────────────────────────────────────────

describe('the false-zero rule', () => {
	it('reports no count at all for a month it could not measure', () => {
		for (const status of [
			'WorkbookMissing',
			'ExcelUnavailable',
			'NoSheet',
			'NoMappings'
		] as const) {
			const month: MonthMarkComparison = unmeasured('AUGUST', status);

			expect(month.counts.workbookXCount, status).toBeUndefined();
			expect(month.counts.dbAbsentCount, status).toBeUndefined();
			expect(month.counts.dbMappedAbsentCount, status).toBeUndefined();
			expect(month.counts.cellsScanned, status).toBeUndefined();
			expect(month.cellsOnlyInWorkbook, status).toHaveLength(0);
			expect(month.cellsOnlyInDatabase, status).toHaveLength(0);
		}
	});

	it('treats only Comparable as comparable', () => {
		expect(isComparable('Comparable')).toBe(true);
		expect(isComparable('NoSheet')).toBe(false);
		expect(isComparable('NoMappings')).toBe(false);
		expect(isComparable('ExcelUnavailable')).toBe(false);
		expect(isComparable('WorkbookMissing')).toBe(false);
	});

	it('does report a measured zero, which is a claim it actually made', () => {
		// The whole point: 0 and "not measured" are different answers and only a
		// measurement earns the first one.
		const months = schoolYear();
		months[3] = {
			...measured(0, 0),
			counts: { workbookXCount: 0, dbAbsentCount: 0, dbMappedAbsentCount: 0, cellsScanned: 66 }
		};

		expect(verdictFor(months)).toBe('DatabaseIsSourceOfTruth');
		expect(months[3]?.counts.workbookXCount).toBe(0);
	});
});
