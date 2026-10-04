import { getCurrentWindow } from '@tauri-apps/api/window';
import { useFileSystem } from '$lib/platform/fs';
import { TauriFileSystem } from '$lib/platform/tauri-fs';
import { registerSf2Preview } from '$lib/features/sf2/preview';
import { databaseStatus } from '$lib/stores/database-status.svelte';
import { onAppQuit, scheduleBackups, stopScheduledBackups } from '$lib/features/backup';

/**
 * One-time wiring for the TypeScript backend.
 *
 * Three of the four things below are not optional and not self-wiring, because
 * each one is an injection point that exists so the feature code stays testable
 * without a Tauri runtime:
 *
 * - `useFileSystem(new TauriFileSystem())` binds the real `FileSystem`. Until it
 *   runs, `getFileSystem()` throws - deliberately, so a missing binding is a
 *   startup error rather than a workbook write that silently goes nowhere.
 * - `registerSf2Preview()` hands the month service its grid builder. Until it
 *   runs, `getSf2MonthPreview()` throws rather than rendering an empty grid that
 *   reads as "nobody was ever marked absent".
 * - `scheduleBackups()` is the interval timer that replaces the Windows Task
 *   Scheduler entry (spec D11) - there is no other producer.
 * - `onAppQuit()` is the backup on quit (spec D11). It runs from Tauri's
 *   close-requested event, so the window is held open until the archive is
 *   written; otherwise the webview is torn down mid-write.
 *
 * Idempotent by construction: SvelteKit can re-mount a layout, and two listeners
 * on close-requested would mean two backups per quit.
 */

let started = false;

export async function bootstrapApp(): Promise<void> {
	if (started) return;
	started = true;

	useFileSystem(new TauriFileSystem());
	registerSf2Preview();
	scheduleBackups();
	databaseStatus.startAutoRetry();
	await bindQuitBackup();
}

/** Back up once as the window closes, then let it close. */
async function bindQuitBackup(): Promise<void> {
	try {
		const appWindow = getCurrentWindow();
		await appWindow.onCloseRequested(async (event) => {
			event.preventDefault();
			try {
				// No quit snapshot of throwaway state: it would pose as a real
				// backup in retention. Manual exports stay available in-session.
				if (databaseStatus.state !== 'temporary') await onAppQuit();
			} catch (error) {
				// A backup that cannot be written must not strand the teacher in a
				// window they cannot close. The failure is already recorded in the
				// backup state the Settings screen shows.
				console.warn('on-quit backup failed; closing anyway', error);
			}
			stopScheduledBackups();
			await appWindow.destroy();
		});
	} catch (error) {
		// No window, or the event is unavailable (tests, or a non-Tauri browser
		// preview). The interval timer still backs up while the app is open.
		console.warn('could not bind the on-quit backup', error);
	}
}
