/**
 * Automatic backups.
 *
 * An interval timer that runs while the app is open, plus one backup as the app
 * quits. A `setInterval` in the page is the whole mechanism, and it stops for free
 * when the window closes (spec D11).
 *
 * The daily guard is kept, and it is what makes an hourly timer safe. Without it
 * an app left open over lunch writes twenty archives a day and retention quietly
 * throws away the good ones. Only an archive that actually carries the database
 * counts — a workbooks-only backup is not a backup of the attendance records.
 *
 * The bootstrap calls `scheduleBackups()` once at start-up and `onAppQuit()` from
 * the window close handler.
 */

import type { BackupSummary } from '$lib/types';
import { createBackup } from './create';
import { listBackups } from './list';
import { recordBackupError } from './state';

/** Hourly, matching the Rust `Duration::from_secs(60 * 60)`. */
export const DEFAULT_BACKUP_INTERVAL_MS = 60 * 60 * 1000;

let timer: ReturnType<typeof setInterval> | null = null;

/**
 * Start the timer. Returns the disposer, so a caller that wants to stop it does
 * not have to also call `stopScheduledBackups()`.
 */
export function scheduleBackups(intervalMs = DEFAULT_BACKUP_INTERVAL_MS): () => void {
	stopScheduledBackups();
	timer = setInterval(() => void ensureDailyBackup(), intervalMs);
	return stopScheduledBackups;
}

export function stopScheduledBackups(): void {
	if (timer === null) return;
	clearInterval(timer);
	timer = null;
}

export function isBackupScheduled(): boolean {
	return timer !== null;
}

/**
 * Take an automatic backup unless today already has one.
 *
 * Best-effort by design: a failed automatic backup is recorded in the state file
 * for the Data Management panel and never propagates, because the caller's only
 * other option would be an unhandled rejection in a timer.
 */
export async function ensureDailyBackup(nowMs = Date.now()): Promise<BackupSummary | undefined> {
	try {
		if (await hasDatabaseBackupToday(nowMs)) return undefined;
		return await createBackup('auto', nowMs);
	} catch (thrown) {
		await recordBackupError(thrown);
		console.warn(`[backup] automatic backup failed: ${describe(thrown)}`);
		return undefined;
	}
}

/**
 * The quit hook.
 *
 * Unconditional, unlike the timer's daily guard: this is the last snapshot
 * before the app closes, so it is worth writing even if today's timer's backup is
 * only an hour old. Retention bounds how many of these can pile up.
 */
export async function onAppQuit(nowMs = Date.now()): Promise<BackupSummary | undefined> {
	stopScheduledBackups();
	try {
		return await createBackup('auto', nowMs);
	} catch (thrown) {
		await recordBackupError(thrown);
		console.warn(`[backup] the quit backup failed: ${describe(thrown)}`);
		return undefined;
	}
}

async function hasDatabaseBackupToday(nowMs: number): Promise<boolean> {
	const day = new Date(nowMs).toDateString();
	return (await listBackups()).some((backup) => {
		if (!backup.includesDatabase) return false;
		return new Date(backup.createdAt * 1000).toDateString() === day;
	});
}

function describe(thrown: unknown): string {
	return thrown instanceof Error ? thrown.message : String(thrown);
}
