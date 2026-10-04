/**
 * The attendance cell arithmetic: the clock rules, the write scope, the differential
 * clear, the marks a write produces, and the reader that reads them back.
 *
 * Two halves in one file because they are one rule seen from opposite sides: an `X`
 * written into a workbook and an `X` read back out of it have to be the same `X`, or
 * the round trip silently loses a term of marks. The workbook half is in
 * `attendance-write.test.ts` and the four commands in `attendance-service.test.ts`.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { date, db, mapping, seedClass, SHEET, useAttendanceTestDb } from './attendance-fixture';
import { MemoryFileSystem, useFileSystem } from '$lib/platform/fs';
import { loadTemplate } from '$lib/features/excel/__tests__/template-fixture';
import type { Worksheet } from 'exceljs';
import { cellText } from '$lib/features/excel/workbook';
import { SF2_ABSENT_MARK } from '$lib/features/sf2/logic';
import {
	attendanceScopeCells,
	differentialClearMarks,
	gridCellFromMark,
	gridCellKey,
	mappedAttendanceRows
} from '../attendance/attendance-marks';
import type { Sf2GridCell } from '../attendance/attendance-marks';
import {
	absentStudentIds,
	attendanceTimestampForDate,
	localDayBoundsTimestampsForDate,
	parseClock,
	presentStudentIds,
	setAttendanceEventForDay,
	SF2_PREVIEW_CORRECTION
} from '../attendance/attendance-events';
import {
	emptyImportOutcome,
	gridCellsToScan,
	importAbsentMarks,
	isAbsentMark
} from '../attendance/attendance-import';
import { exportAttendanceMarks, writeAttendanceToWorkbook } from '../attendance/attendance-write';

useAttendanceTestDb();

afterEach(() => {
	useFileSystem(null);
});

/** A local instant, because every date rule in the SF2 grid is a local one. */
const at = (year: number, month: number, day: number, hour = 9): string =>
	new Date(year, month - 1, day, hour, 0, 0).toISOString();

// ── parseClock ────────────────────────────────────────────────────────────────

describe('parseClock', () => {
	it('reads a normal, afternoon and midnight time', () => {
		expect(parseClock('08:30')).toEqual({ hour: 8, minute: 30 });
		expect(parseClock('14:00')).toEqual({ hour: 14, minute: 0 });
		expect(parseClock('00:00')).toEqual({ hour: 0, minute: 0 });
		expect(parseClock('23:59')).toEqual({ hour: 23, minute: 59 });
	});

	it('tolerates surrounding whitespace and a single-digit hour', () => {
		expect(parseClock('  08:30')).toEqual({ hour: 8, minute: 30 });
		expect(parseClock('08:30  ')).toEqual({ hour: 8, minute: 30 });
		expect(parseClock('8:30')).toEqual({ hour: 8, minute: 30 });
	});

	it('refuses an out-of-range hour or minute rather than clamping it', () => {
		// Clamping 24:00 to 00:00 would silently file the mark under the next day.
		expect(parseClock('24:00')).toBeUndefined();
		expect(parseClock('08:60')).toBeUndefined();
		expect(parseClock('08:99')).toBeUndefined();
	});

	it('refuses text that is not a clock at all', () => {
		expect(parseClock('')).toBeUndefined();
		expect(parseClock('abc')).toBeUndefined();
		expect(parseClock('0830')).toBeUndefined();
		expect(parseClock('ab:30')).toBeUndefined();
		expect(parseClock('08:xy')).toBeUndefined();
	});
});

describe('localDayBoundsTimestampsForDate', () => {
	it('spans exactly one local day, from local midnight to the next', () => {
		const { start, end } = localDayBoundsTimestampsForDate('2026-09-01');
		expect(new Date(start * 1000).getHours()).toBe(0);
		expect(new Date(start * 1000).getDate()).toBe(1);
		expect(new Date(end * 1000).getDate()).toBe(2);
	});

	it('stamps a mark at the class day_start, falling back to 08:00', () => {
		expect(attendanceTimestampForDate('2026-09-01', '07:30')).toBe(
			Math.floor(new Date(2026, 8, 1, 7, 30, 0).getTime() / 1000)
		);
		expect(attendanceTimestampForDate('2026-09-01', 'nonsense')).toBe(
			Math.floor(new Date(2026, 8, 1, 8, 0, 0).getTime() / 1000)
		);
	});
});

// ── The mapped rows and the write scope ───────────────────────────────────────

describe('mappedAttendanceRows', () => {
	it('is empty for nothing, and for only placeholders', () => {
		expect(mappedAttendanceRows([])).toEqual([]);
		expect(mappedAttendanceRows([0, 0])).toEqual([]);
	});

	it('sorts, de-duplicates and drops the row-0 placeholder', () => {
		expect(mappedAttendanceRows([30, 10, 20])).toEqual([10, 20, 30]);
		expect(mappedAttendanceRows([5, 5, 10, 10])).toEqual([5, 10]);
		expect(mappedAttendanceRows([0, 5, 0, 10])).toEqual([5, 10]);
		expect(mappedAttendanceRows([10])).toEqual([10]);
	});
});

describe('attendanceScopeCells', () => {
	const roster = [mapping('s1', 8), mapping('s2', 30), mapping('s3', 0)];
	const dates = [date('01', 'F', 6), date('02', 'H', 8)];

	it('is every mapped learner row × every mapped day column', () => {
		expect(attendanceScopeCells(roster, dates)).toEqual([
			{ sheetName: SHEET, columnLetter: 'F', rowIndex: 8 },
			{ sheetName: SHEET, columnLetter: 'F', rowIndex: 30 },
			{ sheetName: SHEET, columnLetter: 'H', rowIndex: 8 },
			{ sheetName: SHEET, columnLetter: 'H', rowIndex: 30 }
		]);
	});

	it('is empty for a month with no mappings, rather than the whole grid', () => {
		expect(attendanceScopeCells([], dates)).toEqual([]);
		expect(attendanceScopeCells(roster, [])).toEqual([]);
	});
});

describe('differentialClearMarks', () => {
	const scope: Sf2GridCell[] = [
		{ sheetName: SHEET, columnLetter: 'F', rowIndex: 8 },
		{ sheetName: SHEET, columnLetter: 'H', rowIndex: 8 }
	];

	it('blanks an X the database cannot produce', () => {
		expect(differentialClearMarks(scope, [], [scope[0]])).toEqual([
			{ sheetName: SHEET, address: 'F8', value: '' }
		]);
	});

	it('never touches an X the database still proves', () => {
		expect(differentialClearMarks(scope, [scope[0]], [scope[0]])).toEqual([]);
	});

	it('never touches a cell outside the month’s mapped rows and columns', () => {
		expect(
			differentialClearMarks(scope, [], [{ sheetName: SHEET, columnLetter: 'T', rowIndex: 8 }])
		).toEqual([]);
	});

	it('never writes a cell that is not already an X', () => {
		expect(differentialClearMarks(scope, [], [])).toEqual([]);
	});

	it('cannot be the whole grid: an empty everything clears nothing', () => {
		expect(differentialClearMarks([], [], scope)).toEqual([]);
	});

	it('is deterministic, and writes the cleared cells in address order', () => {
		const held = [scope[1], scope[0]];
		const once = differentialClearMarks(scope, [], held);
		expect(differentialClearMarks(scope, [], held)).toEqual(once);
		expect(once.map((mark) => mark.address)).toEqual(['F8', 'H8']);
	});
});

describe('gridCellFromMark / gridCellKey', () => {
	it('recovers a cell from a generated mark', () => {
		expect(gridCellFromMark({ sheetName: SHEET, address: 'AL47', value: '' })).toEqual({
			sheetName: SHEET,
			columnLetter: 'AL',
			rowIndex: 47
		});
	});

	it('answers undefined for anything that does not end in a row number', () => {
		expect(gridCellFromMark({ sheetName: SHEET, address: 'TOTAL', value: '' })).toBeUndefined();
		expect(gridCellFromMark({ sheetName: SHEET, address: '', value: '' })).toBeUndefined();
	});

	it('keys a cell by worksheet and address, so two sheets never collide', () => {
		expect(gridCellKey({ sheetName: 'A', columnLetter: 'F', rowIndex: 8 })).toBe('A!F8');
	});
});

// ── The event write ───────────────────────────────────────────────────────────

describe('setAttendanceEventForDay', () => {
	it('records one absence, and the audit row names the flow that wrote it', async () => {
		const { classId, firstId } = await seedClass();
		await setAttendanceEventForDay({
			studentId: firstId,
			classId,
			date: '2026-09-01',
			dayStart: '07:30',
			eventType: 'absent',
			reason: SF2_PREVIEW_CORRECTION
		});

		expect(await db().query('SELECT id FROM events')).toHaveLength(1);
		// Scoped to the event: seeding the class and the learner audits those too.
		const audit = await db().query<{ summary: string; metadata_json: string }>(
			"SELECT summary, metadata_json FROM audit_events WHERE entity_type = 'attendance_event'"
		);
		expect(audit).toHaveLength(1);
		expect(audit[0].summary).toContain(SF2_PREVIEW_CORRECTION);
		expect(JSON.parse(audit[0].metadata_json)).toMatchObject({ date: '2026-09-01' });
	});

	it('stamps the mark at the class day_start, on the day the grid shows', async () => {
		const { classId, firstId } = await seedClass();
		await setAttendanceEventForDay({
			studentId: firstId,
			classId,
			date: '2026-09-01',
			dayStart: '07:30',
			eventType: 'absent',
			reason: 'test'
		});
		const rows = await db().query<{ timestamp: number }>('SELECT timestamp FROM events');
		const stamped = new Date(Number(rows[0].timestamp) * 1000);
		expect([stamped.getFullYear(), stamped.getMonth(), stamped.getDate()]).toEqual([2026, 8, 1]);
		expect(stamped.getHours()).toBe(7);
	});
});

// ── The day selectors ─────────────────────────────────────────────────────────

describe('absentStudentIds / presentStudentIds', () => {
	const students = [
		{ id: 's1', name: 'Juan', createdAt: '' },
		{ id: 's2', name: 'Maria', createdAt: '' }
	];
	const events = [
		{ id: 'e1', studentId: 's1', classId: 'c', type: 'absent' as const, timestamp: at(2026, 9, 1) },
		{ id: 'e2', studentId: 's2', classId: 'c', type: 'in' as const, timestamp: at(2026, 9, 1) },
		{ id: 'e3', studentId: 's1', type: 'absent' as const, timestamp: at(2026, 9, 1) }
	];

	it('selects by type, class membership and the local day', () => {
		expect([...absentStudentIds(events, students, 'c', '2026-09-01')]).toEqual(['s1']);
		expect([...presentStudentIds(events, students, 'c', '2026-09-01')]).toEqual(['s2']);
		expect(absentStudentIds(events, students, 'c', '2026-09-02').size).toBe(0);
	});

	it('counts an event with no class when the learner is on the roster', () => {
		const loose = [
			{ id: 'e9', studentId: 's2', type: 'absent' as const, timestamp: at(2026, 9, 1) }
		];
		expect([...absentStudentIds(loose, students, 'c', '2026-09-01')]).toEqual(['s2']);
	});

	it('ignores a learner who is not on the roster', () => {
		const stranger = [
			{ id: 'e9', studentId: 'stranger', type: 'absent' as const, timestamp: at(2026, 9, 1) }
		];
		expect(absentStudentIds(stranger, students, 'c', '2026-09-01').size).toBe(0);
	});

	it('files an evening mark under its own day, not the next one', () => {
		const evening = [
			{
				id: 'e9',
				studentId: 's1',
				classId: 'c',
				type: 'absent' as const,
				timestamp: at(2026, 9, 1, 23)
			}
		];
		expect(absentStudentIds(evening, students, 'c', '2026-09-01').size).toBe(1);
		expect(absentStudentIds(evening, students, 'c', '2026-09-02').size).toBe(0);
	});
});

// ── The marks a write produces ────────────────────────────────────────────────

describe('exportAttendanceMarks', () => {
	const roster = [mapping('s1', 8), mapping('s2', 30)];
	const dates = [date('01', 'F', 6), date('02', 'H', 8)];

	it('writes an X for the absent learner and a blank for everyone else', () => {
		const marks = exportAttendanceMarks({
			dates,
			roster,
			absentIdsFor: (day) => (day === '2026-09-01' ? new Set(['s1']) : new Set<string>())
		});
		expect(marks.filter((mark) => mark.value === SF2_ABSENT_MARK).map((m) => m.address)).toEqual([
			'F8'
		]);
		expect(marks.filter((mark) => mark.value === '').map((m) => m.address)).toEqual(['F30']);
	});

	it('skips a day with no absences rather than blanking the whole column', () => {
		// Attendance was never taken, so the workbook may legitimately hold marks the
		// app has no opinion about.
		expect(exportAttendanceMarks({ dates, roster, absentIdsFor: () => new Set<string>() })).toEqual(
			[]
		);
	});

	it('never writes a row-0 placeholder', () => {
		const marks = exportAttendanceMarks({
			dates: [date('01', 'F', 6)],
			roster: [...roster, mapping('s3', 0)],
			absentIdsFor: () => new Set(['s1', 's2', 's3'])
		});
		expect(marks.map((mark) => mark.address).sort()).toEqual(['F30', 'F8']);
	});
});

// ── The import ────────────────────────────────────────────────────────────────

describe('isAbsentMark', () => {
	it('accepts an X in any casing, with whitespace around it', () => {
		expect(isAbsentMark('X')).toBe(true);
		expect(isAbsentMark('x')).toBe(true);
		expect(isAbsentMark('  X  ')).toBe(true);
		expect(isAbsentMark('\tX\n')).toBe(true);
	});

	it('does not mistake anything else for an absence', () => {
		// The SF2 form is present-by-default: an empty cell means present.
		for (const text of ['', '   ', '=COUNTIF(F9:AL9,"X")', '0', '1', 'XX', 'N/A', 'absent']) {
			expect(isAbsentMark(text), text).toBe(false);
		}
	});
});

describe('gridCellsToScan', () => {
	const roster = [mapping('s1', 9), mapping('s2', 10)];
	const dates = [date('01', 'H', 8), date('02', 'I', 9)];

	it('covers every row × date pair', () => {
		const cells = gridCellsToScan(roster, dates);
		expect(cells).toHaveLength(4);
		expect(cells).toContainEqual({ sheetName: SHEET, address: 'H9' });
		expect(cells).toContainEqual({ sheetName: SHEET, address: 'I10' });
	});

	it('skips the row-0 placeholder, which has no cell in any sheet', () => {
		expect(gridCellsToScan([mapping('s1', 0), mapping('s2', 9)], [date('01', 'H', 8)])).toEqual([
			{ sheetName: SHEET, address: 'H9' }
		]);
	});

	it('is empty without mappings, rather than the whole grid', () => {
		expect(gridCellsToScan([], dates)).toEqual([]);
		expect(gridCellsToScan(roster, [])).toEqual([]);
	});

	it('uses the mapped sheet name, so a hidden or renamed sheet is still read', () => {
		expect(
			gridCellsToScan([mapping('s1', 9)], [{ ...date('01', 'H', 8), sheetName: '__SF2_HIDDEN_2' }])
		).toEqual([{ sheetName: '__SF2_HIDDEN_2', address: 'H9' }]);
	});
});

describe('importAbsentMarks', () => {
	const roster = [mapping('s1', 9)];
	const scanDates = [date('01', 'H', 8)];

	async function scan(
		classId: string,
		textFor: (sheet: string, address: string) => string | undefined
	) {
		return importAbsentMarks({
			classId,
			reportMonth: 'SEPTEMBER',
			dayStart: '07:30',
			roster,
			dates: scanDates,
			textFor
		});
	}

	it('reports nothing to scan for a month with no mappings', () => {
		expect(emptyImportOutcome('c1', 'SEPTEMBER', 0)).toEqual({
			classId: 'c1',
			reportMonth: 'SEPTEMBER',
			scannedCells: 0,
			imported: 0,
			alreadyRecorded: 0,
			datesWithMarks: 0,
			mappedRows: 0,
			mappedDates: 0
		});
	});

	it('records a workbook X the database has never seen', async () => {
		const { classId } = await seedClass();
		const outcome = await scan(classId, () => 'X');
		expect(outcome.imported).toBe(1);
		expect(outcome.scannedCells).toBe(1);
		expect(outcome.datesWithMarks).toBe(1);
		expect(outcome.reportMonth).toBe('SEPTEMBER');
	});

	it('is idempotent: a second pass over the same X adds nothing', async () => {
		const { classId } = await seedClass();
		await scan(classId, () => 'X');
		const second = await scan(classId, () => 'X');

		expect(second.imported).toBe(0);
		expect(second.alreadyRecorded).toBe(1);
		expect(await db().query('SELECT id FROM events')).toHaveLength(1);
	});

	it('skips a cell that could not be read, rather than calling it present', async () => {
		const { classId } = await seedClass();
		const outcome = await scan(classId, () => undefined);
		expect(outcome.imported).toBe(0);
		expect(outcome.datesWithMarks).toBe(0);
	});

	it('ignores a blank cell, because the form is present by default', async () => {
		const { classId } = await seedClass();
		expect((await scan(classId, () => '')).imported).toBe(0);
	});
});

// ── The writer and the reader agree on what an X is ───────────────────────────

describe('the writer and the reader agree on what an X is', () => {
	it('round-trips a mark the write path produced', async () => {
		useFileSystem(new MemoryFileSystem());
		const fixture = await loadTemplate();
		const { classId } = await seedClass();
		const roster = [mapping('s1', 8)];
		const dates = [date('01', 'U', 21)];

		await writeAttendanceToWorkbook({
			target: { sourcePath: fixture.path, sheetName: SHEET, roster, dates },
			absentIdsFor: () => new Set(['s1'])
		});

		const workbook = await fixture.open();
		const outcome = await importAbsentMarks({
			classId,
			reportMonth: 'SEPTEMBER',
			dayStart: '07:30',
			roster,
			dates,
			textFor: (sheetName, address) =>
				cellText((workbook.getWorksheet(sheetName) as Worksheet).getCell(address))
		});
		expect(outcome.imported).toBe(1);
	});
});
