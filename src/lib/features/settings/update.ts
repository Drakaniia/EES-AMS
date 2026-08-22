import { invoke } from '@tauri-apps/api/core';

/**
 * Result of a manual update check.
 */
export interface UpdateInfo {
	available: boolean;
	version?: string | null;
	notes?: string | null;
	pubDate?: string | null;
	currentVersion: string;
	error?: string | null;
}

/**
 * Installed + staged update state. No network involved.
 */
export interface UpdateStatus {
	currentVersion: string;
	stagedVersion?: string | null;
	stagedNotes?: string | null;
	stagedPubDate?: string | null;
}

/**
 * Download progress emitted from Rust via the `update://progress` event.
 */
export interface UpdateProgress {
	downloaded: number;
	total?: number | null;
}

export async function checkForUpdates(): Promise<UpdateInfo> {
	return await invoke('check_for_updates');
}

export async function getUpdateStatus(): Promise<UpdateStatus> {
	return await invoke('get_update_status');
}

export async function downloadUpdate(): Promise<void> {
	await invoke('download_update');
}

export async function cancelUpdateDownload(): Promise<void> {
	await invoke('cancel_update_download');
}

export async function installStagedUpdate(): Promise<void> {
	await invoke('install_staged_update');
}

export async function openExternalUrl(url: string): Promise<void> {
	await invoke('open_external_url', { url });
}
