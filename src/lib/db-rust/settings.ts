import { invoke } from '@tauri-apps/api/core';
import type { Settings, AuditEvent, AttendanceMode } from '../types';
export type { Settings, AuditEvent, AttendanceMode } from '../types';

export async function listAuditEvents(limit = 200): Promise<AuditEvent[]> {
	return await invoke('list_audit_events', { limit });
}

export async function clearAuditEvents(): Promise<number> {
	return await invoke('clear_audit_events');
}

export async function getSettings(): Promise<Settings> {
	const backendSettings = (await invoke('get_settings')) as {
		id: string;
		dayStart: string;
		dayEnd: string;
		lateAfter: string;
		quarter: string;
		attendanceMode?: AttendanceMode;
		q1Start?: string;
		q1End?: string;
		q2Start?: string;
		q2End?: string;
		q3Start?: string;
		q3End?: string;
	};
	return {
		id: backendSettings.id,
		dayStart: backendSettings.dayStart,
		dayEnd: backendSettings.dayEnd,
		lateAfter: backendSettings.lateAfter,
		quarter: backendSettings.quarter,
		attendanceMode: backendSettings.attendanceMode ?? 'manual',
		q1Start: backendSettings.q1Start,
		q1End: backendSettings.q1End,
		q2Start: backendSettings.q2Start,
		q2End: backendSettings.q2End,
		q3Start: backendSettings.q3Start,
		q3End: backendSettings.q3End
	};
}

export async function saveSettings(settings: Settings): Promise<Settings> {
	const backendSettings = {
		id: settings.id,
		dayStart: settings.dayStart,
		dayEnd: settings.dayEnd,
		lateAfter: settings.lateAfter,
		quarter: settings.quarter,
		attendanceMode: settings.attendanceMode,
		q1Start: settings.q1Start,
		q1End: settings.q1End,
		q2Start: settings.q2Start,
		q2End: settings.q2End,
		q3Start: settings.q3Start,
		q3End: settings.q3End
	};
	return await invoke('save_settings', { settings: backendSettings });
}
