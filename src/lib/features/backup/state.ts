/**
 * Where the backup module remembers what it did last time.
 *
 * A port of `file_ops.rs`'s `BackupState` and `db-fingerprint.rs`'s sidecar: a
 * small JSON file beside the backups, written through the temp-file + rename
 * helper so a crash mid-write cannot leave the app with no record of its last
 * backup.
 *
 * It is a file and not a `settings` column because `settings` is a single
 * fixed-column row keyed by `id = 'app'`, and putting a key/value there needs a
 * schema change. The fingerprint module made the same argument for the same
 * reason.
 *
 * Google Drive and the sync folder are gone (spec D10), so `sync_folder_path`
 * and `last_sync_error` are gone with them.
 */

import { asAppError, internal } from '$lib/db';
import { getFileSystem } from '$lib/platform/fs';
import { getEesAmsRootDir } from './paths';

const STATE_FILE_NAME = 'backup-state.json';

/**
 * How many archives are kept before the oldest is pruned.
 *
 * The Rust default was 30. 20 is what phase 5's exit criteria were written
 * against, and an archive is now a zip holding a database dump plus the whole
 * workbook tree, so each one is larger than the flat `.db` file the 30 applied
 * to.
 */
export const DEFAULT_RETENTION_LIMIT = 20;

export type BackupState = {
	retentionLimit: number;
	/** Epoch seconds, matching what `BackupStatus` and the settings UI expect. */
	lastBackupAt?: number;
	lastBackupPath?: string;
	lastWorkbooksBackupPath?: string;
	/** The last failure, as the teacher reads it on the Data Management panel. */
	lastError?: string;
};

async function statePath(): Promise<string> {
	return `${await getEesAmsRootDir()}/${STATE_FILE_NAME}`;
}

export async function loadState(): Promise<BackupState> {
	const path = await statePath();
	if (!(await getFileSystem().exists(path))) return defaultState();
	try {
		const parsed: unknown = JSON.parse(await getFileSystem().readTextFile(path));
		return fromJson(parsed);
	} catch (thrown) {
		// A state file that cannot be read must not stop the app from starting or
		// from taking a backup. `getStatus` reports this as `lastError`, exactly
		// as the Rust version did with `BackupState { last_error, ..default }`.
		return {
			...defaultState(),
			lastError: `Failed to read backup settings: ${asAppError(thrown).detail}`
		};
	}
}

export async function saveState(state: BackupState): Promise<void> {
	const path = await statePath();
	try {
		await getFileSystem().writeFileAtomic(path, `${JSON.stringify(state, null, 2)}\n`);
	} catch (thrown) {
		throw internal(`failed to save backup state to ${path}: ${asAppError(thrown).detail}`);
	}
}

export async function getRetentionLimit(): Promise<number> {
	return (await loadState()).retentionLimit;
}

/**
 * Change how many archives are kept.
 *
 * The caller runs `pruneBackups()` afterwards: lowering the number in Settings
 * has to actually free the disk today rather than at the next backup, and the
 * two live in different modules so that `state.ts` stays a pure record of what
 * the app remembers.
 */
export async function setRetentionLimit(limit: number): Promise<void> {
	if (!Number.isInteger(limit) || limit < 1) {
		throw internal(`backup retention must be a whole number of at least 1, got ${limit}`);
	}
	const state = await loadState();
	await saveState({ ...state, retentionLimit: limit });
}

function defaultState(): BackupState {
	return { retentionLimit: DEFAULT_RETENTION_LIMIT };
}

/** Record a failure so the Data Management panel can show it. */
export async function recordBackupError(error: unknown): Promise<void> {
	const message = error instanceof Error ? error.message : asAppError(error).detail;
	await saveState({ ...(await loadState()), lastError: message });
}

function fromJson(parsed: unknown): BackupState {
	if (typeof parsed !== 'object' || parsed === null) return defaultState();
	const raw = parsed as Record<string, unknown>;
	const limit = raw.retentionLimit;
	return {
		retentionLimit:
			typeof limit === 'number' && Number.isInteger(limit) && limit >= 1
				? limit
				: DEFAULT_RETENTION_LIMIT,
		lastBackupAt: optionalNumber(raw.lastBackupAt),
		lastBackupPath: optionalString(raw.lastBackupPath),
		lastWorkbooksBackupPath: optionalString(raw.lastWorkbooksBackupPath),
		lastError: optionalString(raw.lastError)
	};
}

function optionalNumber(value: unknown): number | undefined {
	return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

function optionalString(value: unknown): string | undefined {
	return typeof value === 'string' && value !== '' ? value : undefined;
}
