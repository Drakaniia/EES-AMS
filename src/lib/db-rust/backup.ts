import { invoke } from '@tauri-apps/api/core';
import type {
	ExportData,
	BackupSummary,
	BackupStatus,
	BackupPreview,
	RestoreResult,
	AttendanceEvent,
	Student,
	Class
} from '../types';
export type {
	ExportData,
	BackupSummary,
	BackupStatus,
	BackupPreview,
	RestoreResult
} from '../types';

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
	return await invoke('export_csv_with_folder', { events, students, classes, globalLateAfter });
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

export async function openBackupFolder(): Promise<string> {
	return await invoke('open_backup_folder');
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
