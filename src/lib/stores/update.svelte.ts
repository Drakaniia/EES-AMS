import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
	checkForUpdates,
	cancelUpdateDownload,
	downloadUpdate,
	getUpdateStatus,
	installStagedUpdate,
	type UpdateInfo,
	type UpdateProgress
} from '$lib/features/settings/update';

export type UpdateStatusKind =
	| 'unknown'
	| 'checking'
	| 'upToDate'
	| 'available'
	| 'downloading'
	| 'readyToRestart'
	| 'deferred'
	| 'failed';

export type UpdateFailedStage = 'check' | 'download' | 'install';

/** Minimum interval between manual refresh clicks. */
const REFRESH_COOLDOWN_MS = 10_000;

/**
 * Shared reactive update store (Svelte 5 runes).
 * Single source of truth for the app-start toast, the Settings card, and the
 * sidebar badge. All checks are single-flight and cached per session.
 */
class UpdateStore {
	status = $state<UpdateStatusKind>('unknown');
	updateInfo = $state<UpdateInfo | null>(null);
	currentVersion = $state('');
	stagedVersion = $state<string | null>(null);
	stagedNotes = $state<string | null>(null);
	stagedPubDate = $state<string | null>(null);
	progress = $state<UpdateProgress | null>(null);
	error = $state<string | null>(null);
	failedStage = $state<UpdateFailedStage | null>(null);

	hasStagedUpdate = $derived(this.stagedVersion !== null);
	/** Orange dot on the Settings nav item: update available OR staged. */
	badgeVisible = $derived(this.status === 'available' || this.hasStagedUpdate);

	private lastManualCheckAt = 0;
	private lastCheckPromise: Promise<UpdateInfo> | null = null;
	private progressUnlisten: UnlistenFn | null = null;

	constructor() {
		void this.wireProgressListener();
	}

	// ── Lifecycle ─────────────────────────────────────────────────────────────
	async init() {
		if (this.status !== 'unknown') return;
		try {
			const staged = await getUpdateStatus();
			this.currentVersion = staged.currentVersion;
			if (staged.stagedVersion) {
				this.stagedVersion = staged.stagedVersion;
				this.stagedNotes = staged.stagedNotes ?? null;
				this.stagedPubDate = staged.stagedPubDate ?? null;
				this.status = 'readyToRestart';
				return;
			}
		} catch (error) {
			console.error('Failed to read staged update status:', error);
		}
		await this.runCheck(false);
	}

	private async wireProgressListener() {
		try {
			this.progressUnlisten = await listen<UpdateProgress>('update://progress', (event) => {
				this.progress = event.payload;
			});
		} catch (error) {
			console.error('Failed to listen for update progress:', error);
		}
	}

	// ── Check ─────────────────────────────────────────────────────────────────
	/** Manual refresh: debounced (10 s) and joined to any in-flight check. */
	async refresh() {
		if (this.status === 'checking' || this.status === 'downloading') return;
		if (Date.now() - this.lastManualCheckAt < REFRESH_COOLDOWN_MS) return;
		await this.runCheck(true);
	}

	private async runCheck(recordManual: boolean): Promise<void> {
		if (this.status === 'downloading') return;
		if (this.lastCheckPromise) {
			try {
				await this.lastCheckPromise;
			} catch {
				// The original caller surfaces the error
			}
			return;
		}
		this.status = 'checking';
		this.error = null;
		const promise = checkForUpdates();
		this.lastCheckPromise = promise;
		try {
			const info = await promise;
			this.updateInfo = info;
			this.currentVersion = info.currentVersion;
			if (recordManual) this.lastManualCheckAt = Date.now();
			if (info.error) {
				this.status = 'failed';
				this.failedStage = 'check';
				this.error = info.error;
			} else if (info.available) {
				this.status = 'available';
				this.failedStage = null;
			} else {
				this.status = 'upToDate';
				this.failedStage = null;
				if (this.hasStagedUpdate) this.clearStaged();
			}
		} catch (error) {
			this.status = 'failed';
			this.failedStage = 'check';
			this.error = error instanceof Error ? error.message : String(error);
		} finally {
			this.lastCheckPromise = null;
		}
	}

	// ── Download ──────────────────────────────────────────────────────────────
	async download() {
		if (this.status === 'downloading') return;
		this.status = 'downloading';
		this.error = null;
		this.failedStage = null;
		this.progress = null;
		try {
			await downloadUpdate();
			this.status = 'readyToRestart';
			this.stagedVersion = this.updateInfo?.version ?? null;
			this.stagedNotes = this.updateInfo?.notes ?? null;
			this.stagedPubDate = this.updateInfo?.pubDate ?? null;
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			if (message.toLowerCase().includes('cancelled')) {
				// User cancelled: return to the available state, not an error
				this.status = 'available';
			} else {
				this.status = 'failed';
				this.failedStage = 'download';
				this.error = message;
			}
		}
	}

	async cancel() {
		if (this.status !== 'downloading') return;
		try {
			await cancelUpdateDownload();
		} catch (error) {
			console.error('Failed to cancel update download:', error);
		}
		// The pending download command resolves with "Download cancelled",
		// which flips the status back to `available`.
	}

	// ── Install ───────────────────────────────────────────────────────────────
	async restart() {
		if (!this.hasStagedUpdate) return;
		this.error = null;
		this.failedStage = null;
		try {
			await installStagedUpdate();
			// On Windows the process exits during install; this only runs on
			// platforms where install returns without exiting (dev builds).
			this.clearStaged();
			this.status = 'upToDate';
		} catch (error) {
			this.status = 'failed';
			this.failedStage = 'install';
			this.error = error instanceof Error ? error.message : String(error);
		}
	}

	/** Defer the restart: collapse to a subtle row; the staged update persists. */
	later() {
		if (this.status !== 'readyToRestart') return;
		this.status = 'deferred';
	}

	retry() {
		if (this.failedStage === 'download') void this.download();
		else if (this.failedStage === 'install') void this.restart();
		else void this.refresh();
	}

	private clearStaged() {
		this.stagedVersion = null;
		this.stagedNotes = null;
		this.stagedPubDate = null;
	}
}

// Export singleton instance
export const updateStore = new UpdateStore();
