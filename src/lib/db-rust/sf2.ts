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

/**
 * Switch the active SF2 report month for a class WITHOUT touching the Excel
 * workbook. Pure DB change so the reports page can switch months instantly and
 * avoid the slow Excel automation that `updateSf2WorkbookSettings` runs.
 */
export async function setSf2ReportMonth(classId: string, reportMonth: string): Promise<void> {
	await invoke('set_sf2_report_month', { classId, reportMonth });
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

/**
 * Mark ALL mapped students as present for the current report month.
 * Creates "in" events for every absent student on every day where
 * attendance was taken. Open days (no attendance taken) are left as-is.
 * Returns the number of attendance events created.
 */
export async function presentAllSf2PreviewAttendance(classId: string): Promise<number> {
	return await invoke('present_all_sf2_preview_attendance', { classId });
}

/**
 * Sync attendance to SF2 workbook AND open it in Excel, with real progress
 * events emitted from the Rust backend for a determinate progress bar.
 * The progression goes through 10 steps (1-10).
 * Listen for 'sf2-progress' events via `listen('sf2-progress', ...)` from
 * `@tauri-apps/api/event` to track progress on the frontend.
 */
export async function syncAndOpenSf2Workbook(classId: string): Promise<string> {
	return await invoke('sync_and_open_sf2_workbook', { classId });
}

/**
 * Kill all running EXCEL.EXE processes so orphaned background instances
 * don't prevent the SF2 workbook from opening. Returns the count of
 * terminated processes.
 */
export async function killAllExcelProcesses(): Promise<number> {
	return await invoke('kill_all_excel_processes');
}
