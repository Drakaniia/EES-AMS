import { invoke } from '@tauri-apps/api/core';
import type { Student, Class, Session, AttendanceEvent, Settings, ExportData } from './types';
export type { Student, Class, Session, AttendanceEvent, Settings, ExportData };

// ── Student Operations ───────────────────────────────────────────────────────

export async function listStudents(classId?: string): Promise<Student[]> {
	return await invoke('list_students', { classId });
}

export async function getStudent(id: string): Promise<Student> {
	return await invoke('get_student', { id });
}

export async function findStudentByCard(serial: string): Promise<Student | undefined> {
	return await invoke('find_student_by_card', { serial });
}

export async function saveStudent(student: Student): Promise<Student> {
	if (student.id) {
		return await invoke('update_student', {
			id: student.id,
			req: {
				name: student.name,
				studentNumber: student.studentNumber,
				cardSerial: student.cardSerial,
				classId: student.classId
			}
		});
	} else {
		return await invoke('create_student', {
			req: {
				name: student.name,
				studentNumber: student.studentNumber,
				cardSerial: student.cardSerial,
				classId: student.classId
			}
		});
	}
}

export async function deleteStudent(id: string): Promise<void> {
	return await invoke('delete_student', { id });
}

// ── Class Operations ─────────────────────────────────────────────────────────

export async function listClasses(): Promise<Class[]> {
	const backendClasses = (await invoke('list_classes')) as Array<Class>;
	// Backend already returns camelCase due to serde(rename_all = "camelCase")
	return backendClasses.map((cls) => ({
		id: cls.id,
		name: cls.name,
		room: cls.room,
		dayStart: cls.dayStart,
		dayEnd: cls.dayEnd,
		lateAfter: cls.lateAfter,
		sessions: cls.sessions,
		days: cls.days,
		createdAt: cls.createdAt
	}));
}

export async function getClass(id: string): Promise<Class | undefined> {
	const backendClass = (await invoke('get_class', { id })) as Class | undefined;

	if (!backendClass) return undefined;

	// Backend already returns camelCase due to serde(rename_all = "camelCase")
	return {
		id: backendClass.id,
		name: backendClass.name,
		room: backendClass.room,
		dayStart: backendClass.dayStart,
		dayEnd: backendClass.dayEnd,
		lateAfter: backendClass.lateAfter,
		sessions: backendClass.sessions,
		days: backendClass.days,
		createdAt: backendClass.createdAt
	};
}

export async function saveClass(classData: Class, isUpdate: boolean = false): Promise<Class> {
	let backendClass: Class;

	if (isUpdate) {
		// Update existing class
		backendClass = await invoke('update_class', {
			id: classData.id,
			req: {
				name: classData.name,
				room: classData.room,
				dayStart: classData.dayStart,
				dayEnd: classData.dayEnd,
				lateAfter: classData.lateAfter,
				sessions: classData.sessions,
				days: classData.days
			}
		});
	} else {
		// Create new class - don't pass the frontend-generated ID
		backendClass = await invoke('create_class', {
			req: {
				name: classData.name,
				room: classData.room,
				dayStart: classData.dayStart,
				dayEnd: classData.dayEnd,
				lateAfter: classData.lateAfter,
				sessions: classData.sessions,
				days: classData.days
			}
		});
	}

	// Backend already returns camelCase due to serde(rename_all = "camelCase")
	return {
		id: backendClass.id,
		name: backendClass.name,
		room: backendClass.room,
		dayStart: backendClass.dayStart,
		dayEnd: backendClass.dayEnd,
		lateAfter: backendClass.lateAfter,
		sessions: backendClass.sessions,
		days: backendClass.days,
		createdAt: backendClass.createdAt
	};
}

export async function deleteClass(id: string): Promise<void> {
	return await invoke('delete_class', { id });
}

// ── Event Operations ─────────────────────────────────────────────────────────

export async function listEvents(): Promise<AttendanceEvent[]> {
	return await invoke('list_events');
}

export async function listEventsForStudent(studentId: string): Promise<AttendanceEvent[]> {
	return await invoke('list_events_for_student', { studentId });
}

export async function lastEventForStudent(studentId: string): Promise<AttendanceEvent | undefined> {
	return await invoke('last_event_for_student', { studentId });
}

export async function addEvent(
	event: Omit<AttendanceEvent, 'id' | 'timestamp'>
): Promise<AttendanceEvent> {
	return await invoke('add_event', {
		req: event
	});
}

export async function deleteEvent(id: string): Promise<void> {
	return await invoke('delete_event', { id });
}

// ── Settings Operations ───────────────────────────────────────────────────────

export async function getSettings(): Promise<Settings> {
	const backendSettings = (await invoke('get_settings')) as {
		id: string;
		dayStart: string;
		dayEnd: string;
		lateAfter: string;
		quarter: string;
		q1Start?: string;
		q1End?: string;
		q2Start?: string;
		q2End?: string;
		q3Start?: string;
		q3End?: string;
		q4Start?: string;
		q4End?: string;
	};
	// Backend already returns camelCase due to serde(rename_all = "camelCase")
	return {
		id: backendSettings.id,
		dayStart: backendSettings.dayStart,
		dayEnd: backendSettings.dayEnd,
		lateAfter: backendSettings.lateAfter,
		quarter: backendSettings.quarter,
		q1Start: backendSettings.q1Start,
		q1End: backendSettings.q1End,
		q2Start: backendSettings.q2Start,
		q2End: backendSettings.q2End,
		q3Start: backendSettings.q3Start,
		q3End: backendSettings.q3End,
		q4Start: backendSettings.q4Start,
		q4End: backendSettings.q4End
	};
}

export async function saveSettings(settings: Settings): Promise<Settings> {
	// Backend expects camelCase due to serde(rename_all = "camelCase")
	const backendSettings = {
		id: settings.id,
		dayStart: settings.dayStart,
		dayEnd: settings.dayEnd,
		lateAfter: settings.lateAfter,
		quarter: settings.quarter,
		q1Start: settings.q1Start,
		q1End: settings.q1End,
		q2Start: settings.q2Start,
		q2End: settings.q2End,
		q3Start: settings.q3Start,
		q3End: settings.q3End,
		q4Start: settings.q4Start,
		q4End: settings.q4End
	};
	return await invoke('save_settings', { settings: backendSettings });
}

// ── Export/Import Operations ─────────────────────────────────────────────────

export async function exportAll(): Promise<ExportData> {
	return await invoke('export_all');
}

export async function exportDatabase(): Promise<string> {
	return await invoke('export_database');
}

export async function exportJsonWithFolder(): Promise<string> {
	return await invoke('export_json_with_folder');
}

export async function exportCsvWithFolder(
	events: AttendanceEvent[],
	students: Student[],
	classes: Class[],
	globalLateAfter: string
): Promise<string> {
	return await invoke('export_csv_with_folder', {
		events,
		students,
		classes,
		globalLateAfter
	});
}

export async function exportDtrExcel(
	student: Student,
	classData: Class | undefined,
	events: AttendanceEvent[],
	month: number,
	year: number
): Promise<string> {
	return await invoke('export_dtr_excel', {
		student,
		class: classData,
		events,
		month,
		year
	});
}

export async function importAll(payload: ExportData): Promise<void> {
	return await invoke('import_all', { payload });
}

export async function wipeAll(): Promise<void> {
	return await invoke('wipe_all');
}

// ── Utility Functions ───────────────────────────────────────────────────────

export const uid = () => crypto.randomUUID();
