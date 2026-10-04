/**
 * The zip archive — an in-memory set of named files.
 *
 * `fflate` does the zip itself. The entries are sorted so the same inputs always
 * produce the same bytes, which is what makes two archives comparable in a diff.
 */

import { unzipSync, zipSync } from 'fflate';
import { asAppError, internal } from '$lib/db';
import { getFileSystem } from '$lib/platform/fs';

/** One file inside an archive. `path` is slash-separated and archive-relative. */
export type ArchiveFile = {
	path: string;
	bytes: Uint8Array;
};

export type BackupArchive = {
	files: ArchiveFile[];
};

export function archiveFile(archive: BackupArchive, path: string): Uint8Array | undefined {
	return archive.files.find((file) => file.path === path)?.bytes;
}

export function archiveText(archive: BackupArchive, path: string): string | undefined {
	const bytes = archiveFile(archive, path);
	return bytes === undefined ? undefined : new TextDecoder().decode(bytes);
}

export function buildArchive(files: readonly ArchiveFile[]): Uint8Array {
	const entries: Record<string, Uint8Array> = {};
	for (const file of [...files].sort((left, right) => (left.path < right.path ? -1 : 1))) {
		entries[file.path] = file.bytes;
	}
	return zipSync(entries);
}

export function readArchive(bytes: Uint8Array): BackupArchive {
	const unzipped = unzipSync(bytes);
	return { files: Object.entries(unzipped).map(([path, data]) => ({ path, bytes: data })) };
}

export async function readArchiveFile(path: string): Promise<BackupArchive> {
	try {
		return readArchive(await getFileSystem().readFile(path));
	} catch (thrown) {
		throw internal(`${path} is not a readable EES-AMS backup: ${asAppError(thrown).detail}`);
	}
}
