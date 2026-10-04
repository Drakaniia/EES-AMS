/**
 * The SF2 workbooks inside a backup, and putting them back.
 *
 * A port of `workbooks.rs`, with the recursive copy turned into an in-memory
 * file list: the archive is a zip, so a "copy" is a read now and a write during
 * restore. What is kept is the part that was load-bearing — recurse rather than
 * enumerate, so the `_legacy/` subfolder and any later layout change need no
 * special case, and fail the whole snapshot if any single file cannot be read, so
 * an archive never claims to hold a workbook it does not.
 *
 * `xCount` is read but is never allowed to fail the snapshot: counting needs the
 * Excel engine to parse the file, and a workbook the teacher has open is a
 * normal state, not a reason to lose the backup.
 */

import { asAppError } from '$lib/db';
import { getFileSystem } from '$lib/platform/fs';
import type { ArchiveFile } from './archive';
import type { ManifestWorkbook } from './manifest';
import { WORKBOOK_PREFIX } from './manifest';
import { getWorkbooksDir } from './paths';
import { countXmarks } from './x-count';

type WorkbookSnapshot = {
	/** The workbook bytes, ready to be put in the archive. */
	files: ArchiveFile[];
	/** One manifest entry per collected workbook, in sorted path order. */
	entries: ManifestWorkbook[];
	/** True when the teacher has never made a workbook yet. */
	sourceMissing: boolean;
};

const EMPTY: WorkbookSnapshot = { files: [], entries: [], sourceMissing: true };

export async function collectWorkbooks(): Promise<WorkbookSnapshot> {
	const dir = await getWorkbooksDir();
	if (!(await getFileSystem().exists(dir))) return EMPTY;

	const fs = getFileSystem();
	const relativePaths = await listFilesRecursive(dir);
	relativePaths.sort();

	const files: ArchiveFile[] = [];
	const entries: ManifestWorkbook[] = [];
	for (const relative of relativePaths) {
		const bytes = await fs.readFile(`${dir}/${relative}`);
		const path = `${WORKBOOK_PREFIX}${relative}`;
		files.push({ path, bytes });
		entries.push({ path, bytes: bytes.byteLength, xCount: await safeXCount(`${dir}/${relative}`) });
	}
	return { files, entries, sourceMissing: false };
}

/**
 * Write a backup's workbooks back over the live SF2 directory.
 *
 * Returns how many files were written; 0 means the archive carried none, which
 * the caller reports as "your current workbooks were left in place" rather than
 * as a failure.
 */
export async function writeWorkbooks(files: readonly ArchiveFile[]): Promise<number> {
	const dir = await getWorkbooksDir();
	const fs = getFileSystem();
	let written = 0;
	for (const file of files) {
		const relative = file.path.startsWith(WORKBOOK_PREFIX)
			? file.path.slice(WORKBOOK_PREFIX.length)
			: file.path;
		if (relative === '') continue;
		await fs.writeFileAtomic(`${dir}/${relative}`, file.bytes);
		written += 1;
	}
	return written;
}

/** Every file below `dir`, as slash-separated paths relative to `dir`. */
async function listFilesRecursive(dir: string): Promise<string[]> {
	const fs = getFileSystem();
	const found: string[] = [];
	const pending = [''];
	while (pending.length > 0) {
		const prefix = pending.pop() as string;
		for (const name of await fs.readDir(prefix === '' ? dir : `${dir}/${prefix}`)) {
			// `readDir` appends a separator to a sub-folder on the Tauri side but not
			// on the in-memory one, so the folder-ness is asked for with `stat`
			// rather than guessed from the name.
			const relative = prefix === '' ? name : `${prefix}/${name}`;
			const child = `${dir}/${relative}`.replace(/\/+$/, '');
			if ((await fs.stat(child)).isDirectory) pending.push(relative);
			else found.push(relative);
		}
	}
	return found;
}

async function safeXCount(path: string): Promise<number> {
	try {
		return await countXmarks(path);
	} catch (thrown) {
		// ponytail: 0 means "unknown" for the caller, so a locked workbook cannot
		// fail a backup. Surface the reason rather than swallowing it.
		console.warn(`[backup] could not count X marks in ${path}: ${asAppError(thrown).detail}`);
		return 0;
	}
}
