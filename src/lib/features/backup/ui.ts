/**
 * The one thing only a shell can do here: show the backup folder.
 *
 * Everything else in this module is pure TypeScript over the `FileSystem`
 * interface, which is exactly why the backup code is testable without Tauri.
 *
 * ## The restore picker is not here yet
 *
 * `$lib/api/backup.ts` had `chooseRestoreBackup()` and
 * `chooseRestoreDatabaseFile()`, both of which opened a Tauri file dialog. That
 * needs `@tauri-apps/plugin-dialog`, which phase 5 was specced against but which
 * is **not** in `package.json` — and `package.json` is outside this subsystem's
 * scope. Rather than hand-roll `invoke('plugin:dialog|open')` against a plugin
 * that may not be registered, this module stops at the boundary: the caller
 * supplies the archive path to `restoreBackup(path)` and everything from there
 * on is here. Adding the dependency is a one-line `package.json` change and the
 * two functions drop straight back in.
 */

import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
import { asAppError, internal } from '$lib/db';
import { getBackupDir } from './paths';

/** Open `Documents\EES-AMS\backups` in Explorer. Resolves to the path opened. */
export async function openBackupFolder(): Promise<string> {
	const dir = await getBackupDir();
	try {
		await openPath(dir);
		return dir;
	} catch (first) {
		// `openPath` on a directory depends on the OS file association; revealing
		// the folder inside its parent lands the teacher in the right place anyway.
		try {
			await revealItemInDir(dir);
			return dir;
		} catch (second) {
			throw internal(
				`failed to open ${dir}: ${asAppError(first).detail} / ${asAppError(second).detail}`
			);
		}
	}
}
