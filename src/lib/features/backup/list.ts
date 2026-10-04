/**
 * What is in the backup folder, and what one archive holds.
 *
 * A port of `file_ops.rs`'s listing half and `backup_ops.rs`'s status and
 * preview. Three things survived the port, and all three are about not lying to
 * the teacher:
 *
 *  - an archive whose manifest cannot be parsed is still listed, because it is
 *    still a snapshot they may need; it just reports what the file name and the
 *    file's own size can say.
 *  - `includesDatabase` is false for a workbooks-only archive, and that is the
 *    only thing the daily-backup check looks at, so a workbooks backup never
 *    counts as "today is already backed up".
 *  - the preview refuses an archive from a newer schema *before* anything is
 *    replaced. The Rust version allowed a newer database when snapshotting the
 *    live one — an archive this app just wrote cannot be newer than this app —
 *    and refused it on restore, where the check belongs.
 */

import { asAppError, invalidInput } from '$lib/db';
import { CURRENT_SCHEMA_VERSION } from '$lib/db/migrations';
import { getFileSystem } from '$lib/platform/fs';
import type { BackupPreview, BackupStatus, BackupSummary, BackupWorkbookPreview } from '$lib/types';
import { archiveText, readArchiveFile, type BackupArchive } from './archive';
import { MANIFEST_FILE_NAME, parseManifest, type BackupManifest } from './manifest';
import { baseName, getBackupDir } from './paths';
import { loadState } from './state';

/** `EES-AMS-20260928-142530.zip`. Sorts lexicographically = chronologically. */
const FILE_PREFIX = 'EES-AMS-';
const FILE_SUFFIX = '.zip';
const TIMESTAMP_PATTERN = /^\d{4}-\d{2}-\d{2}-\d{2}-\d{2}$/;

export function backupFileName(nowMs: number): string {
	return `${FILE_PREFIX}${formatTimestamp(nowMs)}${FILE_SUFFIX}`;
}

/** Every archive in the backup folder, newest first. */
export async function listBackups(): Promise<BackupSummary[]> {
	const dir = await getBackupDir();
	const fs = getFileSystem();
	const summaries: BackupSummary[] = [];
	for (const name of await fs.readDir(dir)) {
		if (!isBackupArchiveName(name)) continue;
		summaries.push(await summaryFromPath(`${dir}/${name}`));
	}
	return summaries.sort(
		(left, right) => right.createdAt - left.createdAt || (right.fileName < left.fileName ? -1 : 1)
	);
}

export async function getBackupStatus(): Promise<BackupStatus> {
	const [backups, state] = await Promise.all([listBackups(), loadState()]);
	const latest = backups.find((backup) => backup.includesDatabase) ?? backups[0];
	const workbooksBackup =
		state.lastWorkbooksBackupPath ??
		backups.find((backup) => backup.kind === 'manual_workbooks')?.path;

	return {
		localBackupDir: await getBackupDir(),
		backupCount: backups.length,
		retentionLimit: state.retentionLimit,
		lastBackupAt: state.lastBackupAt ?? latest?.createdAt,
		lastBackupPath: state.lastBackupPath ?? latest?.path,
		lastWorkbooksBackupPath: workbooksBackup,
		lastError: state.lastError
	};
}

/**
 * Inspect an archive without restoring it.
 *
 * Everything here comes out of the manifest, so a preview opens one zip and
 * touches no live data — which is what lets the UI show it before the teacher
 * commits to anything.
 */
export async function previewBackup(path: string): Promise<BackupPreview> {
	if (!(await getFileSystem().exists(path))) {
		throw invalidInput(`backup does not exist: ${path}`);
	}
	const fileName = baseName(path);
	const sizeBytes = await fileSize(path);
	const manifest = readManifest(await readArchiveFile(path));
	const warnings: string[] = [];

	if (!manifest) {
		warnings.push(
			`This archive has no ${MANIFEST_FILE_NAME}, so its app version, schema version and row counts are unknown.`
		);
	} else {
		if (manifest.schemaVersion > CURRENT_SCHEMA_VERSION) {
			throw invalidInput(
				`this backup is schema version ${manifest.schemaVersion}, newer than this app supports (${CURRENT_SCHEMA_VERSION}). Restore it with the newer version of the app.`
			);
		}
		if (manifest.schemaVersion < CURRENT_SCHEMA_VERSION) {
			warnings.push(
				`The backup will be migrated from schema version ${manifest.schemaVersion} to ${CURRENT_SCHEMA_VERSION} during restore.`
			);
		}
	}

	const workbookPreviews = manifestWorkbookPreviews(manifest);
	warnings.push(...workbookAbsenceNotes(manifest));
	const absentCount = manifest?.counts.absent ?? 0;
	const mismatch = workbookAbsenceWarning(workbookPreviews, absentCount);
	if (mismatch) warnings.push(mismatch);

	return {
		sourcePath: path,
		// There is no separate database file any more: the archive is what gets
		// opened, and the `db.sqlite` image inside it is what gets swapped in.
		databasePath: path,
		fileName,
		modifiedAt: Math.floor((await modifiedAt(path)) / 1000),
		sizeBytes,
		schemaVersion: manifest?.schemaVersion ?? 0,
		studentCount: manifest?.counts.students ?? 0,
		classCount: manifest?.counts.classes ?? 0,
		eventCount: manifest?.counts.events ?? 0,
		absentCount,
		settingsCount: manifest?.counts.settings ?? 0,
		sf2TemplateCount: manifest?.counts.sf2Templates ?? 0,
		includesDatabase: manifest?.includesDatabase ?? false,
		workbooks: workbookPreviews,
		warnings
	};
}

/**
 * The phrase every "this archive carries no workbooks" note shares, so the
 * restore path can tell whether the user has already been told without
 * re-deriving the message.
 */
export const NO_WORKBOOKS_MARKER = 'no SF2 workbooks';

/**
 * The critical pre-restore warning: a workbook in this archive holds more
 * absences than the database it is paired with.
 *
 * A restore does not merge the two — it makes the database match the archive
 * exactly — so afterwards the workbook is ahead of the database and the app
 * re-imports the difference the next time SF2 is opened. The user has to know
 * that before they click.
 */
export function workbookAbsenceWarning(
	workbooks: readonly BackupWorkbookPreview[],
	absentCount: number
): string | undefined {
	const expected = workbooks.reduce((most, workbook) => Math.max(most, workbook.xCount), 0);
	if (expected <= 0 || absentCount >= expected) return undefined;

	const names = workbooks
		.filter((workbook) => workbook.xCount === expected)
		.map((workbook) => workbook.fileName)
		.join(', ');
	return `This backup's workbooks record ${expected} X mark(s) but its database holds only ${absentCount} absence(s). Restoring pairs a database that is behind its own workbooks (${names}). The app re-imports the missing marks the next time you open SF2 — restore only if that is what you want.`;
}

/** Everything the user should be told about an archive's workbooks, before a restore runs. */
function workbookAbsenceNotes(manifest: BackupManifest | undefined): string[] {
	if (!manifest) return [];
	if (manifest.workbooks.length === 0) {
		return [
			`This backup holds ${NO_WORKBOOKS_MARKER}. Restoring it leaves the current SF2 workbooks untouched.`
		];
	}
	return [];
}

function manifestWorkbookPreviews(manifest: BackupManifest | undefined): BackupWorkbookPreview[] {
	return (manifest?.workbooks ?? []).map((entry) => ({
		fileName: baseName(entry.path),
		relativePath: entry.path,
		bytes: entry.bytes,
		xCount: entry.xCount
	}));
}

/** Read an archive's manifest, or `undefined` when it has none or cannot be read. */
function readManifest(archive: BackupArchive): BackupManifest | undefined {
	const text = archiveText(archive, MANIFEST_FILE_NAME);
	if (text === undefined) return undefined;
	try {
		return parseManifest(text);
	} catch (thrown) {
		// A manifest we cannot parse is a listing problem, not a restore problem:
		// `restoreBackup` reads it again and fails there, with the parse error.
		console.warn(`[backup] unreadable ${MANIFEST_FILE_NAME}: ${asAppError(thrown).detail}`);
		return undefined;
	}
}

function isBackupArchiveName(name: string): boolean {
	return name.startsWith(FILE_PREFIX) && name.endsWith(FILE_SUFFIX) && !name.endsWith('.tmp');
}

/** Unix seconds from `20260928-142530`, or `undefined` for a name that has none. */
function timestampFromFileName(fileName: string): number | undefined {
	const stem = fileName.slice(FILE_PREFIX.length, fileName.length - FILE_SUFFIX.length);
	// A uniquified name is `EES-AMS-<timestamp>-2.zip`; the suffix is not part of
	// the timestamp but the archive is still the second it claims to be.
	const stamp = TIMESTAMP_PATTERN.test(stem) ? stem : stem.split('-').slice(0, 5).join('-');
	const match = /^(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})$/.exec(stamp);
	if (!match) return undefined;
	const [, year, month, day, hour, minute, second] = match;
	const parsed = Date.parse(`${year}-${month}-${day}T${hour}:${minute}:${second}`);
	return Number.isNaN(parsed) ? undefined : Math.floor(parsed / 1000);
}

async function summaryFromPath(path: string): Promise<BackupSummary> {
	const fileName = baseName(path);
	const manifest = readManifest(await readArchiveFile(path));
	return {
		path,
		fileName,
		createdAt: manifest
			? Math.floor(Date.parse(manifest.createdAt) / 1000)
			: (timestampFromFileName(fileName) ?? 0),
		sizeBytes: await fileSize(path),
		kind: manifest?.kind ?? 'unknown',
		includesDatabase: manifest?.includesDatabase ?? false,
		workbookCount: manifest?.workbooks.length ?? 0,
		totalXCount: (manifest?.workbooks ?? []).reduce((total, entry) => total + entry.xCount, 0)
	};
}

async function fileSize(path: string): Promise<number> {
	try {
		return (await getFileSystem().stat(path)).size;
	} catch {
		return 0;
	}
}

async function modifiedAt(path: string): Promise<number> {
	try {
		return (await getFileSystem().stat(path)).modifiedAt;
	} catch {
		return 0;
	}
}

/** `20260928-142530`, in local time, so the name reads as the teacher's own clock. */
export function formatTimestamp(nowMs: number): string {
	const date = new Date(nowMs);
	const pad = (value: number): string => String(value).padStart(2, '0');
	return (
		`${date.getFullYear()}${pad(date.getMonth() + 1)}${pad(date.getDate())}` +
		`-${pad(date.getHours())}${pad(date.getMinutes())}${pad(date.getSeconds())}`
	);
}
