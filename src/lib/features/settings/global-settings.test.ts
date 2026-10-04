import { describe, it, expect } from 'vitest';
import {
	buildGlobalSettingsPayload,
	normalizeGlobalSettings,
	globalSettingsEqual,
	type GlobalSettingsFields
} from './global-settings';
import type { Settings } from '$lib/types';

const baseFields: GlobalSettingsFields = {
	dayStart: '08:00',
	dayEnd: '15:00',
	lateAfter: '08:45',
	quarter: '1st Quarter',
	attendanceMode: 'manual',
	q1Start: '',
	q1End: '',
	q2Start: '',
	q2End: '',
	q3Start: '',
	q3End: ''
};

const baseSettings: Settings = {
	id: 'app',
	...baseFields
};

describe('buildGlobalSettingsPayload', () => {
	it('carries the fields through with the app row id', () => {
		const payload = buildGlobalSettingsPayload({ ...baseFields, dayStart: '09:00' });
		expect(payload).toEqual({ id: 'app', ...baseFields, dayStart: '09:00' });
	});
});

describe('normalizeGlobalSettings', () => {
	it('defaults a missing quarter window to the empty string', () => {
		const normalized = normalizeGlobalSettings({
			...baseSettings,
			q2Start: undefined
		});
		expect(normalized.q2Start).toBe('');
	});

	it('preserves a present quarter window', () => {
		const normalized = normalizeGlobalSettings({ ...baseSettings, q2Start: '2026-12-01' });
		expect(normalized.q2Start).toBe('2026-12-01');
	});
});

describe('globalSettingsEqual', () => {
	it('returns true for identical settings', () => {
		expect(globalSettingsEqual(baseSettings, { ...baseSettings })).toBe(true);
	});

	it('returns false when a quarter window differs', () => {
		expect(globalSettingsEqual(baseSettings, { ...baseSettings, q2Start: '2026-12-01' })).toBe(
			false
		);
	});

	it('treats an undefined window as the same as an empty one', () => {
		expect(globalSettingsEqual(baseSettings, { ...baseSettings, q3End: undefined })).toBe(true);
	});
});
