import { describe, expect, it } from 'vitest';
import {
	absentCountInMonth,
	attendanceScopeCells,
	buildDateMappings,
	buildStudentMappings,
	databaseXCells,
	diffCells,
	label,
	monthFromMeasurement,
	workbookXCellsInScope,
	type GridCell,
	type RawMark
} from '../compare';
import type { AbsentRecord, RosterRow } from '../model';

/**
 * The comparison itself - the database-side join and the workbook-side scope - with no
 * workbook and no database in it.
 *
 * The verdict rules and the false-zero rule are in `rules.test.ts`; the assertions here
 * are the two that decide what a teacher is told: a cell the database cannot produce is
 * never counted as one it has, and a mark outside the scope the writer uses is never
 * counted as one it is missing.
 */

// ── fixtures ──────────────────────────────────────────────────────────────
// ── the database-side join ────────────────────────────────────────────────

describe('the database-side join', () => {
	function roster(): RosterRow[] {
		return [
			{ studentId: 's1', workbookName: 'Alvarado, Zyron Jay  E.', rowIndex: 8 },
			{ studentId: 's2', workbookName: 'BAPTISMA,SOSDFFIA', rowIndex: 9 }
		];
	}

	function dayGrid() {
		return buildDateMappings('t1', 'SEPTEMBER 2026', 2026, 9, [
			{ column: 8, day: 1 },
			{ column: 9, day: 2 },
			{ column: 10, day: 3 }
		]);
	}

	function absence(studentId: string, date: string): AbsentRecord {
		return { studentId, classId: 'c1', date };
	}

	it('dates a September day grid from its own columns', () => {
		const grid = dayGrid();

		expect(grid).toHaveLength(3);
		expect(grid[0]?.date).toBe('2026-09-01');
		expect(grid[0]?.columnLetter).toBe('H');
		expect(grid[2]?.date).toBe('2026-09-03');
		expect(grid[2]?.columnLetter).toBe('J');
	});

	it('drops a day number that is not a date rather than guessing', () => {
		// February 2026 has 28 days. A 31 in a February column is not a date, and
		// turning it into one would place a mark on the wrong day.
		const grid = buildDateMappings('t1', 'S', 2026, 2, [
			{ column: 8, day: 27 },
			{ column: 9, day: 28 },
			{ column: 10, day: 31 }
		]);

		expect(grid.map((mapping) => mapping.date)).toEqual(['2026-02-27', '2026-02-28']);
	});

	it('drops a day number of zero', () => {
		const grid = buildDateMappings('t1', 'S', 2026, 9, [
			{ column: 8, day: 0 },
			{ column: 9, day: 1 }
		]);

		expect(grid.map((mapping) => mapping.date)).toEqual(['2026-09-01']);
	});

	it('does not turn an absence on a day the month has no column for into a cell', () => {
		expect(databaseXCells(roster(), dayGrid(), [absence('s1', '2026-09-04')])).toHaveLength(0);
	});

	it('does not turn an absence for a learner with no roster row into a cell', () => {
		expect(databaseXCells(roster(), dayGrid(), [absence('unknown', '2026-09-01')])).toHaveLength(0);
	});

	it('turns an absence the grid can hold into a cell', () => {
		const cells = databaseXCells(roster(), dayGrid(), [absence('s2', '2026-09-02')]);

		expect(cells).toHaveLength(1);
		expect(`${cells[0]?.columnLetter}${cells[0]?.rowIndex}`).toBe('I9');
	});

	it('counts two absence rows for one learner and day as one cell', () => {
		const cells = databaseXCells(roster(), dayGrid(), [
			absence('s1', '2026-09-01'),
			absence('s1', '2026-09-01')
		]);

		expect(cells).toHaveLength(1);
	});

	it('keeps a month count from spilling into an adjacent month', () => {
		const absent = [
			absence('s1', '2026-08-31'),
			absence('s1', '2026-09-01'),
			absence('s1', '2026-10-01')
		];

		expect(absentCountInMonth(absent, 2026, 9)).toBe(1);
		expect(absentCountInMonth(absent, 2026, 8)).toBe(1);
		expect(absentCountInMonth(absent, 2026, 10)).toBe(1);
		expect(absentCountInMonth(absent, 2027, 9)).toBe(0);
	});

	it('carries the row the database recorded on a student mapping', () => {
		const mappings = buildStudentMappings('t1', roster());

		expect(mappings).toHaveLength(2);
		expect(mappings[0]?.rowIndex).toBe(8);
		expect(mappings[0]?.studentId).toBe('s1');
		expect(mappings[1]?.rowIndex).toBe(9);
	});

	it('names both sides of a difference rather than only counting it', () => {
		const rows = roster();
		const grid = dayGrid();
		const database = databaseXCells(rows, grid, [absence('s1', '2026-09-01')]);
		const workbook: GridCell[] = [
			{ sheetName: 'SEPTEMBER 2026', columnLetter: 'I', rowIndex: 8 },
			{ sheetName: 'SEPTEMBER 2026', columnLetter: 'H', rowIndex: 8 }
		];

		const { onlyInWorkbook, onlyInDatabase } = diffCells(rows, grid, workbook, database);

		expect(onlyInWorkbook).toHaveLength(1);
		expect(onlyInWorkbook[0]?.studentName).toBe('Alvarado, Zyron Jay  E.');
		expect(onlyInWorkbook[0]?.date).toBe('2026-09-02');
		expect(onlyInWorkbook[0]?.cellAddress).toBe('I8');
		expect(onlyInDatabase).toHaveLength(0);
	});

	it('says which row and column it could not place when a cell has no mapping', () => {
		const named = label({ sheetName: 'SEPTEMBER 2026', columnLetter: 'Q', rowIndex: 77 }, [], []);

		expect(named.studentName).toBe('learner row 77');
		expect(named.date).toBe('unmapped column Q');
		expect(named.cellAddress).toBe('Q77');
	});

	it('returns the same difference list in the same order every run', () => {
		const rows = roster();
		const grid = dayGrid();
		const database = databaseXCells(rows, grid, [absence('s1', '2026-09-01')]);
		// Two cells, deliberately out of the sort order on input.
		const workbook: GridCell[] = [
			{ sheetName: 'SEPTEMBER 2026', columnLetter: 'J', rowIndex: 9 },
			{ sheetName: 'SEPTEMBER 2026', columnLetter: 'I', rowIndex: 8 }
		];

		const first = diffCells(rows, grid, workbook, database);
		const second = diffCells(rows, grid, [...workbook].reverse(), database);

		expect(first.onlyInWorkbook.map((cell) => cell.cellAddress)).toEqual(['I8', 'J9']);
		expect(second.onlyInWorkbook.map((cell) => cell.cellAddress)).toEqual(['I8', 'J9']);
	});
});

// ── the workbook side ─────────────────────────────────────────────────────

describe('the workbook side', () => {
	const rows: RosterRow[] = [
		{ studentId: 's1', workbookName: 'Alvarado, Zyron Jay  E.', rowIndex: 8 },
		{ studentId: 's2', workbookName: 'BAPTISMA,SOSDFFIA', rowIndex: 9 }
	];
	const grid = buildDateMappings('t1', 'SEPTEMBER 2026', 2026, 9, [
		{ column: 8, day: 1 },
		{ column: 9, day: 2 },
		{ column: 10, day: 3 }
	]);
	const scope = attendanceScopeCells(buildStudentMappings('t1', rows), grid);

	function marks(cells: readonly [number, number][]): RawMark[] {
		return cells.map(([rowIndex, columnIndex]) => ({ rowIndex, columnIndex }));
	}

	it('counts a mark in scope', () => {
		const cells = workbookXCellsInScope(marks([[8, 8]]), 'SEPTEMBER 2026', scope);

		expect(cells).toHaveLength(1);
		expect(`${cells[0]?.columnLetter}${cells[0]?.rowIndex}`).toBe('H8');
	});

	it('keeps a mark on a day the month has no column for out of scope', () => {
		// Column M (13) is the second half of a merged day pair: no date behind it, so
		// no mapping covers it.
		const cells = workbookXCellsInScope(marks([[8, 13]]), 'SEPTEMBER 2026', scope);

		expect(cells).toHaveLength(0);
	});

	it('keeps a mark on a row the roster does not cover out of scope', () => {
		// Row 29 is the MALE TOTAL row on the DepEd form: a formula, not a mark.
		const cells = workbookXCellsInScope(marks([[29, 8]]), 'SEPTEMBER 2026', scope);

		expect(cells).toHaveLength(0);
	});

	it('counts the same cell once', () => {
		const cells = workbookXCellsInScope(
			marks([
				[8, 8],
				[8, 8]
			]),
			'SEPTEMBER 2026',
			scope
		);

		expect(cells).toHaveLength(1);
	});

	it('counts nothing rather than everything when the scope is empty', () => {
		const cells = workbookXCellsInScope(
			marks([
				[8, 8],
				[9, 8],
				[30, 8],
				[29, 8]
			]),
			'SEPTEMBER 2026',
			[]
		);

		expect(cells).toHaveLength(0);
	});

	it('builds a scope of every mapped row against every mapped day column', () => {
		expect(scope).toHaveLength(6);
		expect(new Set(scope.map((cell) => cell.rowIndex))).toEqual(new Set([8, 9]));
	});
});

// ── assembling a measured month ───────────────────────────────────────────

describe('monthFromMeasurement', () => {
	const rows: RosterRow[] = [
		{ studentId: 's1', workbookName: 'Alvarado, Zyron Jay  E.', rowIndex: 8 }
	];
	const grid = buildDateMappings('t1', 'SEPTEMBER 2026', 2026, 9, [{ column: 8, day: 1 }]);

	function assemble(workbookCells: GridCell[], databaseCells: GridCell[]) {
		return monthFromMeasurement(
			'SEPTEMBER',
			2026,
			'SEPTEMBER 2026',
			'/workbooks/book.xlsx',
			'perMonthTables',
			'databaseRowMappings',
			rows,
			grid,
			workbookCells,
			databaseCells,
			2,
			3
		);
	}

	it('says nothing is missing when both sides agree', () => {
		const cell: GridCell = { sheetName: 'SEPTEMBER 2026', columnLetter: 'H', rowIndex: 8 };

		const month = assemble([cell], [cell]);

		expect(month.sourceStatus).toBe('Comparable');
		expect(month.reason).toBe('The database holds every X the workbook shows, on the same cells.');
		expect(month.counts).toEqual({
			workbookXCount: 1,
			dbAbsentCount: 3,
			dbMappedAbsentCount: 1,
			cellsScanned: 2
		});
	});

	it('counts absences the grid cannot hold separately from the ones it can', () => {
		const cell: GridCell = { sheetName: 'SEPTEMBER 2026', columnLetter: 'H', rowIndex: 8 };

		const month = assemble([cell], [cell]);

		// Three absences this month, one of which the grid can actually hold.
		expect(month.counts.dbAbsentCount).toBe(3);
		expect(month.counts.dbMappedAbsentCount).toBe(1);
	});

	it('says how many marks the database lacks', () => {
		const month = assemble(
			[
				{ sheetName: 'SEPTEMBER 2026', columnLetter: 'H', rowIndex: 8 },
				{ sheetName: 'SEPTEMBER 2026', columnLetter: 'H', rowIndex: 8 }
			],
			[]
		);

		expect(month.reason).toBe("1 X mark(s) in this month's sheet have no record in the database.");
		expect(month.cellsOnlyInWorkbook).toHaveLength(1);
	});
});
