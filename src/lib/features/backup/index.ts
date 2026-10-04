/**
 * Local backup and restore — migration spec D10 through D13.
 *
 * Google Drive is gone and nothing here knows about it: no OAuth, no keyring, no
 * `reqwest`, no token storage. What is left is a zip archive on disk under
 * `Documents\EES-AMS\backups`, a manifest that says what is inside it, a
 * retention limit, and an interval timer while the app is open.
 *
 * ## The two functions the UI drives
 *
 * | Call | Where |
 * | --- | --- |
 * | `scheduleBackups()` → disposer | app bootstrap, once |
 * | `onAppQuit()` | window close handler |
 *
 * ## The UI contract these replace
 *
 * `$lib/api/backup.ts` is the shape the settings page already speaks. Ten of
 * its exports are re-exported or replaced here and five disappear with Google
 * Drive and the sync folder.
 */

export {
	BACKUPS_FOLDER,
	EXPORTS_FOLDER,
	baseName,
	getBackupDir,
	getEesAmsRootDir,
	getExportsDir,
	getWorkbooksDir
} from './paths';

export {
	MANIFEST_FORMAT_VERSION,
	MANIFEST_FILE_NAME,
	DATABASE_FILE_NAME,
	WORKBOOK_PREFIX,
	parseManifest,
	toManifestJson,
	type BackupManifest,
	type ManifestCounts,
	type ManifestWorkbook
} from './manifest';

export {
	DEFAULT_RETENTION_LIMIT,
	getRetentionLimit,
	setRetentionLimit,
	type BackupState
} from './state';

export { createBackup, createWorkbooksBackup, pruneBackups } from './create';

export {
	NO_WORKBOOKS_MARKER,
	getBackupStatus,
	listBackups,
	previewBackup,
	workbookAbsenceWarning
} from './list';

export { restoreBackup } from './restore';

export {
	DEFAULT_BACKUP_INTERVAL_MS,
	ensureDailyBackup,
	isBackupScheduled,
	onAppQuit,
	scheduleBackups,
	stopScheduledBackups
} from './scheduling';

export { openBackupFolder } from './ui';

export { countXmarks } from './x-count';
