import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const settingsPage = readFileSync('src/routes/settings/+page.svelte', 'utf8');
const dbRust = readFileSync('src/lib/db-rust.ts', 'utf8');
const types = readFileSync('src/lib/types.ts', 'utf8');

test('SF2 import validates before confirmed import and exposes mismatch actions', () => {
	assert.match(dbRust, /validateSf2WorkbookImport/);
	assert.match(dbRust, /importSf2Workbook\(\s*sourcePath: string,\s*proceedAnyway: boolean\s*\)/);
	assert.match(types, /interface Sf2ImportValidation/);
	assert.match(settingsPage, /sf2ValidationDialogOpen/);
	assert.match(settingsPage, /Warning: Student List Mismatch Detected/);
	assert.match(settingsPage, /Review Differences/);
	assert.match(settingsPage, /Download Validation Report/);
	assert.match(settingsPage, /Cancel Import/);
	assert.match(settingsPage, /Proceed Anyway/);
});
