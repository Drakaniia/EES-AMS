import { getVersion } from '@tauri-apps/api/app';
import { emit } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
import { check, type Update } from '@tauri-apps/plugin-updater';

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
interface UpdateStatus {
	currentVersion: string;
	stagedVersion?: string | null;
	stagedNotes?: string | null;
	stagedPubDate?: string | null;
	/**
	 * Set when the attendance record count fell between the previously installed
	 * version and this one. The message names the pre-install backup to restore
	 * from. Loud on purpose: silent data loss is the failure this guards against.
	 */
	attendanceWarning?: string | null;
}

/**
 * Download progress, emitted on the `update://progress` event.
 */
export interface UpdateProgress {
	downloaded: number;
	total?: number | null;
}

/**
 * An update whose installer has been downloaded and is waiting to be installed.
 *
 * The updater plugin keeps the installer bytes in the Rust process, keyed by the
 * update resource id, so the only thing this session has to remember is the
 * handle. That is also the hard limit of the plugin: the bytes cannot be handed
 * to TypeScript, so a staged update does not survive an app restart the way the
 * old `staged-update.json` marker in the app cache dir did.
 */
interface StagedUpdate {
	update: Update;
	version: string;
	notes: string | null;
	pubDate: string | null;
}

let staged: StagedUpdate | null = null;
let cancelRequested = false;
let installing = false;

function describe(thrown: unknown): string {
	return thrown instanceof Error ? thrown.message : String(thrown);
}

/**
 * The Rust command returned this shape rather than failing, so the Settings
 * panel can tell "up to date" apart from "the check itself did not work".
 */
function noUpdate(currentVersion: string, error: string | null): UpdateInfo {
	return { available: false, version: null, notes: null, pubDate: null, currentVersion, error };
}

export async function checkForUpdates(): Promise<UpdateInfo> {
	const currentVersion = await getVersion();
	let update: Update | null;
	try {
		update = await check();
	} catch (error) {
		return noUpdate(currentVersion, `Could not reach the update server: ${describe(error)}`);
	}
	if (!update) return noUpdate(currentVersion, null);
	const info: UpdateInfo = {
		available: true,
		version: update.version,
		notes: update.body ?? null,
		pubDate: update.date ?? null,
		currentVersion,
		error: null
	};
	// The handle is of no use to the caller because `downloadUpdate` runs its own
	// check, exactly as the Rust command did. Release it rather than leak it, and
	// never let a failed release turn a good check into a reported failure.
	await update.close().catch(() => undefined);
	return info;
}

/**
 * Installed version plus the staged download, if this session made one.
 *
 * Reads no network and no files: with the plugin the staged state *is* the
 * in-memory handle, so a fresh launch honestly reports nothing staged rather
 * than promising an install it cannot perform.
 */
export async function getUpdateStatus(): Promise<UpdateStatus> {
	return {
		currentVersion: await getVersion(),
		stagedVersion: staged?.version ?? null,
		stagedNotes: staged?.notes ?? null,
		stagedPubDate: staged?.pubDate ?? null,
		// Always null: this warning was produced by the Rust `backup::fingerprint`
		// sidecar, which the TypeScript migration has not ported yet (the spec puts
		// it in `$lib/features/backup/**`, not here). The field stays so the panel
		// keeps its branch and the store keeps its type.
		attendanceWarning: null
	};
}

/**
 * Downloads the pending update, emitting `update://progress` so the panel can
 * draw the bar, and holds the handle for `installStagedUpdate`.
 */
export async function downloadUpdate(): Promise<void> {
	cancelRequested = false;
	let update: Update | null;
	try {
		update = await check();
	} catch (error) {
		throw new Error(`Update check failed: ${describe(error)}`, { cause: error });
	}
	if (!update) throw new Error('No update available');

	let downloaded = 0;
	let total: number | null = null;
	try {
		await update.download((event) => {
			if (event.event === 'Started') {
				total = event.data.contentLength ?? null;
			} else if (event.event === 'Progress') {
				downloaded += event.data.chunkLength;
			}
			// A panel that is not listening must not abort a download that is fine,
			// so the emit failure is dropped rather than thrown into the callback.
			void emit('update://progress', { downloaded, total } satisfies UpdateProgress).catch(
				() => undefined
			);
		});
	} catch (error) {
		await update.close().catch(() => undefined);
		throw new Error(`Download failed: ${describe(error)}`, { cause: error });
	}

	if (cancelRequested) {
		// ponytail: the plugin has no way to abort an in-flight download, so a
		// cancelled one runs to completion and is then thrown away. The bytes are
		// dropped rather than the cancel being faked; a real abort needs an abort
		// handle in the plugin's `download` command.
		await update.close().catch(() => undefined);
		throw new Error('Download cancelled');
	}

	staged = {
		update,
		version: update.version,
		notes: update.body ?? null,
		pubDate: update.date ?? null
	};
}

/** Asks the running download to be discarded. See `downloadUpdate` for the catch. */
export async function cancelUpdateDownload(): Promise<void> {
	cancelRequested = true;
}

/**
 * Installs the staged update. On Windows the installer is launched and the app
 * process exits; the NSIS installer relaunches the app after installing.
 */
export async function installStagedUpdate(): Promise<void> {
	if (installing) throw new Error('An update install is already in progress');
	installing = true;
	try {
		await installStaged();
	} catch (error) {
		// Only a failure clears the latch: a successful install exits the process on
		// Windows, and on the platforms where it does not, one install per session is
		// what the Rust command allowed too.
		installing = false;
		throw error;
	}
	staged = null;
}

async function installStaged(): Promise<void> {
	const pending = staged;
	// Same wording as the Rust command's "No staged update found". Its
	// restart-recovery branch — re-check the server and install the bytes from
	// `staged-update.json` — cannot be ported: the plugin holds the installer in the
	// Rust process, so there is nothing on disk to come back to.
	if (!pending) throw new Error('No staged update found');

	// Imported lazily so the backup stack (zip, database dump, workbooks) is not
	// pulled into app startup for a path that runs once, after a download.
	const { createBackup } = await import('$lib/features/backup/create');
	try {
		await createBackup('pre_install');
	} catch (error) {
		throw new Error(`Pre-install backup failed: ${describe(error)}`, { cause: error });
	}

	try {
		await pending.update.install();
	} catch (error) {
		throw new Error(`Install failed: ${describe(error)}`, { cause: error });
	}
}

/** Opens a URL in the system browser (used for release notes links). */
export async function openExternalUrl(url: string): Promise<void> {
	try {
		await openUrl(url);
	} catch (error) {
		throw new Error(`Failed to open link: ${describe(error)}`, { cause: error });
	}
}
