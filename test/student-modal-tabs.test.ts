import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const studentForm = readFileSync('src/routes/students/student-form.svelte', 'utf8');

test('add student entry tabs use a morphing indicator and keyed content animation', () => {
	assert.match(studentForm, /add-student-entry-tabs/);
	assert.match(studentForm, /add-student-tab-indicator/);
	assert.match(studentForm, /translateX\(100%\)/);
	assert.match(studentForm, /\{#key entryMode\}/);
});

test('add student entry mode controls expose tab semantics', () => {
	assert.match(studentForm, /role="tablist"/);
	assert.match(studentForm, /role="tab"/);
	assert.match(studentForm, /aria-selected=\{entryMode === tab.value\}/);
});
