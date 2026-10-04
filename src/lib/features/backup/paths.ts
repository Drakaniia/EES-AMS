/**
 * Where every file this app writes lives, resolved in exactly one place.
 *
 * Migration spec D13 moved the app's output out of the app data directory and
 * into `Documents\EES-AMS\`, because a teacher has to be able to find their
 * workbooks and their backups without an explorer session and an app id. That
 * makes the root path a shared contract rather than a per-module detail: the
 * backup folder, the workbook folder and the export folder all hang off it.
 *
 * The root is derived from `getSf2WorkbookDir()` rather than from
 * `documentDir()` + `SF2_ROOT_FOLDER` a second time. Two independent
 * derivations of one directory is the bug this file exists to prevent: the
 * workbook folder had to carry a comment explaining why its constant was equal
 * to a literal in another module, and it drifted.
 */

import { getSf2WorkbookDir } from '$lib/features/sf2/workbook-files';
import { getFileSystem } from '$lib/platform/fs';

/** Sub-folder of the root that holds the zip snapshots. */
export const BACKUPS_FOLDER = 'backups';

/** Sub-folder of the root that holds CSV / JSON / workbook exports. */
export const EXPORTS_FOLDER = 'exports';

/**
 * `Documents\EES-AMS` — the parent of the workbook directory.
 *
 * The workbook directory is the one folder the SF2 code already resolves, so
 * taking its parent keeps the backup folder and the workbook folder siblings
 * rather than two copies of the same idea.
 */
export async function getEesAmsRootDir(): Promise<string> {
	const workbookDir = await getSf2WorkbookDir();
	const separator = workbookDir.lastIndexOf('/');
	return separator > 0 ? workbookDir.slice(0, separator) : workbookDir;
}

/** `Documents\EES-AMS\backups`, created if it is not there yet. */
export async function getBackupDir(): Promise<string> {
	return createDir(`${await getEesAmsRootDir()}/${BACKUPS_FOLDER}`);
}

/**
 * `Documents\EES-AMS\workbooks`.
 *
 * Delegated rather than rebuilt so a backup can never be written beside a
 * *different* workbooks directory than the one the SF2 code reads.
 */
export async function getWorkbooksDir(): Promise<string> {
	return getSf2WorkbookDir();
}

/** `Documents\EES-AMS\exports`, created if it is not there yet. */
export async function getExportsDir(): Promise<string> {
	return createDir(`${await getEesAmsRootDir()}/${EXPORTS_FOLDER}`);
}

async function createDir(path: string): Promise<string> {
	await getFileSystem().mkdirp(path);
	return path;
}

/** The last path segment, or the whole path when it has no separator. */
export function baseName(path: string): string {
	const separator = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
	return separator < 0 ? path : path.slice(separator + 1);
}
