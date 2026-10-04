/**
 * Putting a backup back.
 *
 * A port of `restore_service.rs`, and the one place in this app that overwrites a
 * teacher's live database and workbooks. The ordering is the Rust ordering and
 * it is not arbitrary:
 *
 *  1. Read and validate the archive, before anything is touched. An archive this
 *     app cannot read is refused up front rather than half-applied.
 *  2. Take a **pre-restore** backup of the current state. It is a full archive,
 *     database and workbooks both, so a restore can never destroy the only copy
 *     of an X mark.
 *  3. Replace the database, then run the migration chain over it.
 *  4. Write the workbooks back *after* the database, so a workbook write that
 *     fails leaves a consistent database rather than a half-restored pair.
 *
 * Step 4's failure is a warning, not an error. The database restore itself
 * succeeded and the safety archive still holds everything, so failing the whole
 * call would tell the teacher their restore failed when it did not.
 */

import { asAppError, invalidInput } from '$lib/db';
import { CURRENT_SCHEMA_VERSION } from '$lib/db/migrations';
import type { RestoreResult } from '$lib/types';
import { archiveFile, archiveText, readArchiveFile, type BackupArchive } from './archive';
import { createBackup } from './create';
import { readSchemaVersion, replaceDatabaseFromImage } from './database';
import { NO_WORKBOOKS_MARKER, previewBackup, workbookAbsenceWarning } from './list';
import { DATABASE_FILE_NAME, MANIFEST_FILE_NAME, parseManifest, WORKBOOK_PREFIX } from './manifest';
import { writeWorkbooks } from './workbooks';

export async function restoreBackup(sourcePath: string): Promise<RestoreResult> {
	const archive = await readArchiveFile(sourcePath);
	const manifestText = archiveText(archive, MANIFEST_FILE_NAME);
	if (manifestText === undefined) {
		throw invalidInput(`this backup has no ${MANIFEST_FILE_NAME}, so it cannot be restored safely`);
	}
	// Parsed first, and it throws on anything this build cannot honestly restore,
	// so nothing is written before the archive is known to be good.
	const manifest = parseManifest(manifestText);

	const preview = await previewBackup(sourcePath);
	const warnings = [...preview.warnings];
	// Re-checked here as well as in the preview: the preview is what the user saw
	// and confirmed, and this is the last point before anything is replaced.
	const mismatch = workbookAbsenceWarning(preview.workbooks, preview.absentCount);
	if (mismatch && !warnings.includes(mismatch)) warnings.push(mismatch);

	const safety = await createBackup('pre_restore');

	let schemaVersion = await readSchemaVersion();
	if (manifest.includesDatabase) {
		const image = archiveFile(archive, DATABASE_FILE_NAME);
		if (image === undefined) {
			throw invalidInput(
				`this backup claims to carry a database but has no ${DATABASE_FILE_NAME} inside it`
			);
		}
		schemaVersion = await replaceDatabaseFromImage(image);
	}

	const restored = await writeBackWorkbooks(archive, safety.path, warnings);

	return {
		restoredPath: sourcePath,
		preRestoreBackupPath: safety.path,
		restoredAt: Math.floor(Date.now() / 1000),
		schemaVersion,
		migrated: manifest.schemaVersion < CURRENT_SCHEMA_VERSION,
		workbooksRestored: restored,
		warnings
	};
}

async function writeBackWorkbooks(
	archive: BackupArchive,
	safetyPath: string,
	warnings: string[]
): Promise<boolean> {
	const files = archive.files.filter((file) => file.path.startsWith(WORKBOOK_PREFIX));
	if (files.length > 0) {
		try {
			if ((await writeWorkbooks(files)) > 0) return true;
		} catch (thrown) {
			warnings.push(
				`The database was restored but the SF2 workbooks could not be written back: ${asAppError(thrown).detail}. The previous workbooks are unchanged, and the safety backup at ${safetyPath} holds both.`
			);
			return false;
		}
	}
	warnings.push(
		`This backup held ${NO_WORKBOOKS_MARKER}, so the current workbooks were left in place.`
	);
	return false;
}
