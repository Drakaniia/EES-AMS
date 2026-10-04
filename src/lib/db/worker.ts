/// <reference lib="webworker" />
/**
 * The only place SQLite exists at runtime.
 *
 * Owns one WASM SQLite database backed by OPFS, and answers the message
 * protocol in `protocol.ts`. It is a dedicated Worker so a large report or an
 * attendance import never blocks the UI thread, and so the OPFS sync access
 * handle — which only a dedicated worker may hold — stays owned by one context.
 *
 * The database runs on sqlite-wasm's `opfs-sahpool` VFS. The older `opfs` VFS is
 * deliberately disabled (see `configureSqliteBootstrap`): it installs through a
 * second, classic "async proxy" Worker with a hard 4-second timeout and swallows
 * its own failure, which is how the app used to end up with an undefined
 * `oo1.OpfsDb` and a misleading diagnosis. `opfs-sahpool` holds its sync access
 * handles directly in this worker instead, so there is nothing to time out.
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
type OpfsSahPoolOptions = {
	name?: string;
	directory?: string;
	initialCapacity?: number;
	clearOnInit?: boolean;
	/** Lets a later call re-run the install after an earlier one failed. */
	forceReinitIfPreviouslyFailed?: boolean;
};
/**
 * The slice of the `opfs-sahpool` pool utility this worker uses.
 *
 * `OpfsSAHPoolDb` is the database constructor bound to this pool's VFS;
 * `importDb` overwrites a pooled file in place with a `.sqlite` image (the
 * restore path); `getFileNames` reports what the pool already holds.
 */
type OpfsSahPoolUtil = {
	OpfsSAHPoolDb: new (filename: string) => WasmDb;
	importDb(name: string, bytes: Uint8Array): number;
	getFileNames(): string[];
};
type WasmNamespace = {
	oo1: {
		DB: new (filename: string, mode?: string) => WasmDb;
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
	/** Installs the `opfs-sahpool` VFS. Absent only on builds that disabled it. */
	installOpfsSAHPoolVfs?: (options?: OpfsSahPoolOptions) => Promise<OpfsSahPoolUtil>;
};

/** The `opfs-sahpool` VFS this worker installs, and where its pooled files live. */
const SAHPOOL_VFS_NAME = 'ees-ams-sahpool';
const SAHPOOL_DIRECTORY = '.ees-ams-sahpool';
/**
 * The pool's client-facing name for the database, **with** the leading slash.
 *
 * The pool's `xOpen` resolves every filename with
 * `new URL(name, 'file://localhost/').pathname`, so a bare `ees-ams.sqlite3`
 * becomes `/ees-ams.sqlite3`. `importDb()` does *not* normalize its name, so
 * an import registered as `ees-ams.sqlite3` is a different file from the one
 * the app opens — the name used below has to be the normalized form.
 */
const POOL_DB_PATH = `/${DB_FILENAME}`;
/** The root file the retired `opfs` VFS wrote, migrated into the pool on first open. */
const LEGACY_DB_FILENAME = 'ees-ams.sqlite3';

let db: WasmDb | null = null;
let namespace: WasmNamespace | null = null;
/** The installed pool, needed to write a restored image back (see `importDatabase`). */
let pool: OpfsSahPoolUtil | null = null;
/**
 * Which file the live connection reads: the OPFS pool, or a throwaway
 * in-memory database entered explicitly from the unavailable-DB screen.
 * `import`/`export` branch on it, so a restore into temporary mode lands in
 * memory and never touches the OPFS pool.
 */
let mode: 'opfs-sah' | 'memory' | null = null;

/**
 * Turn off sqlite-wasm's `opfs` VFS before the module bootstraps.
 *
 * That VFS installs through a second "async proxy" Worker with a hard 4-second
 * timeout and reports failure only as a swallowed `config.warn`, leaving
 * `oo1.OpfsDb` undefined with no reason attached. This app uses `opfs-sahpool`
 * instead (see `open()`), so the proxy worker should never be spawned at all.
 *
 * Must run before `sqlite3InitModule()`: the bootstrap reads
 * `globalThis.sqlite3ApiConfig` once, and `opfs-sahpool` is unaffected by the
 * flag.
 */
function configureSqliteBootstrap(): void {
	const g = globalThis as unknown as {
		sqlite3ApiConfig?: { disable?: { vfs?: Record<string, boolean> } };
	};
	const config = g.sqlite3ApiConfig ?? {};
	g.sqlite3ApiConfig = {
		...config,
		disable: { ...config.disable, vfs: { ...config.disable?.vfs, opfs: true } }
	};
}

/** The live `navigator.storage`, passed as the receiver — never detached (see `probeOpfsStorage`). */
function liveStorage(): ProbeStorage | undefined {
	const nav = (globalThis as unknown as { navigator?: { storage?: ProbeStorage } }).navigator;
	return nav?.storage;
}

/**
 * Bring a database written by the retired `opfs` VFS into the `opfs-sahpool`
 * pool.
 *
 * The two VFSes keep their bytes in different places under the OPFS root, so
 * the first launch after the switch would otherwise start empty. The legacy
 * file is the raw SQLite image (the `opfs` VFS adds no header) and is left on
 * disk afterwards as a safety copy.
 */
async function migrateLegacyDatabase(sahp: OpfsSahPoolUtil): Promise<void> {
	if (sahp.getFileNames().includes(POOL_DB_PATH)) return;
	const nav = (
		globalThis as unknown as {
			navigator?: {
				storage?: {
					getDirectory?: () => Promise<{
						getFileHandle(
							name: string
						): Promise<{ getFile(): Promise<{ arrayBuffer(): Promise<ArrayBuffer> }> }>;
					}>;
				};
			};
		}
	).navigator;
	try {
		const root = await nav?.storage?.getDirectory?.();
		if (!root) return;
		const handle = await root.getFileHandle(LEGACY_DB_FILENAME);
		const bytes = new Uint8Array(await (await handle.getFile()).arrayBuffer());
		if (!looksLikeSqlite(bytes)) return;
		sahp.importDb(POOL_DB_PATH, bytes);
	} catch {
		// `NotFoundError` is the normal first-install case (there is no legacy
		// file); any other read failure must not block the fresh database.
	}
}

async function open(): Promise<string> {
	if (db && mode) return mode;
	// The shipped types take no options; the WASM build's own `print` is muted by
	// the postMessage error path, which is where diagnostics actually belong.
	// The namespace is initialised once per worker: a re-import reopens the
	// database below, and re-running the module bootstrap would only re-run its
	// initializers to warn that they already ran.
	configureSqliteBootstrap();
	const sqlite3 = namespace ?? ((await sqlite3InitModule()) as unknown as WasmNamespace);
	namespace = sqlite3;

	try {
		// Sync access handles: the fast path, and the reason this is a Worker.
		// `opfs-sahpool` holds them directly in this context, unlike the `opfs`
		// VFS it replaced, so a failure here is either a missing WebView2 API or
		// genuinely blocked storage — never a proxy worker timing out.
		if (!sqlite3.installOpfsSAHPoolVfs) {
			const env = readOpfsEnv();
			const { cause, note } = classifyMissingOpfs(env);
			throw appError('Database', await diagnoseOpfsFailure(note, cause, env, liveStorage()));
		}
		try {
			pool = await sqlite3.installOpfsSAHPoolVfs({
				name: SAHPOOL_VFS_NAME,
				directory: SAHPOOL_DIRECTORY,
				initialCapacity: 6,
				forceReinitIfPreviouslyFailed: true
			});
		} catch (error) {
			// Unlike the `opfs` VFS, this install does not swallow its reason —
			// keep it so Details says *why* the engine failed to start.
			const env = readOpfsEnv();
			throw appError(
				'Database',
				await diagnoseOpfsFailure(
					`could not start the on-device storage engine (${asAppError(error).detail})`,
					'storage-blocked',
					env,
					liveStorage()
				)
			);
		}
		await migrateLegacyDatabase(pool);
		db = new pool.OpfsSAHPoolDb(POOL_DB_PATH);
		mode = 'opfs-sah';
		return mode;
	} catch (error) {
		// An `AppError` already carries the `[cause=…]` detail; only an
		// unexpected throw needs wrapping.
		if (typeof error === 'object' && error !== null && 'kind' in error && 'detail' in error) {
			throw error;
		}
		throw appError(
			'Database',
			`could not open the on-device database (${asAppError(error).detail})`
		);
	}
}

/**
 * Temporary in-memory database, entered only from the unavailable-DB screen.
 *
 * Same WASM module, same `SqlDriver` surface, no OPFS handle — so attendance
 * and SF2 preview keep working while nothing persists past the worker's life.
 * The OPFS pool is left installed but unused, so a later `open()` reconnects to
 * the real database.
 */
async function openTemporary(): Promise<string> {
	if (db && mode === 'memory') return mode;
	configureSqliteBootstrap();
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
 * Replace the pooled database file with `bytes`, then reopen it.
 *
 * `sqlite3_deserialize()` is deliberately *not* used, though it would be the
 * one-call option: it swaps the connection onto the in-memory buffer, so every
 * write after a restore would live in RAM and vanish with this worker. Writing
 * the image through the pool's `importDb()` — the documented restore path for
 * the `opfs-sahpool` VFS — and reopening makes the archive's bytes the file
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
	if (!pool) throw appError('Internal', 'OPFS database import is unavailable');

	// OPFS sync access handles are exclusive, and the pool writes over the
	// pooled file in place: the connection must be closed first, and reopened
	// once the bytes have landed.
	db?.close();
	db = null;
	try {
		pool.importDb(POOL_DB_PATH, bytes);
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
 * The pool's `importDb` cannot help here — there is no pooled file to write in
 * memory mode — so the image goes through `sqlite3_deserialize` into a fresh
 * `:memory:` connection. `SQLITE_DESERIALIZE_FREEONCLOSE` hands the buffer to
 * SQLite, which frees it when the connection closes; nothing here frees it
 * afterwards.
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
