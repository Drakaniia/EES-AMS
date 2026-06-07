import type { AttendanceMode, Settings } from '$lib/types';

export type GlobalSettingsFields = {
	dayStart: string;
	dayEnd: string;
	lateAfter: string;
	quarter: string;
	attendanceMode: AttendanceMode;
	q1Start: string;
	q1End: string;
	q2Start: string;
	q2End: string;
	q3Start: string;
	q3End: string;
};

export const DEFAULT_GLOBAL_SETTINGS: Settings = {
	id: 'app',
	dayStart: '08:30',
	dayEnd: '15:30',
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

export function buildGlobalSettingsPayload(fields: GlobalSettingsFields): Settings {
	return {
		id: 'app',
		...fields
	};
}

export function normalizeGlobalSettings(settings: Settings): Settings {
	return {
		id: settings.id,
		dayStart: settings.dayStart,
		dayEnd: settings.dayEnd,
		lateAfter: settings.lateAfter,
		quarter: settings.quarter,
		attendanceMode: settings.attendanceMode ?? DEFAULT_GLOBAL_SETTINGS.attendanceMode,
		q1Start: settings.q1Start ?? DEFAULT_GLOBAL_SETTINGS.q1Start,
		q1End: settings.q1End ?? DEFAULT_GLOBAL_SETTINGS.q1End,
		q2Start: settings.q2Start ?? DEFAULT_GLOBAL_SETTINGS.q2Start,
		q2End: settings.q2End ?? DEFAULT_GLOBAL_SETTINGS.q2End,
		q3Start: settings.q3Start ?? DEFAULT_GLOBAL_SETTINGS.q3Start,
		q3End: settings.q3End ?? DEFAULT_GLOBAL_SETTINGS.q3End
	};
}

export function globalSettingsEqual(a: Settings, b: Settings) {
	const left = normalizeGlobalSettings(a);
	const right = normalizeGlobalSettings(b);
	return (
		left.dayStart === right.dayStart &&
		left.dayEnd === right.dayEnd &&
		left.lateAfter === right.lateAfter &&
		left.quarter === right.quarter &&
		left.attendanceMode === right.attendanceMode &&
		left.q1Start === right.q1Start &&
		left.q1End === right.q1End &&
		left.q2Start === right.q2Start &&
		left.q2End === right.q2End &&
		left.q3Start === right.q3Start &&
		left.q3End === right.q3End
	);
}
