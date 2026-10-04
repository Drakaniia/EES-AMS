import {
	exists,
	mkdir,
	readFile,
	readDir,
	rename,
	remove,
	stat,
	writeFile
} from '@tauri-apps/plugin-fs';
import type { FileStat, FileSystem } from './fs';
import { tempPathFor } from './fs';

/**
 * Runtime `FileSystem`, backed by `tauri-plugin-fs`.
 *
 * `writeFileAtomic` is the whole reason this is a class rather than a thin
 * re-export: the Rust code used an Excel lock plus a save-on-close dance to
 * avoid truncating a teacher's workbook. Writing a sibling temp file and
 * renaming over the target gets the same guarantee for a fraction of the code,
 * so the lock layer is not being ported.
 */
export class TauriFileSystem implements FileSystem {
	async readFile(path: string): Promise<Uint8Array> {
		return readFile(path);
	}

	async readTextFile(path: string): Promise<string> {
		return new TextDecoder().decode(await this.readFile(path));
	}

	async writeFileAtomic(path: string, contents: Uint8Array | string): Promise<void> {
		const temp = tempPathFor(path);
		const parent = parentOf(path);
		if (parent && !(await exists(parent))) await mkdir(parent, { recursive: true });
		await writeFile(
			temp,
			typeof contents === 'string' ? new TextEncoder().encode(contents) : contents
		);
		await rename(temp, path);
	}

	async exists(path: string): Promise<boolean> {
		return exists(path);
	}

	async mkdirp(path: string): Promise<void> {
		await mkdir(path, { recursive: true });
	}

	async stat(path: string): Promise<FileStat> {
		const info = await stat(path);
		return {
			isFile: info.isFile,
			isDirectory: info.isDirectory,
			size: info.size,
			modifiedAt: info.mtime ? new Date(info.mtime).getTime() : 0
		};
	}

	async readDir(path: string): Promise<string[]> {
		const entries = await readDir(path);
		return entries.map((entry) => (entry.isDirectory ? `${entry.name}/` : entry.name));
	}

	async remove(path: string): Promise<void> {
		await remove(path);
	}

	async rename(from: string, to: string): Promise<void> {
		await rename(from, to);
	}
}

function parentOf(path: string): string | null {
	const separator = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
	if (separator < 0) return null;
	const parent = path.slice(0, separator);
	return parent.length === 0 ? null : parent;
}
