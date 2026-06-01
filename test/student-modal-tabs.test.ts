import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const studentsPage = readFileSync('src/routes/students/+page.svelte', 'utf8');

test('add student entry tabs use a morphing indicator and keyed content animation', () => {
	assert.match(studentsPage, /function setEntryMode\(mode: EntryMode\)/);
	assert.match(studentsPage, /class="add-student-entry-tabs/);
	assert.match(studentsPage, /add-student-tab-indicator/);
	assert.match(studentsPage, /translateX\(100%\)/);
	assert.match(studentsPage, /\{#key entryMode\}/);
	assert.match(studentsPage, /tab-panel-morph/);
	assert.match(studentsPage, /@keyframes tab-panel-morph/);
});

test('add student entry mode controls expose tab semantics', () => {
	assert.match(studentsPage, /role="tablist"/);
	assert.match(studentsPage, /role="tab"/);
	assert.match(studentsPage, /aria-selected=\{entryMode === tab.value\}/);
});
