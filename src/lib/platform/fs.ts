/**
 * File system access for the app.
 *
 * The webview cannot touch arbitrary Windows paths, so the runtime
 * implementation goes through `tauri-plugin-fs` — an official Tauri plugin, so
 * it adds no hand-written Rust. Everything above this interface is pure
 * TypeScript and testable against `MemoryFileSystem`.
 */

export interface FileStat {
	isFile: boolean;
	isDirectory: boolean;
	size: number;
	/** Milliseconds since the epoch, or 0 when unknown. */
	modifiedAt: number;
}

export interface FileSystem {
	readFile(path: string): Promise<Uint8Array>;
	readTextFile(path: string): Promise<string>;
	/** Writes via a sibling temp file and renames over the target. */
	writeFileAtomic(path: string, contents: Uint8Array | string): Promise<void>;
	exists(path: string): Promise<boolean>;
	mkdirp(path: string): Promise<void>;
	stat(path: string): Promise<FileStat>;
	readDir(path: string): Promise<string[]>;
	remove(path: string): Promise<void>;
	rename(from: string, to: string): Promise<void>;
}

/**
 * The OS temp name for a sibling file. Kept next to the target so the rename
 * stays on one volume and is therefore atomic — a workbook is never observed
 * half-written.
 */
export function tempPathFor(path: string): string {
	const lastSeparator = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
	const directory = lastSeparator >= 0 ? path.slice(0, lastSeparator) : '';
	const name = lastSeparator >= 0 ? path.slice(lastSeparator + 1) : path;
	return `${directory}${directory ? '/' : ''}${name}.tmp`;
}

/** In-memory implementation for tests. Paths are opaque strings. */
export class MemoryFileSystem implements FileSystem {
	readonly files = new Map<string, Uint8Array>();
	readonly directories = new Set<string>(['/']);

	private normalise(path: string): string {
		return path.replace(/\\/g, '/');
	}

	private ensureParents(path: string): void {
		const parts = this.normalise(path).split('/');
		parts.pop();
		let current = '';
		for (const part of parts) {
			if (!part) continue;
			current += `/${part}`;
			this.directories.add(current);
		}
	}

	async readFile(path: string): Promise<Uint8Array> {
		const found = this.files.get(this.normalise(path));
		if (!found) throw new Error(`ENOENT: ${path}`);
		return found;
	}

	async readTextFile(path: string): Promise<string> {
		return new TextDecoder().decode(await this.readFile(path));
	}

	async writeFileAtomic(path: string, contents: Uint8Array | string): Promise<void> {
		const bytes = typeof contents === 'string' ? new TextEncoder().encode(contents) : contents;
		const key = this.normalise(path);
		this.ensureParents(key);
		this.files.set(tempPathFor(key), bytes);
		this.files.set(key, bytes);
		this.files.delete(tempPathFor(key));
	}

	async exists(path: string): Promise<boolean> {
		const key = this.normalise(path);
		return this.files.has(key) || this.directories.has(key);
	}

	async mkdirp(path: string): Promise<void> {
		this.directories.add(this.normalise(path));
	}

	async stat(path: string): Promise<FileStat> {
		const key = this.normalise(path);
		const bytes = this.files.get(key);
		if (bytes) return { isFile: true, isDirectory: false, size: bytes.byteLength, modifiedAt: 0 };
		if (this.directories.has(key)) {
			return { isFile: false, isDirectory: true, size: 0, modifiedAt: 0 };
		}
		throw new Error(`ENOENT: ${path}`);
	}

	async readDir(path: string): Promise<string[]> {
		const prefix = `${this.normalise(path).replace(/\/$/, '')}/`;
		const names = new Set<string>();
		for (const key of [...this.files.keys(), ...this.directories]) {
			if (key.startsWith(prefix)) names.add(key.slice(prefix.length).split('/')[0]);
		}
		return [...names];
	}

	async remove(path: string): Promise<void> {
		const key = this.normalise(path);
		this.files.delete(key);
		this.directories.delete(key);
	}

	async rename(from: string, to: string): Promise<void> {
		const bytes = this.files.get(this.normalise(from));
		if (!bytes) throw new Error(`ENOENT: ${from}`);
		this.ensureParents(to);
		this.files.set(this.normalise(to), bytes);
		this.files.delete(this.normalise(from));
	}
}

let injected: FileSystem | null = null;

/** Test seam, mirroring `useDriver()` in `$lib/db`. */
export function useFileSystem(fileSystem: FileSystem | null): void {
	injected = fileSystem;
}

export function getFileSystem(): FileSystem {
	if (injected) return injected;
	throw new Error(
		'No file system bound. The Tauri implementation is loaded by $lib/platform/tauri-fs at startup.'
	);
}
