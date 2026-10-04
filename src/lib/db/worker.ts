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
import {
	classifyMissingOpfs,
	diagnoseOpfsFailure,
	readOpfsEnv,
	type ProbeStorage
} from './opfs-diagnosis';

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
		/** `sqlite3_deserialize` — load a file image into a connection (memory-mode restores). */
		sqlite3_deserialize?: (
			db: number,
			schema: string,
			data: number,
			dbSize: number,
			bufferSize: number,
			flags: number
		) => number;
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
 * Which file the live connection reads: the OPFS file, or a throwaway
 * in-memory database entered explicitly from the unavailable-DB screen.
 * `import`/`export` branch on it, so a restore into temporary mode lands in
 * memory and never touches the OPFS file.
 */
let mode: 'opfs-sah' | 'memory' | null = null;

/** The live `navigator.storage`, passed as the receiver — never detached (see `probeOpfsStorage`). */
function liveStorage(): ProbeStorage | undefined {
	const nav = (globalThis as unknown as { navigator?: { storage?: ProbeStorage } }).navigator;
	return nav?.storage;
}

async function open(): Promise<string> {
	if (db && mode) return mode;
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
			const env = readOpfsEnv();
			const { cause, note } = classifyMissingOpfs(env);
			throw appError('Database', await diagnoseOpfsFailure(note, cause, env, liveStorage()));
		}
		try {
			db = new sqlite3.oo1.OpfsDb(DB_FILENAME);
		} catch (error) {
			const env = readOpfsEnv();
			throw appError(
				'Database',
				await diagnoseOpfsFailure(
					`could not open OPFS database (${asAppError(error).detail})`,
					'storage-blocked',
					env,
					liveStorage()
				)
			);
		}
		mode = 'opfs-sah';
		return mode;
	} catch (error) {
		if (typeof error === 'object' && error !== null && 'kind' in error && 'detail' in error) {
			throw error;
		}
		throw appError('Database', `could not open OPFS database (${asAppError(error).detail})`);
	}
}

/**
 * Temporary in-memory database, entered only from the unavailable-DB screen.
 *
 * Same WASM module, same `SqlDriver` surface, no OPFS handle — so attendance
 * and SF2 preview keep working while nothing persists past the worker's life.
 * The OPFS file is closed first (it stays on disk, untouched) because sync
 * access handles are exclusive.
 */
async function openTemporary(): Promise<string> {
	if (db && mode === 'memory') return mode;
	const sqlite3 = namespace ?? ((await sqlite3InitModule()) as unknown as WasmNamespace);
	namespace = sqlite3;
	db?.close();
	db = new sqlite3.oo1.DB(':memory:');
	mode = 'memory';
	return mode;
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
			mode = null;
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
	if (mode === 'memory') {
		await importIntoMemory(bytes);
		return null;
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

/**
 * Load a file image into the temporary in-memory database (a restore while in
 * temporary mode).
 *
 * OPFS's `importDb` cannot help here — there is no file to write — so the
 * image goes through `sqlite3_deserialize` into a fresh `:memory:` connection.
 * `SQLITE_DESERIALIZE_FREEONCLOSE` hands the buffer to SQLite, which frees it
 * when the connection closes; nothing here frees it afterwards.
 */
async function importIntoMemory(bytes: Uint8Array): Promise<void> {
	const sqlite3 = namespace;
	const capi = sqlite3?.capi;
	const wasm = sqlite3?.wasm;
	if (!sqlite3 || !capi?.sqlite3_deserialize || !wasm) {
		throw appError('Internal', 'in-memory database restore is unavailable');
	}
	db?.close();
	db = null;
	const fresh = new sqlite3.oo1.DB(':memory:');
	const pData = wasm.alloc(bytes.length);
	wasm.heap8u().set(bytes, pData);
	let code: number;
	try {
		code = capi.sqlite3_deserialize(fresh.pointer, 'main', pData, bytes.length, bytes.length, 1);
	} catch (thrown) {
		fresh.close();
		throw appError(
			'Database',
			`could not load the restored database (${thrown instanceof Error ? thrown.message : String(thrown)})`
		);
	}
	if (code !== 0) {
		fresh.close();
		throw appError('Database', `could not load the restored database (error code ${code})`);
	}
	db = fresh;
	mode = 'memory';
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
	const request = event.data;
	let response: WorkerResponse;
	try {
		if (request.op === 'open') {
			const value = await open();
			response = { id: request.id, ok: true, value };
		} else if (request.op === 'open-temporary') {
			const value = await openTemporary();
			response = { id: request.id, ok: true, value };
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
