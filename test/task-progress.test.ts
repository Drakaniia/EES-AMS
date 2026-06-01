import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

const progressComponentPath = 'src/lib/components/ui/TaskProgress.svelte';
const settingsPage = readFileSync('src/routes/settings/+page.svelte', 'utf8');
const attendancePage = readFileSync('src/routes/attendance/+page.svelte', 'utf8');

test('provides an accessible animated task progress component', () => {
	assert.equal(existsSync(progressComponentPath), true);

	const progressComponent = readFileSync(progressComponentPath, 'utf8');

	assert.match(progressComponent, /role="progressbar"/);
	assert.match(progressComponent, /aria-live="polite"/);
	assert.match(progressComponent, /animate-spin/);
	assert.match(progressComponent, /progress-slide/);
});

test('SF2 import and workbook creation expose progress indicators', () => {
	assert.match(
		settingsPage,
		/import TaskProgress from '\$lib\/components\/ui\/TaskProgress\.svelte';/
	);
	assert.match(settingsPage, /active=\{sf2Importing\}/);
	assert.match(settingsPage, /title="Importing SF2 workbook"/);
	assert.match(settingsPage, /active=\{sf2TemplateCreating \|\| sf2SettingsSaving\}/);
	assert.match(settingsPage, /title=\{sf2TemplateProgressTitle\}/);
});

test('attendance session closing exposes progress indicator', () => {
	assert.match(
		attendancePage,
		/import TaskProgress from '\$lib\/components\/ui\/TaskProgress\.svelte';/
	);
	assert.match(attendancePage, /active=\{isClosingDay\}/);
	assert.match(attendancePage, /title="Closing attendance session"/);
	assert.match(attendancePage, /animate-spin/);
});
