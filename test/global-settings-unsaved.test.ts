import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const settingsPage = readFileSync('src/routes/settings/+page.svelte', 'utf8');

test('global settings track dirty state before reloads can overwrite form fields', () => {
	assert.match(settingsPage, /let savedGlobalSettingsSnapshot = \$state<Settings \| null>\(null\)/);
	assert.match(settingsPage, /let pendingGlobalSettingsReload = \$state<Settings \| null>\(null\)/);
	assert.match(settingsPage, /let unsavedGlobalDialogOpen = \$state\(false\)/);
	assert.match(settingsPage, /let globalSettingsDirty = \$derived\.by\(\s*\(\) =>/);
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
