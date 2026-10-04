/**
 * The two places the app asks the OS for a file.
 *
 * ## Why the import is dynamic
 *
 * `@tauri-apps/plugin-dialog` is a shell-only module: it exists to open a native
 * picker inside a Tauri window. A static import would put it in the module graph of
 * every consumer of `$lib/api` — including Vitest, which has no window and must
 * never resolve a plugin it does not call. A dynamic import keeps the dependency
 * where it belongs: at the moment a picker is actually opened.
 *
 * Everything after a path is chosen is plain TypeScript, which is why the restore
 * and workbook-open flows are testable without a shell.
 */

import { internal } from '$lib/db';

type DialogModule = typeof import('@tauri-apps/plugin-dialog');

async function dialog(): Promise<DialogModule> {
	try {
		return await import('@tauri-apps/plugin-dialog');
	} catch (thrown) {
		throw internal(`the file picker is unavailable: ${String(thrown)}`);
	}
}

/** `null` when the user dismissed the picker, which is not an error. */
async function pickOne(directory: boolean, extensions: string[]): Promise<string | null> {
	const { open } = await dialog();
	const picked = await open({
		directory,
		multiple: false,
		filters: directory ? undefined : [{ name: 'Supported file', extensions }]
	});
	return typeof picked === 'string' ? picked : null;
}

/** One workbook — the pick-then-act flow `open_sf2_workbook` always had. */
export async function pickWorkbookFile(): Promise<string | null> {
	return await pickOne(false, ['xls', 'xlsx', 'xlsm']);
}

/** One backup archive. D12 stores backups as zips, so the picker filters to zips. */
export async function pickBackupArchive(): Promise<string | null> {
	return await pickOne(false, ['zip']);
}
