import { describe, expect, it } from 'vitest';
import type { AttendanceEvent, Class, Student } from '$lib/types';
import { buildAttendanceCsv, escapeCsvField } from '../csv';

/**
 * The attendance CSV the school is handed.
 *
 * Two things make or break this file. Quoting: a name like `Dela Cruz, Jr.` is
 * ordinary text, and an unquoted comma silently shifts every later column. And
 * lateness: the row reports the *earliest* check-in of the day, because a card
 * reader fires more than once and the later scans are not when the child
 * arrived.
 */

const classOne: Class = {
	id: 'c1',
	name: 'Mabini, Grade 1',
	room: 'Room 1',
	dayStart: '08:00',
	dayEnd: '15:00',
	lateAfter: '08:45',
	sessions: [
		{ name: 'Morning', startTime: '07:30', endTime: '11:30', lateAfter: '08:15' },
		{ name: 'Afternoon', startTime: '13:00', endTime: '17:00', lateAfter: '13:30' }
	],
	days: [1, 2, 3, 4, 5],
	createdAt: '2025-01-01T00:00:00.000Z'
};

const students: Student[] = [
	{ id: 's1', name: 'Dela Cruz, Jr.', classId: 'c1', createdAt: '2025-01-01T00:00:00.000Z' },
	{ id: 's2', name: 'Santos', createdAt: '2025-01-01T00:00:00.000Z' }
];

/** Local-time constructor: these rows are about "08:47 in the school's morning". */
function at(hours: number, minutes: number, day = 15): string {
	return new Date(2025, 0, day, hours, minutes, 0).toISOString();
}

function checkIn(studentId: string, hours: number, minutes: number, day = 15): AttendanceEvent {
	return {
		id: `${studentId}-${hours}-${minutes}-${day}`,
		studentId,
		type: 'in',
		timestamp: at(hours, minutes, day)
	};
}

describe('escapeCsvField', () => {
	it('quotes commas, quotes and newlines, and doubles embedded quotes', () => {
		expect(escapeCsvField('Dela Cruz, Jr.')).toBe('"Dela Cruz, Jr."');
		expect(escapeCsvField('The "Best" class')).toBe('"The ""Best"" class"');
		expect(escapeCsvField('line one\nline two')).toBe('"line one\nline two"');
		expect(escapeCsvField('carriage\rreturn')).toBe('"carriage\rreturn"');
	});

	it('leaves ordinary text alone', () => {
		expect(escapeCsvField('Santos')).toBe('Santos');
		expect(escapeCsvField('')).toBe('');
	});
});

describe('buildAttendanceCsv', () => {
	it('starts with the header the school expects', () => {
		expect(buildAttendanceCsv([], [], [])).toBe('Date,Class,Room,Name,IN,Late\n');
	});

	it('reports the earliest check-in of the day, not the last scan', () => {
		const csv = buildAttendanceCsv(
			[checkIn('s1', 8, 50), checkIn('s1', 8, 2), checkIn('s1', 9, 15)],
			students,
			[classOne]
		);
		const [row] = csv.trim().split('\n').slice(1);
		expect(row.endsWith(',08:02,No')).toBe(true);
	});

	it('marks a check-in after the session threshold as late', () => {
		const csv = buildAttendanceCsv([checkIn('s1', 8, 47)], students, [classOne]);
		expect(csv.trim().split('\n')[1].endsWith('08:47,Yes')).toBe(true);
	});

	it('uses the afternoon session threshold for an afternoon check-in', () => {
		const csv = buildAttendanceCsv([checkIn('s1', 13, 31)], students, [classOne]);
		expect(csv.trim().split('\n')[1].endsWith('13:31,Yes')).toBe(true);
	});

	it('ignores absence marks, which are not check-ins', () => {
		const absent: AttendanceEvent = {
			id: 'a1',
			studentId: 's1',
			classId: 'c1',
			type: 'absent',
			timestamp: at(8, 0)
		};
		expect(buildAttendanceCsv([absent], students, [classOne])).toBe(
			'Date,Class,Room,Name,IN,Late\n'
		);
	});

	it('keeps one row per student per day', () => {
		const csv = buildAttendanceCsv(
			[checkIn('s1', 8, 2), checkIn('s1', 8, 5), checkIn('s1', 8, 9, 16)],
			students,
			[classOne]
		);
		expect(csv.trim().split('\n')).toHaveLength(3);
	});

	it('falls back to Unknown and N/A for a student with no class', () => {
		const csv = buildAttendanceCsv([checkIn('s2', 8, 2)], students, [classOne]);
		expect(csv.trim().split('\n')[1]).toContain(',Unknown,N/A,Santos,08:02,');
	});

	it('drops a check-in for a student who is no longer on the roster', () => {
		const orphan = checkIn('gone', 8, 2);
		expect(buildAttendanceCsv([orphan], students, [classOne])).toBe(
			'Date,Class,Room,Name,IN,Late\n'
		);
	});

	it('quotes a comma in a class name so the columns do not shift', () => {
		const csv = buildAttendanceCsv([checkIn('s1', 8, 2)], students, [classOne]);
		expect(csv.trim().split('\n')[1]).toBe(
			`2025-01-15,"Mabini, Grade 1",Room 1,"Dela Cruz, Jr.",08:02,No`
		);
	});
});
