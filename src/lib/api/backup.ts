/**
 * `$lib/api/backup.ts` → `$lib/features/backup/**` + `$lib/db/repos/transfer`.
 *
 * ## What the migration spec changed (D10–D13)
 *
 * Google Drive is gone and the sync folder is gone. What is left is a zip archive
 * under `Documents\EES-AMS\backups`, a retention limit, and a database that
 * round-trips as a real `.sqlite` file image. The five Google Drive / sync-folder
 * commands are
 * *not* re-exported here — see the handover table for the call sites to delete.
 *
 * The exports the old bridge wrote to a save dialog now land in
 * `Documents\EES-AMS\exports` (D13), which is why each of these returns a path the
 * teacher can open rather than asking where to put it.
 */

import { createBackup, createWorkbooksBackup } from '$lib/features/backup/create';
import { getBackupStatus, listBackups, previewBackup } from '$lib/features/backup/list';
import { getExportsDir } from '$lib/features/backup/paths';
import { restoreBackup as restoreArchive } from '$lib/features/backup/restore';
import { openBackupFolder } from '$lib/features/backup/ui';
import { buildAttendanceCsv } from '$lib/features/settings/csv';
import {
	exportAll as collectExport,
	exportCounts,
	importAll as importSnapshot,
	recordDataExportAudit,
	toPrettyJson,
	wipeAll as wipeTables
} from '$lib/db/repos/transfer';
import { dumpDatabase } from '$lib/features/backup/database';
import { getFileSystem } from '$lib/platform/fs';
import { discardStagedRestore, stageRestoreSource } from './restore-staging';
import type {
	AttendanceEvent,
	BackupPreview,
	BackupStatus,
	Class,
	ExportData,
	RestoreResult,
	Student,
	WipeOutcome
} from '$lib/types';

export { getBackupStatus, listBackups, openBackupFolder };

export type {
	BackupKind,
	BackupPreview,
	BackupStatus,
	BackupWorkbookPreview,
	ExportData,
	RestoreResult,
	WipeOutcome
} from '$lib/types';

// â”€â”€ Snapshot, import, wipe â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

export async function exportAll(): Promise<ExportData> {
	return await collectExport();
}

/**
 * The old bridge's `importAll(payload: ExportData)`, whose `settings` are the
 * narrowed wire type.
 */
export async function importAll(payload: ExportData): Promise<void> {
	await importSnapshot(payload);
}

/**
 * The one irreversible action in the app, so the safety copy comes first and its
 * path goes into the outcome. A backup that *cannot* be written is not a reason to
 * refuse the wipe — the teacher asked for it — so the copy is best-effort and a
 * `null` path is the honest report.
 */
export async function wipeAll(): Promise<WipeOutcome> {
	let preWipeBackupPath: string | null;
	try {
		preWipeBackupPath = (await createBackup('pre_wipe')).path;
	} catch {
		preWipeBackupPath = null;
	}
	return await wipeTables(preWipeBackupPath);
}

// â”€â”€ Exports to `Documents\EES-AMS\exports` â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/**
 * `attendance-<n>.csv`, `ees-ams-<n>.json`, `attendance-<n>.sql`.
 *
 * A unix stamp rather than a formatted date: it sorts correctly in Explorer, and
 * a file name is not worth a locale dependency.
 */
function stampedName(prefix: string, extension: string): string {
	return `${prefix}-${Date.now()}.${extension}`;
}

async function writeExport(name: string, contents: string): Promise<string> {
	const path = `${await getExportsDir()}/${name}`;
	await getFileSystem().writeFileAtomic(path, contents);
	return path;
}

/**
 * The whole database as SQL text.
 *
 * Text is the point here, not the constraint it once was: the worker can hand
 * out a real `.sqlite` image (and a backup archive's `db.sqlite` is one), but
 * this export is the readable one — `INSERT` per row, diffable, pasteable into
 * any SQLite client — for looking at what the database holds without opening
 * the app.
 */
export async function exportDatabase(): Promise<string> {
	const dump = await dumpDatabase();
	await recordDataExportAudit({ format: 'sql', bytes: dump.length }, 'Exported SQL dump');
	return await writeExport(stampedName('attendance', 'sql'), dump);
}

export async function exportJsonWithFolder(): Promise<string> {
	const snapshot = await collectExport();
	const path = await writeExport(stampedName('ees-ams', 'json'), toPrettyJson(snapshot));
	await recordDataExportAudit({ ...exportCounts(snapshot), path });
	return path;
}

export async function exportCsvWithFolder(
	events: AttendanceEvent[],
	students: Student[],
	classes: Class[],
	globalLateAfter: string
): Promise<string> {
	const csv = buildAttendanceCsv(events, students, classes, globalLateAfter);
	const path = await writeExport(stampedName('attendance', 'csv'), csv);
	await recordDataExportAudit({ format: 'csv', rows: events.length, path });
	return path;
}

// â”€â”€ Snapshots and restore â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

export async function createBackupNow(): Promise<BackupStatus> {
	await createBackup('manual');
	return await getBackupStatus();
}

/**
 * "Back up workbooks now": copies the SF2 workbooks into their own backup folder
 * without duplicating the database.
 */
export async function createWorkbooksBackupNow(): Promise<BackupStatus> {
	await createWorkbooksBackup();
	return await getBackupStatus();
}

/**
 * Pick a backup archive to restore. `null` when the picker is dismissed.
 *
 * `./pickers` is pulled in lazily because it reaches for `@tauri-apps/plugin-dialog`,
 * which only exists inside a shell: nothing else in this module should pay for it.
 *
 * The picked path is staged inside the app's own folder before anything reads it.
 * The fs capability is a static allow-list and cannot reach a path the teacher chose
 * at runtime, so previewing — and therefore restoring — the file where it sits is a
 * permission error. `restore-staging.ts` has the why; this is the one line that
 * makes the preview's `sourcePath` an in-scope path.
 */
export async function chooseRestoreBackup(): Promise<BackupPreview | null> {
	const { pickBackupArchive } = await import('./pickers');
	const picked = await pickBackupArchive();
	return picked === null ? null : await previewBackup(await stageRestoreSource(picked));
}

/**
 * Restore, then drop the staged copy.
 *
 * The cleanup is keyed off the staging folder, so restoring an archive that was
 * never staged — a backup out of `backups\`, which `listBackups` offers directly —
 * leaves it exactly where it was.
 */
export async function restoreBackup(sourcePath: string): Promise<RestoreResult> {
	const result = await restoreArchive(sourcePath);
	await discardStagedRestore(sourcePath);
	return result;
}
