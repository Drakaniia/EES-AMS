/**
 * SqlDriver — the whole of the data-access surface.
 *
 * Two implementations exist and nothing else in the app may talk to SQLite
 * directly: `client.ts` (WASM SQLite in a Worker over OPFS, the app runtime)
 * and `node-driver.ts` (`node:sqlite`, tests only). Every repo is written
 * against this interface so tests run real SQL without a browser.
 *
 * `node:sqlite` returns SQLite's native column values; `bigint` for INTEGER,
 * `Uint8Array` for BLOB, `null` for NULL. That is the contract `query()`
 * promises to its callers, and repos are responsible for coercing to the
 * shapes in `src/lib/types.ts`.
 */

export type SqlParam = string | number | bigint | Uint8Array | null;

export interface SqlDriver {
	/** `SELECT`. Resolves rows as objects keyed by column name. */
	query<T = Record<string, SqlValue>>(sql: string, params?: SqlParam[]): Promise<T[]>;
	/** A single row, or `undefined`. Lazy sugar over `query()[0]`. */
	queryOne<T = Record<string, SqlValue>>(sql: string, params?: SqlParam[]): Promise<T | undefined>;
	/** `INSERT`/`UPDATE`/`DELETE`/`CREATE`. Resolves the affected row count. */
	execute(sql: string, params?: SqlParam[]): Promise<number>;
	/** Multi-statement DDL and migration bodies, executed in order. */
	script(sql: string): Promise<void>;
	/** `BEGIN`/`COMMIT`, rolling back if `fn` throws. */
	transaction<T>(fn: () => Promise<T>): Promise<T>;
	/**
	 * The live database as a `.sqlite` file image — one consistent, fully-
	 * checkpointed copy of every page. This is what a backup archive carries.
	 */
	exportFile(): Promise<Uint8Array>;
	/** Replace the whole database with a `.sqlite` file image, as a restore does. */
	importFile(bytes: Uint8Array): Promise<void>;
	/** Flush and release the underlying database handle. */
	close(): Promise<void>;
}

/**
 * True when `bytes` begin with SQLite's file header, i.e. when they could open
 * as a database at all.
 *
 * Both drivers check this before replacing a live database with an image: an
 * archive member that is not a database should be refused while the current
 * data is still untouched, not after the connection has been closed.
 */
export function looksLikeSqlite(bytes: Uint8Array): boolean {
	const header = 'SQLite format 3\0';
	if (bytes.byteLength < header.length) return false;
	for (let index = 0; index < header.length; index += 1) {
		if (bytes[index] !== header.charCodeAt(index)) return false;
	}
	return true;
}

export type SqlValue = string | number | bigint | Uint8Array | null;
