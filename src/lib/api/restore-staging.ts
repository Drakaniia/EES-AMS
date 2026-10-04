/**
 * Staging a backup archive the teacher picked from somewhere the app cannot read.
 *
 * ## Why the picked path cannot be used directly
 *
 * The fs capability in `src-tauri/capabilities/default.json` is static: it allows
 * `$DOCUMENT/EES-AMS`, `$DOCUMENT/EES-AMS/**`, `$APPDATA` and `$APPDATA/**`, and
 * nothing else. A backup on a USB stick, a second drive or any other folder the
 * file dialog can reach is outside it, so `getFileSystem().readFile(picked)` failed
 * with a permission error at the first read and the restore died there.
 *
 * `tauri-plugin-fs` v2 has no runtime "allow this path" call —
 * `startAccessingSecurityScopedResource` is iOS-only and a no-op on Windows — so no
 * capability edit can reach a path the user chose at runtime. Copying the archive
 * somewhere the allow-list already covers is the only thing the webview can do, and
 * it is what this module does.
 *
 * ## Why a folder of its own, and why the copy is renamed
 *
 * `restore-staging` sits beside `backups`, `workbooks` and `exports` rather than
 * inside one of them: `listBackups()` only reads `backups/`, so a staged copy can
 * never show up in the backup list as if it were one of the teacher's own archives.
 * The staged name is prefixed `restore-` and keeps the original base name, so it is
 * both unmistakably transient in a file listing and still recognisable in the
 * confirm dialog, which shows `preview.fileName`.
 *
 * The copy is written with `writeFileAtomic`, so the target path never exists in a
 * half-written state — a teacher cannot pick a partial archive even by hand.
 *
 * ## Crash leftovers: swept, not recovered
 *
 * A staged path is reachable only through the `BackupPreview` held in memory by the
 * restore dialog. A crash drops that reference, and after a restart the file is a
 * byte-for-byte duplicate of something the teacher still has wherever they picked
 * it from — while the app's own state is already protected by the pre-restore
 * safety archive `restoreBackup` takes. So a leftover is not recoverable through
 * any flow the app offers, and every stage attempt sweeps the folder first. The
 * cost is one `readDir` and one `remove` per file, which is the whole recovery
 * mechanism.
 */

import { baseName, getEesAmsRootDir } from '$lib/features/backup/paths';
import { getFileSystem } from '$lib/platform/fs';

const STAGING_FOLDER = 'restore-staging';

/**
 * Whether `path` is a staged copy, from its parent folder alone.
 *
 * Pure, so cleanup needs no disk round-trip, and narrow enough that a real archive
 * in `backups\` — the only other place this app writes a `.zip` — is never a match.
 */
function isStagedRestore(path: string): boolean {
	const segments = path.replace(/\\/g, '/').split('/');
	return segments[segments.length - 2] === STAGING_FOLDER;
}

async function stagingDir(): Promise<string> {
	const dir = `${await getEesAmsRootDir()}/${STAGING_FOLDER}`;
	await getFileSystem().mkdirp(dir);
	return dir;
}

/**
 * Copy a picked archive into the staging folder and return the in-scope path.
 *
 * ## Same-named archives are deliberately overwritten, not disambiguated
 *
 * The folder holds **one** staged copy at a time, because the app only ever has one
 * live reference to one: the `BackupPreview` in the restore dialog, guarded by
 * `restoreChoosing`. Re-picking or dismissing replaces it. So there is nothing to
 * disambiguate against, and the sweep above runs *before* the write — which makes
 * the overwrite safe rather than racy. Two backups called
 * `EES-AMS-20260928-142530.zip` on two drives get one staging slot, holding
 * whichever was picked last, and never a half-updated blend of the two.
 *
 * The alternative — a per-stage token in the name — buys nothing here and costs a
 * promise the filesystem cannot keep: a token taken from the clock repeats inside
 * one millisecond, so two picks in quick succession could still land on one path.
 * The sweep is the guarantee, not the name.
 */
export async function stageRestoreSource(pickedPath: string): Promise<string> {
	const fs = getFileSystem();
	const bytes = await fs.readFile(pickedPath);
	const dir = await stagingDir();
	for (const name of await fs.readDir(dir)) {
		// Only files are ever written here, but `TauriFileSystem.readDir` marks a
		// directory with a trailing `/` and `MemoryFileSystem` does not — so skip
		// them on the one signal both implementations agree on, rather than let a
		// stray folder decide whether the sweep throws.
		if (name.endsWith('/')) continue;
		await fs.remove(`${dir}/${name}`);
	}
	const path = `${dir}/restore-${baseName(pickedPath)}`;
	await fs.writeFileAtomic(path, bytes);
	return path;
}

/**
 * Delete a staged copy once its restore has succeeded.
 *
 * Best-effort by the same rule as the pre-wipe backup in `backup.ts`: a stale copy
 * is harmless, and failing to delete one must not fail a restore that worked. A
 * restore that *throws* leaves the copy in place, so the teacher can press the
 * button again without re-picking.
 */
export async function discardStagedRestore(path: string): Promise<void> {
	if (!isStagedRestore(path)) return;
	try {
		await getFileSystem().remove(path);
	} catch {
		// swept by the next stage attempt
	}
}
