import { invoke } from '@tauri-apps/api/core';
import type {
	Sf2ImportSummary,
	Sf2ImportValidation,
	Sf2TemplateDraft,
	Sf2WorkbookSettings,
	Sf2ExportPreview,
	Sf2ExportReadiness,
	Sf2ExportResult
} from '../types';
export type {
	Sf2ImportSummary,
	Sf2ImportValidation,
	Sf2TemplateDraft,
	Sf2WorkbookSettings,
	Sf2ExportPreview,
	Sf2PreviewCell,
	Sf2PreviewStudentRow,
	Sf2ExportReadiness,
	Sf2ExportResult,
	Sf2ValidationDuplicate,
	Sf2ValidationLearner,
	Sf2ValidationStudent,
	Sf2CloseDaySummary
} from '../types';

export async function validateSf2WorkbookImport(): Promise<Sf2ImportValidation> {
	return await invoke('validate_sf2_workbook_import');
}

export async function importSf2Workbook(
	sourcePath: string,
	proceedAnyway: boolean
): Promise<Sf2ImportSummary> {
	return await invoke('import_sf2_workbook', { sourcePath, proceedAnyway });
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

export async function getSf2ExportReadiness(classId?: string): Promise<Sf2ExportReadiness> {
	return await invoke('get_sf2_export_readiness', { classId: classId || null });
}

export async function getSf2ExportPreview(classId?: string): Promise<Sf2ExportPreview> {
	return await invoke('get_sf2_export_preview', { classId: classId || null });
}

/** Sync the latest attendance events from the DB to the SF2 Excel working copy. */
export async function syncSf2Attendance(classId: string): Promise<void> {
	await invoke('sync_sf2_attendance', { classId });
}

/** Sync the class roster to the SF2 working workbook.
 *  For bundled templates this re-assigns all students to available row slots.
 *  For imported workbooks this returns a clear explanation. */
export async function syncSf2Roster(classId: string): Promise<void> {
	await invoke('sync_sf2_roster', { classId });
}

/** Lightweight toggle — only writes the DB event, no Excel I/O or preview rebuild. */
export async function toggleSf2PreviewAttendance(
	classId: string,
	studentId: string,
	date: string,
	present: boolean
): Promise<void> {
	await invoke('toggle_sf2_preview_attendance', { classId, studentId, date, present });
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
