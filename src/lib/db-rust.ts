import { invoke } from '@tauri-apps/api/core';
import type {
	Student,
	StudentGender,
	Class,
	Session,
	AttendanceEvent,
	AttendanceAuditEntry,
	AuditEvent,
	AttendanceType,
	AttendanceMode,
	Settings,
	CreateEventRequest,
	UpdateEventRequest,
	ExportData,
	BackupSummary,
	BackupStatus,
	BackupPreview,
	RestoreResult,
	Sf2ImportSummary,
	Sf2TemplateDraft,
	Sf2WorkbookSettings,
	Sf2CloseDaySummary,
	Sf2ExportPreview,
	Sf2PreviewCell,
	Sf2PreviewStudentRow,
	Sf2ExportReadiness,
	Sf2ExportResult
} from './types';
export type {
	Student,
	StudentGender,
	Class,
	Session,
	AttendanceEvent,
	AttendanceAuditEntry,
	AuditEvent,
	AttendanceType,
	AttendanceMode,
	Settings,
	CreateEventRequest,
	UpdateEventRequest,
	ExportData,
	BackupSummary,
	BackupStatus,
	BackupPreview,
	RestoreResult,
	Sf2ImportSummary,
	Sf2TemplateDraft,
	Sf2WorkbookSettings,
	Sf2CloseDaySummary,
	Sf2ExportPreview,
	Sf2PreviewCell,
	Sf2PreviewStudentRow,
	Sf2ExportReadiness,
	Sf2ExportResult
};

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
				gender: student.gender,
				cardSerial: student.cardSerial,
				classId: student.classId
			}
		});
	} else {
		return await invoke('create_student', {
			req: {
				name: student.name,
				gender: student.gender,
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

export async function addEvent(event: CreateEventRequest): Promise<AttendanceEvent> {
	return await invoke('add_event', {
		req: event
	});
}

export async function updateEvent(id: string, req: UpdateEventRequest): Promise<AttendanceEvent> {
	return await invoke('update_event', { id, req });
}

export async function deleteEvent(id: string, reason?: string): Promise<void> {
	return await invoke('delete_event', { id, reason });
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

// ── Settings Operations ───────────────────────────────────────────────────────

export async function listAuditEvents(limit = 200): Promise<AuditEvent[]> {
	return await invoke('list_audit_events', { limit });
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
	// Backend already returns camelCase due to serde(rename_all = "camelCase")
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
	// Backend expects camelCase due to serde(rename_all = "camelCase")
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

export async function importAll(payload: ExportData): Promise<void> {
	return await invoke('import_all', { payload });
}

export async function wipeAll(): Promise<void> {
	return await invoke('wipe_all');
}

export async function getBackupStatus(): Promise<BackupStatus> {
	return await invoke('get_backup_status');
}

export async function createBackupNow(): Promise<BackupStatus> {
	return await invoke('create_backup_now');
}

export async function listBackups(): Promise<BackupSummary[]> {
	return await invoke('list_backups');
}

export async function chooseBackupSyncFolder(): Promise<BackupStatus> {
	return await invoke('choose_backup_sync_folder');
}

export async function clearBackupSyncFolder(): Promise<BackupStatus> {
	return await invoke('clear_backup_sync_folder');
}

export async function connectGoogleDriveBackup(): Promise<BackupStatus> {
	return await invoke('connect_google_drive_backup');
}

export async function disconnectGoogleDriveBackup(): Promise<BackupStatus> {
	return await invoke('disconnect_google_drive_backup');
}

export async function uploadLatestBackupToGoogleDrive(): Promise<BackupStatus> {
	return await invoke('upload_latest_backup_to_google_drive');
}

export async function chooseRestoreBackup(): Promise<BackupPreview | null> {
	return await invoke('choose_restore_backup');
}

export async function restoreBackup(sourcePath: string): Promise<RestoreResult> {
	return await invoke('restore_backup', { sourcePath });
}

// ── SF2 Excel Bridge Operations ─────────────────────────────────────────────

export async function importSf2Workbook(): Promise<Sf2ImportSummary> {
	return await invoke('import_sf2_workbook');
}

export async function createSf2WorkbookFromTemplate(
	draft: Sf2TemplateDraft
): Promise<Sf2ImportSummary> {
	return await invoke('create_sf2_workbook_from_template', { draft });
}

export async function getSf2WorkbookSettings(classId?: string): Promise<Sf2WorkbookSettings> {
	return await invoke('get_sf2_workbook_settings', { classId: classId || null });
}

export async function updateSf2WorkbookSettings(
	draft: Sf2TemplateDraft
): Promise<Sf2ImportSummary> {
	return await invoke('update_sf2_workbook_settings', { draft });
}

export async function closeSf2AttendanceDay(
	classId: string,
	date?: string
): Promise<Sf2CloseDaySummary> {
	return await invoke('close_sf2_attendance_day', { classId, date });
}

export async function getSf2ExportReadiness(classId?: string): Promise<Sf2ExportReadiness> {
	return await invoke('get_sf2_export_readiness', { classId: classId || null });
}

export async function getSf2ExportPreview(classId?: string): Promise<Sf2ExportPreview> {
	return await invoke('get_sf2_export_preview', { classId: classId || null });
}

export async function setSf2PreviewAttendance(
	classId: string,
	studentId: string,
	date: string,
	present: boolean
): Promise<Sf2ExportPreview> {
	return await invoke('set_sf2_preview_attendance', { classId, studentId, date, present });
}

export async function exportSf2Workbook(classId: string): Promise<Sf2ExportResult> {
	return await invoke('export_sf2_workbook', { classId });
}

export async function openSf2Workbook(classId?: string): Promise<string> {
	return await invoke('open_sf2_workbook', { classId: classId || null });
}

// ── Utility Functions ───────────────────────────────────────────────────────

export const uid = () => crypto.randomUUID();
