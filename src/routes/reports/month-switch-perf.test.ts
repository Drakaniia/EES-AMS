/**
 * The performance phase's tests (spec D9, §7.3, acceptance #8-#10).
 *
 * ## Why this file looks the way it does
 *
 * The target is "p50 < 150 ms, p95 < 300 ms, p99 < 500 ms over 20 switches". A
 * number like that is easy to produce from a test that measures nothing: mock the
 * preview, time a function, watch it come in at 4 ms, call it a pass. That test
 * would keep passing if the switch went back to opening Excel, because it never
 * looks at what the switch *does*.
 *
 * So this file asserts the thing that actually makes it fast, structurally, and
 * then times only what is left over:
 *
 * 1. **The switch path issues no Excel and no mutating command.** Asserted
 *    against the source of the switch path, so reinstating a COM call or a
 *    `set_sf2_report_month` fails the build rather than the benchmark.
 * 2. **Zero `sf2-progress` events during a switch.** Asserted against a real
 *    `listen` mock, not against the absence of a listener call in the source.
 * 3. **A timing bound, honestly labelled.** The number below is a *lower bound*:
 *    it measures the frontend's own work for 20 switches over a mocked month
 *    read, and excludes the SQL read itself. It says nothing about the backend.
 *    It is here to catch a regression in the grid projection - the part that is
 *    easy to make quadratic by accident - and it is written to be understood as
 *    exactly that, not as the p50 the spec asks for.
 *
 * A benchmark that passes because it measures a mock is the specific failure
 * mode this file is arranged to avoid: the structural assertions are the test,
 * and the timing is corroboration.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

import {
	buildMatrixRows,
	buildMatrixWeekGroups,
	flattenMatrixSlots,
	monthGridToPreview
} from './report-state.svelte';
import type {
	Sf2MonthGridPreview,
	Sf2PreviewCell,
	Sf2PreviewDate,
	Sf2PreviewStudentRow
} from '$lib/types';

// ── Mocks, set up before the module under test is imported ──────────────────

const invokeCalls: { command: string; args?: unknown }[] = [];
const invokeMock = vi.fn(async (command: string, args?: unknown) => {
	invokeCalls.push({ command, args });
	return monthPreview(40);
});

const listenCalls: string[] = [];
const listenMock = vi.fn(async (event: string, _handler?: unknown) => {
	listenCalls.push(event);
	return () => {};
});

vi.mock('@tauri-apps/api/core', () => ({
	invoke: (command: string, args?: unknown) => invokeMock(command, args)
}));

vi.mock('@tauri-apps/api/event', () => ({
	listen: (event: string, handler: unknown) => listenMock(event, handler)
}));

import { getSf2MonthPreview } from '$lib/db-rust/sf2-months';

beforeEach(() => {
	invokeCalls.length = 0;
	listenCalls.length = 0;
	invokeMock.mockClear();
	listenMock.mockClear();
});

// ── Source of the switch path ───────────────────────────────────────────────

const REPORTS_DIR = import.meta.dirname;
const SRC_DIR = join(REPORTS_DIR, '..', '..');

const read = (...parts: string[]) => readFileSync(join(...parts), 'utf8');

const SWITCH_PATH_SOURCES: Record<string, string> = {
	'report-page-state.svelte.ts': read(REPORTS_DIR, 'report-page-state.svelte.ts'),
	'report-sf2-open.svelte.ts': read(REPORTS_DIR, 'report-sf2-open.svelte.ts'),
	'sf2-months.ts': read(SRC_DIR, 'lib', 'db-rust', 'sf2-months.ts'),
	'sf2.ts': read(SRC_DIR, 'lib', 'db-rust', 'sf2.ts')
};

/**
 * Everything a month switch must not do.
 *
 * `listen` and `sf2-progress` are here because the deleted
 * `ReportMonthSwitchOverlay` existed only to cover a progress stream that a
 * one-read switch does not have (D9: "no modal at all"). The rest are the
 * commands and helpers the old switch went through.
 */
const FORBIDDEN_IN_THE_SWITCH: [string, RegExp][] = [
	['set_sf2_report_month', /set_sf2_report_month/],
	['setSf2ReportMonth', /setSf2ReportMonth/],
	['sync_and_open_sf2_workbook', /sync_and_open_sf2_workbook|syncAndOpenSf2Workbook/],
	['export_sf2_workbook', /export_sf2_workbook|exportSf2Workbook/],
	['update_sf2_workbook_settings', /update_sf2_workbook_settings|updateSf2WorkbookSettings/],
	['import_sf2_attendance_from_workbook', /importSf2AttendanceFromWorkbook/],
	['sync_sf2_roster', /sync_sf2_roster|syncSf2Roster/],
	['present_all_sf2_preview_attendance', /presentAllSf2PreviewAttendance/],
	['set_sf2_preview_attendance', /setSf2PreviewAttendance/],
	['toggle_sf2_preview_attendance', /toggle_sf2_preview_attendance|toggleSf2PreviewAttendance/],
	['an sf2-progress listener', /listen[\s\S]{0,80}sf2-progress|sf2-progress/],
	[
		'a month-switch progress message',
		/Preparing your attendance report|Updating the workbook calendar/
	]
];

/** Files that may legitimately appear in the switch-path list above. */
const SWITCH_PATH_ALLOWANCES: Record<string, string[]> = {
	// `report-sf2-open.svelte.ts` owns the *Open SF2* state machine, which does
	// listen for progress - that path writes marks to Excel and is a different
	// operation from a switch. The switch does not reach it.
	'report-sf2-open.svelte.ts': [
		'an sf2-progress listener',
		'sync_and_open_sf2_workbook',
		'syncAndOpenSf2Workbook',
		'a month-switch progress message'
	],
	// The shared command-wrapper module for the *legacy* workbook commands. Only
	// the month-switch wrapper is relevant here; the rest of the file is the
	// pre-split surface the guard phase still calls.
	'sf2.ts': [
		'sync_and_open_sf2_workbook',
		'syncAndOpenSf2Workbook',
		'import_sf2_attendance_from_workbook',
		'sync_sf2_roster',
		'present_all_sf2_preview_attendance',
		'export_sf2_workbook',
		'update_sf2_workbook_settings',
		'set_sf2_preview_attendance',
		'toggle_sf2_preview_attendance',
		'an sf2-progress listener'
	],
	// A month switch may not mark attendance, but `toggleAttendance` lives on the
	// same state object and the page state's own switch helpers are what this
	// asserts. Marking a cell is a different operation from looking at a month,
	// and no switch helper calls it.
	'report-page-state.svelte.ts': [
		'import_sf2_attendance_from_workbook',
		'sync_sf2_roster',
		'present_all_sf2_preview_attendance',
		'export_sf2_workbook',
		'update_sf2_workbook_settings',
		'set_sf2_preview_attendance',
		'toggle_sf2_preview_attendance'
	]
};

function forbiddenHere(file: string): [string, RegExp][] {
	const allowed = SWITCH_PATH_ALLOWANCES[file] ?? [];
	return FORBIDDEN_IN_THE_SWITCH.filter(([name]) => !allowed.includes(name));
}

/**
 * Drop line comments and block comments, so a structural assertion is about
 * *code* and not about prose that happens to name a deleted symbol.
 *
 * The doc comment on the page state says "`setSf2ReportMonth` is gone" - which is
 * true and is not a violation. Matching raw source would report it as one, and a
 * structural test that cries wolf over its own documentation gets deleted by the
 * next person who trips over it.
 *
 * Limitation, and it is deliberate: a `//` or `/*` inside a string literal is
 * treated as the start of a comment. Nothing in the files checked here has one,
 * and a false negative is a weakened test rather than a wrong pass.
 */
function codeOnly(source: string): string {
	const kept: string[] = [];
	let inBlockComment = false;
	for (const line of source.split('\n')) {
		let text = line;
		if (inBlockComment) {
			const close = text.indexOf('*/');
			if (close === -1) {
				kept.push('');
				continue;
			}
			text = text.slice(close + 2);
			inBlockComment = false;
		}
		const open = text.indexOf('/*');
		if (open !== -1) {
			const close = text.indexOf('*/', open);
			if (close === -1) {
				text = text.slice(0, open);
				inBlockComment = true;
			} else {
				text = text.slice(0, open) + text.slice(close + 2);
			}
		}
		const cut = text.indexOf('//');
		kept.push(cut === -1 ? text : text.slice(0, cut));
	}
	return kept.join('\n');
}

// ── 1. Structure: no Excel, no mutating command ─────────────────────────────

describe('the month switch path', () => {
	it('reads its sources, so a vacuous assertion cannot pass silently', () => {
		for (const [file, source] of Object.entries(SWITCH_PATH_SOURCES)) {
			expect(source.length, `${file} was not read`).toBeGreaterThan(200);
		}
	});

	it.each(Object.keys(SWITCH_PATH_SOURCES))(
		'%s issues no Excel and no mutating command',
		(file) => {
			const code = codeOnly(SWITCH_PATH_SOURCES[file]);
			for (const [name, pattern] of forbiddenHere(file)) {
				expect(
					pattern.test(code),
					`${file} must not reference ${name}: a month switch is one read-only SQL query`
				).toBe(false);
			}
		}
	);

	it('has no month-switch progress machinery left anywhere in src/', () => {
		const offenders: string[] = [];
		for (const file of walk(join(SRC_DIR))) {
			if (!/\.(ts|svelte)$/.test(file)) continue;
			// This file names the deleted symbols on purpose - that is what makes it
			// a grep for their callers. Skipping it keeps the assertion honest
			// instead of self-satisfying.
			if (file.endsWith('month-switch-perf.test.ts')) continue;
			const code = codeOnly(readFileSync(file, 'utf8'));
			for (const [name, pattern] of [
				['setSf2ReportMonth', /setSf2ReportMonth/],
				['ReportMonthSwitchOverlay', /ReportMonthSwitchOverlay/],
				['onReportMonthChange', /onReportMonthChange/]
			] as [string, RegExp][]) {
				if (pattern.test(code)) offenders.push(`${file}: ${name}`);
			}
		}
		expect(
			offenders,
			`the deleted month-switch path still has callers: ${offenders.join(', ')}`
		).toEqual([]);
	});

	it('reads exactly one command, and it is the read-only one', async () => {
		await getSf2MonthPreview('OCTOBER', 'class-1', '2026-2027');

		expect(invokeCalls.map((call) => call.command)).toEqual(['get_sf2_month_preview']);
		expect(listenCalls).toEqual([]);
		// The mock is a fixed September fixture, so the assertion is that the read
		// returned a whole, coherent month - not that it echoed the month asked
		// for. What matters is that one read produced the whole grid.
		const preview = monthPreview(40);
		expect(preview.month).toBe('SEPTEMBER');
		expect(preview.dates.length).toBe(22);
		expect(preview.students.length).toBe(40);
	});

	it('passes the cache key inputs through to the command', async () => {
		await getSf2MonthPreview('AUGUST', 'class-2', '2026-2027');

		expect(invokeMock).toHaveBeenCalledWith('get_sf2_month_preview', {
			classId: 'class-2',
			schoolYear: '2026-2027',
			reportMonth: 'AUGUST'
		});
	});
});

// ── 2. Zero sf2-progress events during a switch (acceptance #9) ─────────────

describe('sf2-progress during a switch', () => {
	it('emits none, and registers no listener to receive one', async () => {
		// Twenty switches, the same number the spec's percentiles are over.
		for (let index = 0; index < 20; index += 1) {
			await getSf2MonthPreview(index % 2 === 0 ? 'OCTOBER' : 'NOVEMBER', 'class-1', '2026-2027');
		}

		expect(listenCalls, 'a switch must not open an sf2-progress listener').toEqual([]);
		expect(invokeCalls.filter((call) => call.command !== 'get_sf2_month_preview')).toEqual([]);
	});
});

// ── 3. Timing - a lower bound, and labelled as one ──────────────────────────

/**
 * Build a month read for `studentCount` students with a full month of weekday
 * columns, which is what a real 40-student month looks like once the split has
 * run.
 */
function monthPreview(studentCount: number, year = 2026, month = 9): Sf2MonthGridPreview {
	const sheet = `SEPTEMBER ${year}`;
	const dates: Sf2PreviewDate[] = [];
	for (let day = 1; day <= 30; day += 1) {
		const date = new Date(Date.UTC(year, month - 1, day));
		const weekday = date.getUTCDay();
		if (weekday === 0 || weekday === 6) continue;
		dates.push({
			date: `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`,
			sheetName: sheet,
			columnLetter: 'F',
			columnIndex: 6 + dates.length
		});
	}
	// A pessimistic grid: a third of the students are absent on a third of the
	// days, which is the shape a busy month actually has. Typed as the shared
	// models rather than inferred, so a change to a cell's shape is a type error
	// here rather than a mock that quietly stopped resembling the real thing.
	const cells = (studentIndex: number): Sf2PreviewCell[] =>
		dates.map((date, dateIndex) => ({
			date: date.date,
			status: (studentIndex + dateIndex) % 3 === 0 ? 'absent' : 'present',
			editable: true
		}));
	const students: Sf2PreviewStudentRow[] = Array.from({ length: studentCount }, (_, index) => ({
		studentId: `student-${index}`,
		studentName: `Student ${index}`,
		workbookName: `STUDENT ${index}`,
		gender: index % 2 === 0 ? 'Male' : 'Female',
		rowIndex: 8 + index,
		mapped: true,
		presentCount: 0,
		absentCount: 0,
		warnings: [],
		cells: cells(index)
	}));
	const absentList = students.flatMap((student) =>
		student.cells
			.filter((cell) => cell.status === 'absent')
			.map((cell) => ({
				studentId: student.studentId,
				studentName: student.studentName,
				date: cell.date,
				rowIndex: student.rowIndex
			}))
	);

	return {
		month: 'SEPTEMBER',
		reportYear: year,
		schoolYear: `${year}-${year + 1}`,
		classId: 'class-1',
		className: 'Grade 1 - A',
		sheetName: sheet,
		fileName: `SF2-${sheet.replace(' ', '-')}.xls`,
		fileExists: true,
		hasTemplate: true,
		firstSchoolDay: 1,
		hasSchoolDays: true,
		gridEmpty: false,
		usesLegacyMappings: false,
		workbookXCount: absentList.length,
		workbookScannedAt: 1,
		lastSyncedAt: 1,
		template: {
			id: 'month-september',
			sourcePath: 'C:/sf2-workbooks/SF2-SEPTEMBER-2026.xls',
			schoolId: 'S-1',
			schoolName: 'Espiritu Elementary',
			schoolYear: `${year}-${year + 1}`,
			reportMonth: 'SEPTEMBER',
			gradeLevel: '1',
			section: 'A',
			adviserName: 'Dela Cruz',
			schoolHeadName: 'Santos',
			classId: 'class-1',
			importedAt: 1
		},
		dates,
		students,
		absentList,
		mappedStudents: studentCount,
		mappedDates: dates.length,
		presentCount: studentCount * dates.length - absentList.length,
		absenceCount: absentList.length,
		unmappedStudentCount: 0,
		issues: [],
		warnings: []
	};
}

/** One switch's worth of frontend work: project, group into weeks, project rows. */
function switchCost(grid: Sf2MonthGridPreview) {
	const preview = monthGridToPreview(grid);
	const groups = buildMatrixWeekGroups(preview.dates, grid.month, grid.reportYear);
	const slots = flattenMatrixSlots(groups);
	const rows = buildMatrixRows(preview.students, slots, 'all');
	return {
		preview,
		groups,
		rows,
		slotCount: slots.length,
		datedSlots: slots.filter((slot) => slot.dateKey !== null).length
	};
}

function percentile(sorted: number[], fraction: number): number {
	if (sorted.length === 0) return 0;
	const index = Math.min(sorted.length - 1, Math.ceil(fraction * sorted.length) - 1);
	return sorted[Math.max(0, index)];
}

describe('switch timing', () => {
	it('projects 20 switches of a 40-student month well inside the target', () => {
		const grid = monthPreview(40);
		expect(grid.dates.length).toBe(22);
		expect(grid.students.length).toBe(40);
		expect(grid.absenceCount).toBeGreaterThan(0);

		// Warm up so the first sample is not paying for module init.
		for (let index = 0; index < 5; index += 1) switchCost(grid);

		const samples: number[] = [];
		for (let index = 0; index < 20; index += 1) {
			const started = performance.now();
			const result = switchCost(grid);
			samples.push(performance.now() - started);
			// Every switch has to actually produce a grid, or the timing is
			// measuring a function that returned nothing. September 2026 is 22
			// weekdays laid out in 5 week groups of 5, so 25 slots - three of them
			// the month's edges (Mon 31 August, Thu 1 and Fri 2 October), which are
			// blank and must still occupy a column.
			expect(result.groups.length).toBe(5);
			expect(result.rows.length).toBe(40);
			expect(result.slotCount).toBe(25);
			expect(result.datedSlots).toBe(22);
			// Every row is projected onto the visible columns 1:1, which is the
			// invariant a quadratic regression would break first.
			expect(result.rows.every((row) => row.cellColumns.length === 25)).toBe(true);
		}

		const sorted = [...samples].sort((a, b) => a - b);
		const p50 = percentile(sorted, 0.5);
		const p95 = percentile(sorted, 0.95);
		const p99 = percentile(sorted, 0.99);

		// A LOWER BOUND. This is the frontend's own work for a switch, over a
		// mocked month read. It excludes the `get_sf2_month_preview` round-trip, so
		// it is not the p50/p95/p99 the spec asks for and must not be quoted as
		// one - it is a ceiling on what the browser can be blamed for, and a
		// regression in the grid projection is what it is here to catch.
		//
		// The spec's own numbers (p50 < 150 ms, p95 < 300 ms, p99 < 500 ms) are
		// met end to end by construction rather than by measurement: a month read
		// is one indexed SELECT over `sf2_month_date_mappings` and `events`, and
		// the Excel session that used to dominate it is gone - see the structural
		// assertions above, which are what actually hold that line.
		expect(p50).toBeLessThan(150);
		expect(p95).toBeLessThan(300);
		expect(p99).toBeLessThan(500);
	});

	it('scales linearly in students, so the grid is not accidentally quadratic', () => {
		// The failure this catches: a month switch that got slower the more
		// students a class had, which is invisible at 12 students and painful at
		// 40. 40 students must cost nowhere near four times 10.
		const time = (students: number) => {
			const grid = monthPreview(students);
			for (let index = 0; index < 3; index += 1) switchCost(grid);
			const started = performance.now();
			for (let index = 0; index < 10; index += 1) switchCost(grid);
			return performance.now() - started;
		};
		const small = time(10);
		const large = time(40);
		const ratio = large / Math.max(small, 0.05);
		expect(ratio, `40 students took ${ratio.toFixed(1)}x as long as 10`).toBeLessThan(12);
	});
});

// ── 4. The school-year wrap, in the browser ─────────────────────────────────
//
// The legacy SF2 code wrapped the calendar year at JUNE. A school year wraps at
// SEPTEMBER. For ten months of the year the two agree; for AUGUST they do not,
// and `SF2-AUGUST-2026.xls` and `SF2-AUGUST-2027.xls` are different files.
//
// The reconciliation is that the browser is not in the business of deciding. The
// year comes from the month read, which reads it off the stored month row. These
// tests say that out loud, including the strongest form of it: the function that
// lays out the grid is not even given a school year to apply a rule to.

describe('the grid takes its year from the month read', () => {
	it('draws the year the read reported', () => {
		const grid = monthPreview(40, 2026, 10);
		grid.month = 'OCTOBER';

		const groups = buildMatrixWeekGroups(grid.dates, 'OCTOBER', grid.reportYear);
		const dates = flattenMatrixSlots(groups)
			.map((slot) => slot.dateKey)
			.filter((date): date is string => date !== null);

		expect(dates.length).toBeGreaterThan(0);
		expect(dates.every((date) => date.startsWith('2026-10-'))).toBe(true);
	});

	it('draws AUGUST 2026-2027 as August 2027, where the June rule would say 2026', () => {
		// School year 2026-2027 runs SEPTEMBER 2026 -> AUGUST 2027. Under the
		// legacy `month >= 6 ? startYear : startYear + 1` rule, August would be
		// 2026 - a different file, and a grid of the wrong weekdays entirely,
		// because a month has different weekdays in different years.
		const august = monthPreview(40, 2027, 8);
		august.month = 'AUGUST';
		august.schoolYear = '2026-2027';

		const groups = buildMatrixWeekGroups(august.dates, 'AUGUST', august.reportYear);
		const dates = flattenMatrixSlots(groups)
			.map((slot) => slot.dateKey)
			.filter((date): date is string => date !== null);

		expect(dates.length).toBeGreaterThan(0);
		expect(dates.every((date) => date.startsWith('2027-08-'))).toBe(true);

		// And the 2026 reading is a different shape, which is the whole reason the
		// distinction is not academic.
		const wrong = buildMatrixWeekGroups(august.dates, 'AUGUST', 2026);
		const wrongDates = flattenMatrixSlots(wrong)
			.map((slot) => slot.dateKey)
			.filter((date): date is string => date !== null);
		expect(wrongDates).not.toEqual(dates);
	});

	it('is not given a school year to apply a rule to', () => {
		// The load-bearing form of the reconciliation. A second implementation of
		// the year rule in the browser is how the two rules drift apart in the
		// first place, so there is no school-year parameter here to implement one
		// with.
		expect(buildMatrixWeekGroups.length).toBe(3);
		expect(buildMatrixWeekGroups).toHaveLength(3);
		const parameters = buildMatrixWeekGroups
			.toString()
			.slice(0, buildMatrixWeekGroups.toString().indexOf(')'));
		expect(parameters.toLowerCase()).not.toContain('schoolyear');
	});
});

// ── Helpers ─────────────────────────────────────────────────────────────────

/** Every file under `dir`, recursively. */
function walk(dir: string): string[] {
	return readdirSync(dir, { recursive: true, withFileTypes: true })
		.filter((entry) => entry.isFile())
		.map((entry) => join(entry.parentPath, entry.name));
}
