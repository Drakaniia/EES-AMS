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
	...baseFields,
	brandingLogoPath: null,
	brandingTitle: 'EES AMS'
};

describe('buildGlobalSettingsPayload', () => {
	it('includes brandingLogoPath and brandingTitle from fields', () => {
		const payload = buildGlobalSettingsPayload({
			...baseFields,
			brandingLogoPath: '/some/path.png',
			brandingTitle: 'Mrs. Santos'
		});
		expect(payload.brandingLogoPath).toBe('/some/path.png');
		expect(payload.brandingTitle).toBe('Mrs. Santos');
	});

	it('defaults brandingLogoPath to null when not provided', () => {
		const payload = buildGlobalSettingsPayload(baseFields);
		expect(payload.brandingLogoPath).toBeNull();
	});

	it('defaults brandingTitle to EES AMS when not provided', () => {
		const payload = buildGlobalSettingsPayload(baseFields);
		expect(payload.brandingTitle).toBe('EES AMS');
	});
});

describe('normalizeGlobalSettings', () => {
	it('preserves brandingLogoPath when set', () => {
		const input: Settings = {
			...baseSettings,
			brandingLogoPath: '/assets/branding/user-abc.png'
		};
		const normalized = normalizeGlobalSettings(input);
		expect(normalized.brandingLogoPath).toBe('/assets/branding/user-abc.png');
	});

	it('defaults brandingLogoPath to null when undefined', () => {
		const input: Settings = {
			...baseSettings,
			brandingLogoPath: undefined
		};
		const normalized = normalizeGlobalSettings(input);
		expect(normalized.brandingLogoPath).toBeNull();
	});

	it('preserves brandingTitle when set', () => {
		const input: Settings = {
			...baseSettings,
			brandingTitle: 'Room 201 - Grade 3'
		};
		const normalized = normalizeGlobalSettings(input);
		expect(normalized.brandingTitle).toBe('Room 201 - Grade 3');
	});

	it('defaults brandingTitle to EES AMS when undefined', () => {
		const input: Settings = {
			...baseSettings,
			brandingTitle: undefined
		};
		const normalized = normalizeGlobalSettings(input);
		expect(normalized.brandingTitle).toBe('EES AMS');
	});
});

describe('globalSettingsEqual', () => {
	it('returns true when branding fields match', () => {
		const a: Settings = { ...baseSettings, brandingTitle: 'Custom' };
		const b: Settings = { ...baseSettings, brandingTitle: 'Custom' };
		expect(globalSettingsEqual(a, b)).toBe(true);
	});

	it('returns false when brandingTitle differs', () => {
		const a: Settings = { ...baseSettings, brandingTitle: 'Custom A' };
		const b: Settings = { ...baseSettings, brandingTitle: 'Custom B' };
		expect(globalSettingsEqual(a, b)).toBe(false);
	});

	it('returns false when brandingLogoPath differs', () => {
		const a: Settings = { ...baseSettings, brandingLogoPath: '/a.png' };
		const b: Settings = { ...baseSettings, brandingLogoPath: '/b.png' };
		expect(globalSettingsEqual(a, b)).toBe(false);
	});

	it('returns true when both brandingLogoPath are null', () => {
		const a: Settings = { ...baseSettings, brandingLogoPath: null };
		const b: Settings = { ...baseSettings, brandingLogoPath: null };
		expect(globalSettingsEqual(a, b)).toBe(true);
	});
});
