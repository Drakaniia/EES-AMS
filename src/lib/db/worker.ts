/// <reference lib="webworker" />
/**
 * The only place SQLite exists at runtime.
 *
 * Owns one WASM SQLite database backed by OPFS, and answers the message
 * protocol in `protocol.ts`. It is a dedicated Worker so a large report or an
 * attendance import never blocks the UI thread, and so the OPFS sync access
 * handle — which only a dedicated worker may hold — stays owned by one context.
 */
import sqlite3InitModule from '@sqlite.org/sqlite-wasm';
import { DB_FILENAME, type WorkerRequest, type WorkerResponse } from './protocol';
import { appError, asAppError, type AppError } from './error';
import { looksLikeSqlite, type SqlParam } from './driver';

/**
 * The slice of the sqlite-wasm API this worker uses. Declaring it here keeps
 * `any` out of the codebase; the real module has a much larger surface.
 */
type RowCallback = (row: Record<string, unknown>) => number | void;
type ExecOptions = {
	sql: string;
	bind?: SqlParam[];
	rowMode?: 'object';
	callback?: RowCallback;
	returnValue?: 'resultRows';
	resultRows?: unknown[];
};
type WasmDb = {
	exec(options: ExecOptions | string): unknown;
	prepare(sql: string): {
		bind(params: SqlParam[]): unknown;
		step(): boolean;
		get(column?: string): Record<string, unknown>;
		reset(): void;
		finalize(): void;
	};
	/** `sqlite3_changes()` — the row count of the statement just executed. */
	changes(total?: boolean): number;
	close(): void;
	readonly pointer: number;
};
type WasmNamespace = {
	oo1: {
		DB: new (filename: string, mode?: string) => WasmDb;
		/**
		 * The `opfs` VFS's database class. `importDb` is its static file-image
		 * writer — the supported way to replace the file this VFS reads.
		 */
		OpfsDb: (new (filename: string) => WasmDb) & {
			importDb?(filename: string, bytes: Uint8Array): Promise<number>;
		};
	};
	capi: {
		/** `sqlite3_serialize(db, zSchema, pN, mFlags)` — the database as a file image. */
		sqlite3_serialize?: (db: number, schema: string, pN: number, mFlags: 0) => number;
		/** Frees memory SQLite allocated, including a `sqlite3_serialize()` result. */
		sqlite3_free?: (ptr: number) => void;
	};
	/** The allocator and heap access the size out-parameter needs. */
	wasm: {
		alloc(bytes: number): number;
		dealloc(ptr: number): void;
		peek(addr: number, representation?: string): number | bigint;
		heap8u(): Uint8Array;
	};
};

let db: WasmDb | null = null;
let namespace: WasmNamespace | null = null;

/**
 * Why the `opfs` VFS refused to install, in teacher-actionable terms.
 *
 * Mirrors sqlite-wasm's `vfsInstallationFeatureCheck` (SAB+Atomics, worker
 * context, FileSystem sync-access APIs), whose own failure is swallowed to a
 * `warn` by the module bootstrap — leaving `oo1.OpfsDb` undefined with no
 * reason attached. SAB present + OpfsDb missing means headers are fine and
 * the PC's WebView2 is too old for OPFS sync access handles.
 */
function describeMissingOpfs(): string {
	const g = globalThis as Record<string, unknown>;
	const sabMissing =
		typeof SharedArrayBuffer === 'undefined' || typeof (g['Atomics'] as object) === 'undefined';
	if (sabMissing) {
		return 'the opfs VFS is unavailable (SharedArrayBuffer is missing); the app must be served with COOP/COEP headers';
	}
	const fh = g['FileSystemHandle'];
	const dir = g['FileSystemDirectoryHandle'];
	const fileHandle = g['FileSystemFileHandle'] as
		| { prototype?: { createSyncAccessHandle?: unknown } }
		| undefined;
	const hasSyncHandle = typeof fileHandle?.prototype?.createSyncAccessHandle !== 'undefined';
	const nav = g['navigator'] as { storage?: { getDirectory?: unknown } } | undefined;
	const hasGetDirectory = typeof nav?.storage?.getDirectory !== 'undefined';
	if (
		typeof fh === 'undefined' ||
		typeof dir === 'undefined' ||
		!hasSyncHandle ||
		!hasGetDirectory
	) {
		return 'the opfs VFS is unavailable (this PC’s WebView2 runtime lacks OPFS sync-access handles); update “Microsoft Edge WebView2 Runtime” to the latest version, then reopen the app';
	}
	if (typeof crossOriginIsolated !== 'undefined' && !crossOriginIsolated) {
		return 'the opfs VFS is unavailable (the window is not cross-origin isolated); the app must be served with COOP/COEP headers';
	}
	return 'the opfs VFS is unavailable (OPFS storage is blocked on this PC); check disk space and site-storage permissions, then reopen the app';
}

type SyncAccessHandleLike = {
	write(buffer: Uint8Array, options?: { at?: number }): number;
	truncate(size: number): void;
	close(): void;
};
type ProbeFileHandle = {
	createSyncAccessHandle(): Promise<SyncAccessHandleLike>;
};
type ProbeRoot = {
	getFileHandle(name: string, options?: { create?: boolean }): Promise<ProbeFileHandle>;
	removeEntry(name: string): Promise<void>;
};

/**
 * Facts about this runtime, appended to every OPFS failure so the message
 * names the actual environment instead of guessing it. `protocol` tells dev
 * (`http:` + Vite) apart from release (`https:` + Tauri asset protocol).
 */
function envFacts(): string {
	const g = globalThis as Record<string, unknown>;
	const loc = g['location'] as { protocol?: unknown } | undefined;
	return [
		`protocol=${typeof loc?.protocol === 'string' ? loc.protocol : 'unknown'}`,
		`isolated=${typeof crossOriginIsolated !== 'undefined' ? String(crossOriginIsolated) : 'unknown'}`,
		`sab=${typeof SharedArrayBuffer !== 'undefined' ? 'yes' : 'no'}`
	].join(' ');
}

/**
 * Live OPFS probe, run only on the failure path.
 *
 * The sqlite-wasm VFS install failure is swallowed to a `warn` by the module
 * bootstrap, so a missing `OpfsDb` alone cannot say whether storage is
 * blocked or the proxy worker simply failed to start. Writing a probe file
 * through a sync access handle answers exactly that: if the probe passes,
 * storage is fine and the failure is the VFS install (proxy/headers), not
 * the PC.
 */
async function probeOpfsStorage(): Promise<string | null> {
	try {
		const nav = (globalThis as unknown as { navigator?: { storage?: { getDirectory?: unknown } } })
			.navigator;
		const getDirectory = nav?.storage?.getDirectory;
		if (typeof getDirectory !== 'function') return 'navigator.storage.getDirectory is missing';
		const root = (await (getDirectory as () => Promise<ProbeRoot>)()) as ProbeRoot;
		const file = await root.getFileHandle('.ees-ams-opfs-probe', { create: true });
		const access = await file.createSyncAccessHandle();
		try {
			access.write(new Uint8Array([1, 2, 3, 4]), { at: 0 });
			access.truncate(4);
		} finally {
			access.close();
		}
		await root.removeEntry('.ees-ams-opfs-probe');
		return null;
	} catch (error) {
		return error instanceof Error ? error.message : String(error);
	}
}

/** The static reason plus the live probe, so the message is evidence, not a guess. */
async function diagnoseOpfsFailure(note: string): Promise<string> {
	const probe = await probeOpfsStorage();
	const storage =
		probe === null
			? 'OPFS storage probe passed, so the failure is the SQLite proxy worker/headers rather than blocked storage'
			: `OPFS storage probe failed: ${probe}`;
	return `${note} [${envFacts()}; ${storage}]`;
}

async function open(): Promise<string> {
	if (db) return DB_FILENAME;
	// The shipped types take no options; the WASM build's own `print` is muted by
	// the postMessage error path, which is where diagnostics actually belong.
	// The namespace is initialised once per worker: a re-import reopens the
	// database below, and re-running the module bootstrap would only re-run its
	// initializers to warn that they already ran.
	const sqlite3 = namespace ?? ((await sqlite3InitModule()) as unknown as WasmNamespace);
	namespace = sqlite3;

	try {
		// Sync access handles: the fast path, and the reason this is a Worker.
		// `OpfsDb` is installed by the module's `opfs` VFS initializer, which
		// reports failure as a warn and leaves this undefined — so the message
		// below names the real cause instead of a bare "not a constructor".
		// NOTE: SharedArrayBuffer being present proves COOP/COEP headers are
		// fine — when it is missing here the cause is almost always an
		// outdated Edge WebView2 Runtime on that PC (no
		// FileSystemSyncAccessHandle in workers), not the app's headers.
		// Mirror sqlite-wasm's own vfsInstallationFeatureCheck so the teacher
		// gets the actionable reason instead of a header red herring.
		if (!sqlite3.oo1.OpfsDb) {
			throw appError('Database', await diagnoseOpfsFailure(describeMissingOpfs()));
		}
		try {
			db = new sqlite3.oo1.OpfsDb(DB_FILENAME);
		} catch (error) {
			throw appError(
				'Database',
				await diagnoseOpfsFailure(`could not open OPFS database (${asAppError(error).detail})`)
			);
		}
		return 'opfs-sah';
	} catch (error) {
		if (typeof error === 'object' && error !== null && 'kind' in error && 'detail' in error) {
			throw error;
		}
		throw appError('Database', `could not open OPFS database (${asAppError(error).detail})`);
	}
}

function requireDb(): WasmDb {
	if (!db) throw appError('Database', 'database is not open');
	return db;
}

/**
 * SQLite hands back `bigint` for INTEGER and `Uint8Array` for BLOB. `rusqlite`
 * surfaced those as JSON numbers, so normalise here — once — rather than in
 * every repo.
 */
function normaliseRow(row: Record<string, unknown>): Record<string, unknown> {
	const out: Record<string, unknown> = {};
	for (const [key, value] of Object.entries(row)) {
		out[key] = typeof value === 'bigint' ? Number(value) : value;
	}
	return out;
}

function query(sql: string, params: SqlParam[]): Record<string, unknown>[] {
	const rows: Record<string, unknown>[] = [];
	requireDb().exec({
		sql,
		bind: params,
		rowMode: 'object',
		callback: (row) => void rows.push(row)
	});
	return rows.map(normaliseRow);
}

function execute(sql: string, params: SqlParam[]): number {
	const handle = requireDb();
	handle.exec({ sql, bind: params });
	return Number(handle.changes());
}

function run(request: WorkerRequest): unknown {
	switch (request.op) {
		case 'query':
			return query(request.sql ?? '', request.params ?? []);
		case 'execute':
			return execute(request.sql ?? '', request.params ?? []);
		case 'script':
			requireDb().exec(request.sql ?? '');
			return null;
		case 'export':
			return exportDatabase();
		case 'import':
			return importDatabase(request.bytes);
		case 'close': {
			db?.close();
			db = null;
			return null;
		}
		default:
			throw appError('InvalidInput', `unknown worker op ${String(request.op)}`);
	}
}

/**
 * The whole database as a `.sqlite` file image: one `sqlite3_serialize()` call.
 *
 * The bytes are a real database file — openable by any SQLite client, and
 * exactly what `import` writes back on a restore. SQLite allocates the result,
 * so it is released with `sqlite3_free`; the length comes back through an
 * 8-byte out-parameter, which is why the out-parameter gets its own allocation.
 */
function exportDatabase(): Uint8Array {
	const handle = requireDb();
	const capi = namespace?.capi;
	const wasm = namespace?.wasm;
	if (!capi?.sqlite3_serialize || !capi.sqlite3_free || !wasm) {
		throw appError('Internal', 'sqlite3_serialize is unavailable');
	}
	const pSize = wasm.alloc(8);
	try {
		const pData = capi.sqlite3_serialize(handle.pointer, 'main', pSize, 0);
		if (!pData) throw appError('Database', 'sqlite3_serialize failed');
		try {
			const size = Number(wasm.peek(pSize, 'i64'));
			return wasm.heap8u().slice(pData, pData + size);
		} finally {
			capi.sqlite3_free(pData);
		}
	} finally {
		wasm.dealloc(pSize);
	}
}

/**
 * Replace the database file with `bytes`, then reopen it.
 *
 * `sqlite3_deserialize()` is deliberately *not* used, though it would be the
 * one-call option: it swaps the connection onto the in-memory buffer, so every
 * write after a restore would live in RAM and vanish with this worker. Writing
 * the image through `OpfsDb.importDb()` — the documented pairing for the `opfs`
 * VFS this worker runs on — and reopening makes the archive's bytes the file
 * itself, which is what "restore" has to mean.
 *
 * A failed write reopens the old file first: a restore that cannot land leaves
 * the teacher exactly where they were.
 */
async function importDatabase(bytes: Uint8Array | undefined): Promise<null> {
	if (!bytes || !looksLikeSqlite(bytes)) {
		throw appError('InvalidInput', 'import requires the bytes of a SQLite database file');
	}
	const importer = namespace?.oo1.OpfsDb?.importDb;
	if (!importer) throw appError('Internal', 'OPFS database import is unavailable');

	// OPFS sync access handles are exclusive: the file must be closed before it
	// can be rewritten, and reopened once it has been.
	db?.close();
	db = null;
	try {
		await importer(DB_FILENAME, bytes);
	} catch (thrown) {
		await open();
		throw appError(
			'Database',
			`could not write the restored database (${thrown instanceof Error ? thrown.message : String(thrown)})`
		);
	}
	await open();
	return null;
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
	const request = event.data;
	let response: WorkerResponse;
	try {
		if (request.op === 'open') {
			const mode = await open();
			response = { id: request.id, ok: true, value: mode };
		} else {
			// `await`: `import` closes and reopens the database around an async
			// OPFS write, while every other op answers synchronously.
			response = { id: request.id, ok: true, value: await run(request) };
		}
	} catch (thrown) {
		const error: AppError =
			typeof thrown === 'object' && thrown !== null && 'kind' in thrown
				? (thrown as AppError)
				: appError('Internal', thrown instanceof Error ? thrown.message : String(thrown));
		response = { id: request.id, ok: false, error };
	}
	self.postMessage(response);
};
