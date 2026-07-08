import { invoke } from '@tauri-apps/api/core';
import type {
	AttendanceEvent,
	CreateEventRequest,
	UpdateEventRequest,
	AttendanceAuditEntry
} from '../types';
export type {
	AttendanceEvent,
	CreateEventRequest,
	UpdateEventRequest,
	AttendanceAuditEntry
} from '../types';

export async function listEvents(): Promise<AttendanceEvent[]> {
	return await invoke('list_events');
}

export async function listEventsForDate(date: string): Promise<AttendanceEvent[]> {
	return await invoke('list_events_for_date', { date });
}

export async function listEventsForStudent(studentId: string): Promise<AttendanceEvent[]> {
	return await invoke('list_events_for_student', { studentId });
}

export async function lastEventForStudent(studentId: string): Promise<AttendanceEvent | undefined> {
	return await invoke('last_event_for_student', { studentId });
}

export async function addEvent(event: CreateEventRequest): Promise<AttendanceEvent> {
	return await invoke('add_event', { req: event });
}

export async function addEvents(events: CreateEventRequest[]): Promise<AttendanceEvent[]> {
	return await invoke('add_events', { reqs: events });
}

export async function updateEvent(id: string, req: UpdateEventRequest): Promise<AttendanceEvent> {
	return await invoke('update_event', { id, req });
}

export async function deleteEvent(id: string, reason?: string): Promise<void> {
	return await invoke('delete_event', { id, reason });
}

export async function deleteEvents(ids: string[], reason?: string): Promise<void> {
	return await invoke('delete_events', { ids, reason });
}

export async function listAttendanceAudit(filters?: {
	eventId?: string;
	studentId?: string;
}): Promise<AttendanceAuditEntry[]> {
	return await invoke('list_attendance_audit', {
		eventId: filters?.eventId,
		studentId: filters?.studentId
	});
}
