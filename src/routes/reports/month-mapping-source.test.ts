/**
 * Where the grid's mappings come from, and what the grid does with them.
 *
 * ## Why this file exists
 *
 * The reported bug was two symptoms - no X marks in the Reports grid, and a grid
 * that could not be clicked - from one cause. Both come from the *mapping* half
 * of a grid row rather than from the marks: `cell.editable` and `row.mapped` are
 * derived from mappings, and a read that resolves mappings from tables which
 * happen to be empty produces a grid that renders every learner as unmapped.
 *
 * An unmapped row cannot show an X (the preview builder hard-codes those cells
 * to `present`) and cannot be clicked (`editable: false`). So the grid was not
 * merely empty - it was *asserting* that nobody was ever absent while the
 * database held absences, and refusing to let the teacher correct it.
 *
 * The fix has a half in the month read (it resolves mappings from the per-month
 * tables and falls back to the pre-split ones) and a half here (the grid must draw
 * what the read gives it, for whichever month is on screen). The read half is covered
 * in `src/lib/features/sf2/month/__tests__/`. This
 * file covers the half that is the grid's own, plus the one property that only
 * exists end to end: a cell is clickable exactly when the component's condition
 * says so.
 */

import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
	buildMatrixRows,
	buildMatrixWeekGroups,
	flattenMatrixSlots,
	monthGridToPreview
} from './report-state.svelte';
import type {
	Sf2MonthGridPreview,
	Sf2PreviewAbsence,
	Sf2PreviewDate,
	Sf2PreviewStudentRow
} from '$lib/types';

// ── The component's own click condition ─────────────────────────────────────

/**
 * `disabled={!cell.editable || !row.mapped || correctingCellKey !== null}` from
 * `report-table.svelte`, minus the transient `correctingCellKey` guard.
 *
 * Reproduced here rather than imported, because the component is a Svelte file
 * with no exported expression to import and the whole point is that this
 * condition is what the user is hitting. If the template's condition changes,
 * this needs to change with it - which is the deal.
 */
function cellIsDisabled(row: Sf2PreviewStudentRow, editable: boolean | undefined): boolean {
	return !editable || !row.mapped;
}

// ── Fixtures ────────────────────────────────────────────────────────────────

const CLASS_ID = 'class-1';
const SCHOOL_YEAR = '2026-2027';

type DaySpec = { day: number; mapped: boolean };

function pad(value: number): string {
	return String(value).padStart(2, '0');
}

function isWeekday(year: number, monthIndex: number, day: number): boolean {
	const weekday = new Date(Date.UTC(year, monthIndex, day)).getUTCDay();
	return weekday !== 0 && weekday !== 6;
}

/** Every Monday-Friday of a month, in date order, as the read returns them. */
function monthDates(year: number, monthIndex: number, specs: DaySpec[]): Sf2PreviewDate[] {
	const sheet = `${monthIndex === 8 ? 'SEPTEMBER' : 'OCTOBER'} ${year}`;
	const dates: Sf2PreviewDate[] = [];
	for (const spec of specs) {
		if (!isWeekday(year, monthIndex, spec.day)) continue;
		dates.push({
			date: `${year}-${pad(monthIndex + 1)}-${pad(spec.day)}`,
			sheetName: sheet,
			columnLetter: spec.mapped ? 'F' : '',
			columnIndex: spec.mapped ? 6 + dates.length : 0
		});
	}
	return dates;
}

type StudentSpec = { id: string; name: string; absentOn: string[] };

/**
 * Build the rows the preview builder produces.
 *
 * The shape is the load-bearing part, so it is written out rather than mocked: a
 * *mapped* row is one with a mapping, and every one of its cells is editable
 * exactly when the student record resolved. An *unmapped* row is the shape the
 * bug produced - `mapped: false` and cells that report `present` regardless of
 * what the database holds.
 */
function monthStudents(dates: Sf2PreviewDate[], specs: StudentSpec[]): Sf2PreviewStudentRow[] {
	return specs.map((spec) => ({
		studentId: spec.id,
		studentName: spec.name,
		workbookName: spec.name.toUpperCase(),
		gender: 'Male',
		rowIndex: 8 + specs.indexOf(spec),
		mapped: true,
		presentCount: 0,
		absentCount: spec.absentOn.length,
		warnings: [],
		cells: dates.map((date) => ({
			date: date.date,
			status: spec.absentOn.includes(date.date) ? ('absent' as const) : ('present' as const),
			editable: true
		}))
	}));
}

/** The row shape of a month whose mappings resolved to nothing. */
function unmappedStudents(dates: Sf2PreviewDate[], specs: StudentSpec[]): Sf2PreviewStudentRow[] {
	return specs.map((spec) => ({
		studentId: spec.id,
		studentName: spec.name,
		workbookName: '',
		gender: 'Male' as const,
		rowIndex: 0,
		mapped: false,
		presentCount: 0,
		absentCount: 0,
		warnings: ['Not linked to an SF2 workbook row.'],
		cells: dates.map((date) => ({
			date: date.date,
			// The builder's own answer for an unmapped row, and the reason the
			// grid showed no X marks: there is no mapping to say who was absent.
			status: 'present' as const,
			editable: false
		}))
	}));
}

function absentListFor(students: Sf2PreviewStudentRow[]): Sf2PreviewAbsence[] {
	return students.flatMap((row) =>
		row.cells
			.filter((cell) => cell.status === 'absent')
			.map((cell) => ({
				studentId: row.studentId,
				studentName: row.studentName,
				date: cell.date,
				rowIndex: row.rowIndex
			}))
	);
}

type GridOptions = {
	month: string;
	year: number;
	/** 0 = September, 9 = October. */
	monthIndex: number;
	dates: Sf2PreviewDate[];
	students: Sf2PreviewStudentRow[];
	usesLegacyMappings: boolean;
	hasTemplate: boolean;
};

function grid(options: GridOptions): Sf2MonthGridPreview {
	const absentList = absentListFor(options.students);
	const mappedDates = options.dates.filter((date) => date.columnLetter !== '').length;
	return {
		month: options.month,
		reportYear: options.year,
		schoolYear: SCHOOL_YEAR,
		classId: CLASS_ID,
		className: 'Grade 1 - A',
		sheetName: options.hasTemplate ? `${options.month} ${options.year}` : '',
		fileName: `SF2-${options.month}-${options.year}.xls`,
		fileExists: false,
		hasTemplate: options.hasTemplate,
		firstSchoolDay: 1,
		hasSchoolDays: true,
		gridEmpty: mappedDates === 0,
		usesLegacyMappings: options.usesLegacyMappings,
		workbookXCount: 0,
		workbookScannedAt: undefined,
		lastSyncedAt: undefined,
		template: {
			id: options.hasTemplate ? 'month-row' : 'legacy-row',
			sourcePath: 'C:/sf2-workbooks/SF2.xls',
			schoolId: 'S-1',
			schoolName: 'Espiritu Elementary',
			schoolYear: SCHOOL_YEAR,
			// The month on screen, never the pre-split row's own month. A header
			// claiming OCTOBER over September's days is the same lie as October's
			// columns under September's.
			reportMonth: options.month,
			gradeLevel: '1',
			section: 'A',
			adviserName: 'Dela Cruz',
			schoolHeadName: 'Santos',
			classId: CLASS_ID,
			importedAt: 1
		},
		dates: options.dates,
		students: options.students,
		absentList,
		mappedStudents: options.students.filter((row) => row.mapped).length,
		mappedDates,
		presentCount: 0,
		absenceCount: absentList.length,
		unmappedStudentCount: options.students.filter((row) => !row.mapped).length,
		issues: [],
		warnings: options.usesLegacyMappings
			? ['SEPTEMBER is drawn from the SF2 mappings recorded before per-month workbooks existed.']
			: []
	};
}

const JUAN: StudentSpec = { id: 'student-juan', name: 'Juan Dela Cruz', absentOn: [] };
const MARIA: StudentSpec = { id: 'student-maria', name: 'Maria Santos', absentOn: [] };

const SEPTEMBER_SPEC: DaySpec[] = [
	{ day: 1, mapped: true },
	{ day: 2, mapped: true },
	{ day: 3, mapped: true }
];
const OCTOBER_SPEC: DaySpec[] = [
	{ day: 1, mapped: true },
	{ day: 2, mapped: true },
	{ day: 5, mapped: true }
];

/** What the grid actually renders for one month read. */
function render(g: Sf2MonthGridPreview) {
	const preview = monthGridToPreview(g);
	const groups = buildMatrixWeekGroups(preview.dates, g.month, g.reportYear);
	const slots = flattenMatrixSlots(groups);
	const rows = buildMatrixRows(preview.students, slots, 'all');
	return { preview, groups, slots, rows };
}

// ── 1. A month served from the pre-split tables is clickable and shows its X ──

describe('a month whose mappings came from the pre-split tables', () => {
	const september = grid({
		month: 'SEPTEMBER',
		year: 2026,
		monthIndex: 8,
		dates: monthDates(2026, 8, SEPTEMBER_SPEC),
		students: monthStudents(monthDates(2026, 8, SEPTEMBER_SPEC), [
			{ ...JUAN, absentOn: ['2026-09-02'] },
			MARIA
		]),
		usesLegacyMappings: true,
		hasTemplate: false
	});

	it('reports where the grid is being drawn from', () => {
		expect(september.usesLegacyMappings).toBe(true);
		expect(september.hasTemplate).toBe(false);
		expect(september.warnings.join(' ')).toContain('before per-month workbooks existed');
	});

	it('shows the X the database holds', () => {
		const { preview, rows } = render(september);
		expect(preview.absenceCount).toBe(1);
		expect(preview.absentList).toEqual([
			{ studentId: 'student-juan', studentName: 'Juan Dela Cruz', date: '2026-09-02', rowIndex: 8 }
		]);
		// And the cell that carries it says so, rather than reading as present.
		const juan = rows.find((row) => row.studentId === 'student-juan');
		const absentCell = juan?.cellColumns.find((cell) => cell?.date === '2026-09-02');
		expect(absentCell?.status).toBe('absent');
	});

	it('leaves every cell clickable, which is the component condition', () => {
		const { rows } = render(september);
		expect(rows).toHaveLength(2);
		for (const row of rows) {
			expect(row.mapped).toBe(true);
			for (const cell of row.cellColumns) {
				if (!cell) continue;
				expect(cellIsDisabled(row, cell.editable)).toBe(false);
			}
		}
	});

	it('describes the workbook under the month on screen, not the pre-split row month', () => {
		const { preview } = render(september);
		expect(preview.template?.reportMonth).toBe('SEPTEMBER');
		// The identity is what keeps Open SF2 and Sync roster enabled for a month
		// whose marks are on screen, so a blank panel here would be a regression
		// of the very buttons a teacher needs.
		expect(preview.template?.schoolName).toBe('Espiritu Elementary');
	});
});

describe('the same month when its mappings resolve to nothing', () => {
	const dates = monthDates(2026, 8, SEPTEMBER_SPEC);
	const broken = grid({
		month: 'SEPTEMBER',
		year: 2026,
		monthIndex: 8,
		dates,
		students: unmappedStudents(dates, [{ ...JUAN, absentOn: ['2026-09-02'] }, MARIA]),
		usesLegacyMappings: false,
		hasTemplate: true
	});

	it('is exactly what the user reported: no X marks, nothing clickable', () => {
		// The regression fixture. It is here so the assertions above have something
		// to be different from - if this stops reproducing the bug, the tests
		// beside it have stopped testing anything.
		const { preview, rows } = render(broken);
		expect(preview.absenceCount).toBe(0);
		expect(preview.absentList).toEqual([]);
		for (const row of rows) {
			expect(row.mapped).toBe(false);
			for (const cell of row.cellColumns) {
				if (!cell) continue;
				expect(cellIsDisabled(row, cell.editable)).toBe(true);
			}
		}
	});
});

// ── 2. One file, twelve month sheets, each with its own marks (§0 A1) ───────

describe('two months of the same workbook', () => {
	const septemberDates = monthDates(2026, 8, SEPTEMBER_SPEC);
	const octoberDates = monthDates(2026, 9, OCTOBER_SPEC);
	const september = grid({
		month: 'SEPTEMBER',
		year: 2026,
		monthIndex: 8,
		dates: septemberDates,
		students: monthStudents(septemberDates, [{ ...JUAN, absentOn: ['2026-09-02'] }, MARIA]),
		usesLegacyMappings: true,
		hasTemplate: false
	});
	const october = grid({
		month: 'OCTOBER',
		year: 2026,
		monthIndex: 9,
		dates: octoberDates,
		students: monthStudents(octoberDates, [{ ...JUAN, absentOn: ['2026-10-05'] }, MARIA]),
		usesLegacyMappings: true,
		hasTemplate: true
	});

	it('render different marks, so the month filter is not ignored', () => {
		const septemberRendered = render(september);
		const octoberRendered = render(october);

		expect(septemberRendered.preview.absentList.map((absence) => absence.date)).toEqual([
			'2026-09-02'
		]);
		expect(octoberRendered.preview.absentList.map((absence) => absence.date)).toEqual([
			'2026-10-05'
		]);

		const septemberMarked = septemberRendered.rows
			.flatMap((row) => row.cellColumns)
			.filter((cell) => cell?.status === 'absent')
			.map((cell) => cell?.date);
		const octoberMarked = octoberRendered.rows
			.flatMap((row) => row.cellColumns)
			.filter((cell) => cell?.status === 'absent')
			.map((cell) => cell?.date);
		expect(septemberMarked).toEqual(['2026-09-02']);
		expect(octoberMarked).toEqual(['2026-10-05']);
	});

	it('lay out different days, so one month never wears the other month columns', () => {
		const septemberSlots = render(september)
			.slots.map((slot) => slot.dateKey)
			.filter((key): key is string => key !== null);
		const octoberSlots = render(october)
			.slots.map((slot) => slot.dateKey)
			.filter((key): key is string => key !== null);

		expect(septemberSlots.every((key) => key.startsWith('2026-09-'))).toBe(true);
		expect(octoberSlots.every((key) => key.startsWith('2026-10-'))).toBe(true);
		expect(septemberSlots).not.toEqual(octoberSlots);
	});
});

// ── 3. The unscoped pre-split read is not on the load or switch path ────────

describe('the legacy export preview is not a grid source any more', () => {
	const REPORTS_DIR = import.meta.dirname;
	const pageState = readFileSync(join(REPORTS_DIR, 'report-page-state.svelte.ts'), 'utf8');

	/**
	 * `get_sf2_export_preview` reads the pre-split tables with no month scoping
	 * of its own and derives the calendar from whichever month the pre-split row
	 * last held. Replacing a month read's grid with it is how a month came to be
	 * drawn with a different month's days: the read asked for SEPTEMBER, the
	 * answer was October's dates, and the header still said September.
	 *
	 * The month read now resolves the pre-split tables itself, month-scoped, so
	 * the page has one grid source and it is the scoped one. Asserted structurally
	 * because a second call site re-added later would be invisible to any
	 * behavioural test of the grid.
	 */
	it('is not called from the Reports page state', () => {
		expect(pageState).not.toMatch(/getSf2ExportPreview/);
		expect(pageState).not.toMatch(/get_sf2_export_preview/);
		expect(pageState).not.toMatch(/loadPreSplitFallback/);
	});

	it('is not called from the month cache or the open state machine', () => {
		for (const file of ['report-sf2-open.svelte.ts']) {
			const source = readFileSync(join(REPORTS_DIR, file), 'utf8');
			expect(source, file).not.toMatch(/getSf2ExportPreview/);
		}
	});
});
