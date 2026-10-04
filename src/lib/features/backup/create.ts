/**
 * Writing a backup, and pruning the old ones.
 *
 * A port of `backup_ops.rs`'s `create_backup_at`, `create_workbooks_backup` and
 * `enforce_retention`. Two decisions are worth stating because they are the ones
 * the Rust version got wrong for its own shape:
 *
 *  - The archive is written **once, directly, through the atomic write helper**,
 *    not assembled in a `.tmp` sibling and renamed. `writeFileAtomic` already
 *    writes a sibling temp file and renames over the target, and it is the same
 *    guarantee D15 settled on for workbooks. The Rust `.tmp` sibling, its stale-
 *    temp sweep and its 60-minute `STALE_TEMP_AGE` window all existed to do what
 *    one helper does.
 *  - The summary is taken **before** pruning. A caller that asked for a backup
 *    must never be handed a path that retention has just deleted.
 */

import { CURRENT_SCHEMA_VERSION } from '$lib/db/migrations';
import { getFileSystem } from '$lib/platform/fs';
import type { BackupKind, BackupSummary } from '$lib/types';
import { buildArchive, type ArchiveFile } from './archive';
import { readDatabaseCounts, exportDatabaseImage } from './database';
import { backupFileName, listBackups } from './list';
import {
	DATABASE_FILE_NAME,
	MANIFEST_FILE_NAME,
	MANIFEST_FORMAT_VERSION,
	toManifestJson,
	type BackupManifest
} from './manifest';
import { getBackupDir } from './paths';
import { loadState, saveState } from './state';
import { collectWorkbooks } from './workbooks';

/**
 * A full snapshot: the database file image, the SF2 workbooks, and the manifest.
 *
 * `nowMs` is a parameter rather than a clock read inside, so a test can write
 * two archives a second apart and assert on retention instead of on how long the
 * test took.
 */
export async function createBackup(kind: BackupKind, nowMs = Date.now()): Promise<BackupSummary> {
	const files: ArchiveFile[] = [{ path: DATABASE_FILE_NAME, bytes: await exportDatabaseImage() }];
	const workbooks = await collectWorkbooks();
	files.push(...workbooks.files);

	const manifest: BackupManifest = {
		formatVersion: MANIFEST_FORMAT_VERSION,
		appVersion: await readAppVersion(),
		schemaVersion: CURRENT_SCHEMA_VERSION,
		createdAt: new Date(nowMs).toISOString(),
		kind,
		includesDatabase: true,
		counts: await readDatabaseCounts(),
		workbooks: workbooks.entries
	};
	return writeArchiveFile(files, manifest, nowMs, (state, summary) => ({
		...state,
		lastBackupAt: summary.createdAt,
		lastBackupPath: summary.path,
		lastError: undefined
	}));
}

/**
 * The "Back up workbooks now" archive.
 *
 * It deliberately carries **no** database. The database has its own backups, and
 * a second copy here would be free to drift from the first, which is exactly the
 * failure the manifest's counts exist to detect.
 */
export async function createWorkbooksBackup(nowMs = Date.now()): Promise<BackupSummary> {
	const workbooks = await collectWorkbooks();
	const manifest: BackupManifest = {
		formatVersion: MANIFEST_FORMAT_VERSION,
		appVersion: await readAppVersion(),
		schemaVersion: CURRENT_SCHEMA_VERSION,
		createdAt: new Date(nowMs).toISOString(),
		kind: 'manual_workbooks',
		includesDatabase: false,
		counts: await readDatabaseCounts(),
		workbooks: workbooks.entries
	};
	return writeArchiveFile(workbooks.files, manifest, nowMs, (state, summary) => ({
		...state,
		lastWorkbooksBackupPath: summary.path,
		lastError: undefined
	}));
}

/**
 * Delete the oldest archives beyond the retention limit.
 *
 * Retention counts *entries*, and an entry is one archive whatever it holds. A
 * disk-budget cap would be worth adding — an archive is now a zip with a whole
 * workbook tree in it — but the count is what the settings page reports.
 */
export async function pruneBackups(): Promise<string[]> {
	const [backups, state] = await Promise.all([listBackups(), loadState()]);
	const stale = backups.slice(state.retentionLimit);
	for (const backup of stale) await getFileSystem().remove(backup.path);
	return stale.map((backup) => backup.path);
}

async function writeArchiveFile(
	files: readonly ArchiveFile[],
	manifest: BackupManifest,
	nowMs: number,
	update: (
		state: Awaited<ReturnType<typeof loadState>>,
		summary: BackupSummary
	) => Awaited<ReturnType<typeof loadState>>
): Promise<BackupSummary> {
	const dir = await getBackupDir();
	const path = await uniquePath(dir, backupFileName(nowMs));
	await getFileSystem().writeFileAtomic(
		path,
		buildArchive([
			...files,
			{ path: MANIFEST_FILE_NAME, bytes: new TextEncoder().encode(toManifestJson(manifest)) }
		])
	);

	const summary: BackupSummary = {
		path,
		fileName: path.slice(path.lastIndexOf('/') + 1),
		createdAt: Math.floor(Date.parse(manifest.createdAt) / 1000),
		sizeBytes: (await getFileSystem().stat(path)).size,
		kind: manifest.kind,
		includesDatabase: manifest.includesDatabase,
		workbookCount: manifest.workbooks.length,
		totalXCount: manifest.workbooks.reduce((total, entry) => total + entry.xCount, 0)
	};

	await pruneBackups();
	await saveState(update(await loadState(), summary));
	return summary;
}

/**
 * `EES-AMS-<timestamp>.zip`, with a `-2`, `-3`, … suffix if one already exists.
 *
 * Two backups can land in the same second — the hourly timer and a "Back up
 * now" click, or the timer and the quit hook — and overwriting the first would
 * destroy the snapshot it just took.
 */
async function uniquePath(dir: string, fileName: string): Promise<string> {
	const fs = getFileSystem();
	let candidate = `${dir}/${fileName}`;
	for (let suffix = 2; await fs.exists(candidate); suffix += 1) {
		candidate = `${dir}/${fileName.replace(/\.zip$/, '')}-${suffix}.zip`;
	}
	return candidate;
}

/**
 * The app version, for the manifest.
 *
 * Absent in a test and in a browser preview, and that must not fail a backup —
 * the manifest's version field is provenance, and the schema version next to it
 * is what a restore actually checks.
 */
async function readAppVersion(): Promise<string> {
	try {
		const { getVersion } = await import('@tauri-apps/api/app');
		return await getVersion();
	} catch {
		return 'unknown';
	}
}
