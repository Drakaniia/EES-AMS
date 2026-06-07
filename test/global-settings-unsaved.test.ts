import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
	buildGlobalSettingsPayload,
	globalSettingsEqual,
	normalizeGlobalSettings
} from '../src/lib/features/settings/global-settings';
import type { Settings } from '../src/lib/types';

const settingsPage = readFileSync('src/routes/settings/+page.svelte', 'utf8');

function settings(overrides: Partial<Settings> = {}): Settings {
	return {
		id: 'app',
		dayStart: '08:30',
		dayEnd: '15:30',
		lateAfter: '08:45',
		quarter: '1st Quarter',
		attendanceMode: 'manual',
		...overrides
	};
}

test('global settings workflow normalizes optional fields for dirty checks', () => {
	const normalized = normalizeGlobalSettings(settings());

	assert.equal(normalized.q1Start, '');
	assert.equal(normalized.q1End, '');
	assert.equal(normalized.q2Start, '');
	assert.equal(normalized.q2End, '');
	assert.equal(normalized.q3Start, '');
	assert.equal(normalized.q3End, '');
	assert.equal(globalSettingsEqual(settings(), normalized), true);
});

test('global settings workflow builds the save payload from form fields', () => {
	const payload = buildGlobalSettingsPayload({
		dayStart: '07:30',
		dayEnd: '14:30',
		lateAfter: '07:45',
		quarter: '2nd Quarter',
		attendanceMode: 'card_reader',
		q1Start: '2026-06-01',
		q1End: '2026-08-31',
		q2Start: '',
		q2End: '',
		q3Start: '',
		q3End: ''
	});

	assert.equal(payload.id, 'app');
	assert.equal(payload.dayStart, '07:30');
	assert.equal(payload.attendanceMode, 'card_reader');
	assert.equal(payload.q1Start, '2026-06-01');
});

test('settings page wires global dirty state to feature helpers', () => {
	assert.match(settingsPage, /from '\$lib\/features\/settings\/global-settings'/);
	assert.match(settingsPage, /let savedGlobalSettingsSnapshot = \$state<Settings \| null>\(null\)/);
	assert.match(settingsPage, /let pendingGlobalSettingsReload = \$state<Settings \| null>\(null\)/);
	assert.match(settingsPage, /let unsavedGlobalDialogOpen = \$state\(false\)/);
	assert.match(settingsPage, /let globalSettingsDirty = \$derived\.by\(\s*\(\) =>/);
	assert.match(settingsPage, /buildGlobalSettingsPayload\(\{/);
	assert.match(settingsPage, /function applyGlobalSettings\(settings: Settings\)/);
	assert.match(settingsPage, /function handleGlobalSettingsFocusOut\(event: FocusEvent\)/);
	assert.match(settingsPage, /onfocusout=\{handleGlobalSettingsFocusOut\}/);
});

test('global settings reload prompts instead of resetting unsaved edits', () => {
	assert.match(settingsPage, /if \(globalSettingsDirty\) \{/);
	assert.match(settingsPage, /pendingGlobalSettingsReload = loadedSettings/);
	assert.match(settingsPage, /unsavedGlobalDialogOpen = true/);
	assert.match(settingsPage, /return;/);
});

test('global settings unsaved dialog offers save discard and keep editing actions', () => {
	assert.match(settingsPage, /title="Unsaved Global Settings"/);
	assert.match(settingsPage, /description="You have unsaved changes in Global Configuration\."/);
	assert.match(settingsPage, /onclick=\{keepEditingGlobalSettings\}/);
	assert.match(settingsPage, /onclick=\{discardGlobalSettingsChanges\}/);
	assert.match(settingsPage, /onclick=\{saveGlobalSettingsFromDialog\}/);
	assert.match(settingsPage, />\s*Keep Editing\s*</);
	assert.match(settingsPage, />\s*Discard Changes\s*</);
	assert.match(settingsPage, /Save Changes/);
});
