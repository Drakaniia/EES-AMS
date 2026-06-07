import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { sf2ValidationReportText } from '../src/lib/features/settings/sf2-validation';
import type { Sf2ImportValidation } from '../src/lib/types';

const settingsPage = readFileSync('src/routes/settings/+page.svelte', 'utf8');
const settingsNative = readFileSync('src/lib/features/settings/native.ts', 'utf8');
const types = readFileSync('src/lib/types.ts', 'utf8');

test('SF2 import validates before confirmed import and exposes mismatch actions', () => {
	assert.match(settingsNative, /validateSf2WorkbookImport/);
	assert.match(settingsNative, /importSf2Workbook/);
	assert.match(types, /interface Sf2ImportValidation/);
	assert.match(settingsPage, /sf2ValidationDialogOpen/);
	assert.match(settingsPage, /Warning: Student List Mismatch Detected/);
	assert.match(settingsPage, /Review Differences/);
	assert.match(settingsPage, /Download Validation Report/);
	assert.match(settingsPage, /Cancel Import/);
	assert.match(settingsPage, /Proceed Anyway/);
});

test('SF2 validation report text is built outside the settings route', () => {
	const validation: Sf2ImportValidation = {
		sourcePath: 'C:/sf2.xls',
		className: 'Grade 1',
		currentStudentCount: 1,
		sf2LearnerCount: 1,
		missingFromSf2: [
			{
				studentId: 'student-1',
				name: 'Learner One',
				normalizedName: 'LEARNER ONE',
				gender: 'Male'
			}
		],
		missingFromCurrent: [],
		possibleNameMismatches: [],
		duplicateCurrentStudents: [],
		duplicateSf2Learners: [],
		missingLearnerInfo: [],
		hasDiscrepancies: true
	};

	const report = sf2ValidationReportText(validation);

	assert.match(report, /Warning: Student List Mismatch Detected/);
	assert.match(report, /Source path: C:\/sf2\.xls/);
	assert.match(report, /- Learner One \(Male\)/);
});
