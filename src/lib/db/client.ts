import type { AppError } from './error';
import type { SqlDriver, SqlParam } from './driver';
import type { WorkerRequest, WorkerResponse } from './protocol';
import { migrate } from './migrations';

/**
 * Main-thread half of the SQLite driver: a typed RPC over `worker.ts`.
 *
 * Every call is a `postMessage` round trip, which is why nothing above this
 * layer may assume synchronous queries. Transactions hold an exclusive gate so
 * a stray query from another caller cannot land between `BEGIN` and `COMMIT`.
 */
export class WorkerSqlDriver implements SqlDriver {
	private readonly worker: Worker;
	private nextId = 1;
	private readonly pending = new Map<
		number,
		{ resolve: (value: unknown) => void; reject: (error: AppError) => void }
	>();
	private opening: Promise<string> | null = null;
	private migrated: Promise<void> | null = null;
	/** Non-null while a transaction owns the connection. */
	private gate: Promise<void> | null = null;
	/** True while this driver's outermost transaction body is running. */
	private inTransaction = false;

	constructor() {
		this.worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
		this.worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
			const response = event.data;
			const waiter = this.pending.get(response.id);
			if (!waiter) return;
			this.pending.delete(response.id);
			if (response.ok) waiter.resolve(response.value);
			else waiter.reject(response.error);
		};
	}

	/** Opens the OPFS database and runs migrations. Idempotent. */
	async open(): Promise<string> {
		const mode = await this.ensureConnected();
		await this.ensureMigrated();
		return mode;
	}

	/**
	 * Opens a throwaway in-memory database and runs migrations over it.
	 *
	 * Entered only from the unavailable-DB screen: same `SqlDriver` surface,
	 * nothing persists. A later `open()` recovery closes it — export first if
	 * it holds anything worth keeping (the status store asks).
	 */
	async openTemporary(): Promise<string> {
		const mode = await (this.send('open-temporary') as Promise<string>);
		this.opening = Promise.resolve(mode);
		this.migrated = null;
		await this.ensureMigrated();
		return mode;
	}

	/**
	 * Forget the current connection and open OPFS fresh.
	 *
	 * Recovery from temporary mode: the memory database is closed (export it
	 * first when it holds anything — the caller asks), then `open()` runs the
	 * real OPFS open plus migrations instead of reusing the cached connection.
	 */
	async recover(): Promise<string> {
		await this.send('close').catch(() => {});
		this.opening = null;
		this.migrated = null;
		return this.open();
	}

	/** Raw open without migrations. Migration itself runs on top of this. */
	private ensureConnected(): Promise<string> {
		this.opening ??= (this.send('open') as Promise<string>).then(
			(mode) => mode,
			(error: unknown) => {
				// A failed open must not poison the driver: Retry reuses this
				// singleton, so drop the cached rejection and let the next call
				// actually open again instead of replaying this failure forever.
				this.opening = null;
				throw error;
			}
		);
		return this.opening;
	}

	/**
	 * Run the v1→v25 chain once. The facade talks straight to the worker so a
	 * migration step never re-enters `ensureMigrated` (which would deadlock:
	 * `open` → `migrate` → `query` → `open` …).
	 */
	private ensureMigrated(): Promise<void> {
		this.migrated ??= (async () => {
			try {
				await this.ensureConnected();
				const raw: SqlDriver = {
					query: <T>(sql: string, params: SqlParam[] = []) => this.rawQuery<T>(sql, params),
					queryOne: async <T>(sql: string, params: SqlParam[] = []) =>
						(await this.rawQuery<T>(sql, params))[0],
					execute: (sql: string, params: SqlParam[] = []) => this.rawExecute(sql, params),
					script: (sql: string) => this.rawScript(sql),
					transaction: <T>(fn: () => Promise<T>) => this.rawTransaction(fn),
					exportFile: () => this.rawExport(),
					importFile: (bytes: Uint8Array) => this.rawImport(bytes),
					close: () => this.close()
				};
				await migrate(raw);
			} catch (error: unknown) {
				// Same poison rule as ensureConnected: a failed migration must
				// be re-runnable, otherwise every later query dead-ends here.
				this.migrated = null;
				throw error;
			}
		})();
		return this.migrated;
	}

	private send(
		op: WorkerRequest['op'],
		payload: Omit<WorkerRequest, 'id' | 'op'> = {},
		transfer: Transferable[] = []
	): Promise<unknown> {
		const id = this.nextId++;
		return new Promise<unknown>((resolve, reject) => {
			this.pending.set(id, { resolve, reject });
			this.worker.postMessage({ id, op, ...payload } satisfies WorkerRequest, transfer);
		});
	}

	/**
	 * Waits for any in-flight transaction to finish before running `fn`.
	 *
	 * Re-entrant, and it has to be: a transaction body is written against the
	 * same `SqlDriver` every repo uses (`await driver.execute(...)` inside
	 * `transaction`), so its own queries arrive here while *it* holds the gate. A
	 * gate that only released on the way out would wait for itself forever, which
	 * is why no write path could ever complete at runtime.
	 *
	 * ponytail: a *different* caller arriving mid-transaction joins it instead of
	 * waiting, so its write rolls back with the transaction if the body throws.
	 * Thread a per-transaction driver through `transaction(fn)` if that ever
	 * matters; for a single-teacher desktop app it does not.
	 */
	private async exclusive<T>(fn: () => Promise<T>): Promise<T> {
		if (this.inTransaction) return await fn();
		while (this.gate) await this.gate;
		let release!: () => void;
		this.gate = new Promise<void>((resolve) => {
			release = resolve;
		});
		try {
			return await fn();
		} finally {
			this.gate = null;
			release();
		}
	}

	async query<T = Record<string, unknown>>(sql: string, params: SqlParam[] = []): Promise<T[]> {
		await this.ensureMigrated();
		return this.rawQuery<T>(sql, params);
	}

	async queryOne<T = Record<string, unknown>>(
		sql: string,
		params: SqlParam[] = []
	): Promise<T | undefined> {
		const rows = await this.query<T>(sql, params);
		return rows[0];
	}

	async execute(sql: string, params: SqlParam[] = []): Promise<number> {
		await this.ensureMigrated();
		return this.rawExecute(sql, params);
	}

	async script(sql: string): Promise<void> {
		await this.ensureMigrated();
		await this.rawScript(sql);
	}

	async transaction<T>(fn: () => Promise<T>): Promise<T> {
		await this.ensureMigrated();
		return this.rawTransaction(fn);
	}

	/** The live database as a `.sqlite` file image (`sqlite3_serialize`). */
	async exportFile(): Promise<Uint8Array> {
		await this.ensureMigrated();
		return this.rawExport();
	}

	/** Replaces the database contents with an existing `.sqlite` file's bytes. */
	async importFile(bytes: Uint8Array): Promise<void> {
		await this.ensureMigrated();
		await this.rawImport(bytes);
		// The file is now different bytes with its own `user_version`; migrate
		// again on next use so an older restore walks forward like any database.
		this.migrated = null;
	}

	private rawQuery<T>(sql: string, params: SqlParam[] = []): Promise<T[]> {
		return this.ensureConnected().then(() =>
			this.exclusive(() => this.send('query', { sql, params }) as Promise<T[]>)
		);
	}

	private rawExecute(sql: string, params: SqlParam[] = []): Promise<number> {
		return this.ensureConnected().then(() =>
			this.exclusive(() => this.send('execute', { sql, params }) as Promise<number>)
		);
	}

	private rawScript(sql: string): Promise<void> {
		return this.ensureConnected().then(() =>
			this.exclusive(() => this.send('script', { sql }) as Promise<void>)
		);
	}

	private rawTransaction<T>(fn: () => Promise<T>): Promise<T> {
		// SQLite has no nested transactions, and a nested call has nothing of its
		// own to commit: the outermost body already brackets both.
		if (this.inTransaction) return fn();
		return this.ensureConnected().then(() =>
			this.exclusive(async () => {
				this.inTransaction = true;
				try {
					await this.send('execute', { sql: 'BEGIN' });
					try {
						const result = await fn();
						await this.send('execute', { sql: 'COMMIT' });
						return result;
					} catch (thrown) {
						await this.send('execute', { sql: 'ROLLBACK' }).catch(() => {});
						throw thrown;
					}
				} finally {
					this.inTransaction = false;
				}
			})
		);
	}

	private rawExport(): Promise<Uint8Array> {
		return this.ensureConnected().then(() =>
			this.exclusive(() => this.send('export') as Promise<Uint8Array>)
		);
	}

	private rawImport(bytes: Uint8Array): Promise<void> {
		return this.ensureConnected().then(() =>
			this.exclusive(
				() =>
					new Promise<void>((resolve, reject) => {
						const id = this.nextId++;
						const copy = new Uint8Array(bytes);
						this.pending.set(id, {
							resolve: () => resolve(),
							reject: (error) => reject(error)
						});
						this.worker.postMessage({ id, op: 'import', bytes: copy } satisfies WorkerRequest, [
							copy.buffer
						]);
					})
			)
		);
	}

	async close(): Promise<void> {
		await this.send('close');
		this.opening = null;
		this.migrated = null;
		this.worker.terminate();
	}
}
