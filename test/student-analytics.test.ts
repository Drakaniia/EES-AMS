import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
	filterAttendanceEventsForQuarter,
	getQuarterDateRange
} from '../src/lib/student-analytics';
import type { Settings } from '../src/lib/types';

const studentAttendanceModal = readFileSync(
	'src/lib/components/students/StudentAttendanceModal.svelte',
	'utf8'
);

function settings(overrides: Partial<Settings> = {}): Settings {
	return {
		id: 'app',
		dayStart: '08:30',
		dayEnd: '15:30',
		lateAfter: '08:45',
		quarter: '1st Quarter',
		attendanceMode: 'manual',
		q1Start: '2025-08-01',
		q1End: '2025-10-31',
		q2Start: '2025-11-01',
		q2End: '2026-01-31',
		q3Start: '2026-02-01',
		q3End: '2026-04-30',
		...overrides
	};
}

function event(id: string, timestamp: string) {
	return { id, timestamp };
}

test('student quarterly analytics use configured school-quarter dates', () => {
	const filtered = filterAttendanceEventsForQuarter(
		[
			event('calendar-q1', '2025-02-05T08:30:00Z'),
			event('school-q1', '2025-08-05T08:30:00Z'),
			event('after-q1', '2025-11-01T08:30:00Z')
		],
		settings(),
		2025
	);

	assert.deepEqual(
		filtered.map((item) => item.id),
		['school-q1']
	);
});

test('student quarterly analytics map each active quarter label to its saved date range', () => {
	assert.deepEqual(getQuarterDateRange(settings({ quarter: '2nd Quarter' }), 2025), {
		startDate: '2025-11-01',
		endDate: '2026-01-31',
		source: 'settings'
	});

	assert.deepEqual(getQuarterDateRange(settings({ quarter: '3rd Quarter' }), 2025), {
		startDate: '2026-02-01',
		endDate: '2026-04-30',
		source: 'settings'
	});
});

test('student quarterly analytics keep calendar fallback when saved dates are missing', () => {
	const filtered = filterAttendanceEventsForQuarter(
		[
			event('before-q2', '2026-03-31T08:30:00Z'),
			event('calendar-q2', '2026-04-01T08:30:00Z'),
			event('after-q2', '2026-07-01T08:30:00Z')
		],
		settings({ quarter: '2nd Quarter', q2Start: undefined, q2End: undefined }),
		2026
	);

	assert.deepEqual(
		filtered.map((item) => item.id),
		['calendar-q2']
	);
});

test('student attendance modal no longer compares numeric calendar quarters to labels', () => {
	assert.match(studentAttendanceModal, /filterAttendanceEventsForQuarter/);
	assert.doesNotMatch(studentAttendanceModal, /Math\.floor\(d\.getMonth\(\) \/ 3\) \+ 1/);
	assert.doesNotMatch(studentAttendanceModal, /q\.toString\(\) === settings\?\.quarter/);
});
