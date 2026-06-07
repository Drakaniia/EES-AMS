import assert from 'node:assert/strict';
import test from 'node:test';

import {
	normalizeSf2ReportMonth,
	sf2ImportedSettingsDraftDefaults,
	shouldPromptForSf2SettingsUpdate
} from '../src/lib/features/settings/sf2-workbook';
import type { Sf2WorkbookSettings } from '../src/lib/types';

function workbookSettings(overrides: Partial<Sf2WorkbookSettings> = {}): Sf2WorkbookSettings {
	return {
		templateId: 'template-1',
		classId: 'class-1',
		className: 'Grade 1 - Sampaguita',
		sourcePath: 'C:/sf2.xls',
		schoolId: '123456',
		schoolName: 'Espiritu Elementary School',
		schoolYear: '2025-2026',
		reportMonth: 'MARCH',
		gradeLevel: '1',
		section: 'Sampaguita',
		adviserName: 'Ada Teacher',
		schoolHeadName: 'Grace Principal',
		firstSchoolDay: 2,
		learnerNames: ['One Learner'],
		datesMapped: 20,
		...overrides
	};
}

test('normalizes SF2 report month values imported from workbook cells', () => {
	assert.equal(normalizeSf2ReportMonth('march'), 'MARCH');
	assert.equal(normalizeSf2ReportMonth('Report for the Month of: June'), 'JUNE');
	assert.equal(normalizeSf2ReportMonth(''), '');
});

test('detects imported SF2 report month that differs from the current month', () => {
	const today = new Date(2026, 5, 1);

	assert.equal(shouldPromptForSf2SettingsUpdate(workbookSettings(), today), true);
	assert.equal(
		shouldPromptForSf2SettingsUpdate(workbookSettings({ reportMonth: 'June' }), today),
		false
	);
});

test('prefills imported settings while selecting the current SF2 month', () => {
	const today = new Date(2026, 5, 1);
	const defaults = sf2ImportedSettingsDraftDefaults(workbookSettings(), today);

	assert.equal(defaults.schoolId, '123456');
	assert.equal(defaults.schoolName, 'Espiritu Elementary School');
	assert.equal(defaults.gradeLevel, '1');
	assert.equal(defaults.section, 'Sampaguita');
	assert.equal(defaults.adviserName, 'Ada Teacher');
	assert.equal(defaults.schoolHeadName, 'Grace Principal');
	assert.equal(defaults.reportMonth, 'JUNE');
	assert.equal(defaults.schoolYear, '2026-2027');
	assert.equal(defaults.firstSchoolDay, 1);
});

test('keeps the imported school year when it already matches the current month year', () => {
	const today = new Date(2027, 0, 6);
	const defaults = sf2ImportedSettingsDraftDefaults(
		workbookSettings({ schoolYear: '2026-2027', reportMonth: 'DECEMBER' }),
		today
	);

	assert.equal(defaults.reportMonth, 'JANUARY');
	assert.equal(defaults.schoolYear, '2026-2027');
	assert.equal(defaults.firstSchoolDay, 1);
});
