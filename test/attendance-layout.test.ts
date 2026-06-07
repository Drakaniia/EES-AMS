import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const attendancePage = readFileSync('src/routes/attendance/+page.svelte', 'utf8');

test('manual attendance boxes keep a stable fixed size for full names', () => {
	assert.match(attendancePage, /auto-rows-\[116px\]/);
	assert.match(attendancePage, /h-\[116px\][^"]*overflow-hidden/);
	assert.match(attendancePage, /student-card-name/);
	assert.match(attendancePage, /minmax\(168px,1fr\)/);
	assert.doesNotMatch(attendancePage, /line-clamp-2/);
});

test('manual attendance list does not truncate student names', () => {
	assert.doesNotMatch(
		attendancePage,
		/<div class="truncate text-base font-semibold">\{student\.name\}/
	);
});
