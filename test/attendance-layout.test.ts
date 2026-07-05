import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const attendanceGrid = readFileSync('src/routes/attendance/attendance-grid.svelte', 'utf8');

test('manual attendance boxes keep a stable fixed size for full names', () => {
	assert.match(attendanceGrid, /auto-rows-\[116px\]/);
	assert.match(attendanceGrid, /h-\[116px\][^"]*overflow-hidden/);
	assert.match(attendanceGrid, /student-card-name/);
	assert.match(attendanceGrid, /minmax\(168px,1fr\)/);
	assert.doesNotMatch(attendanceGrid, /line-clamp-2/);
});

test('manual attendance list does not truncate student names', () => {
	assert.doesNotMatch(
		attendanceGrid,
		/<div class="truncate text-base font-semibold">\{student\.name\}/
	);
});
