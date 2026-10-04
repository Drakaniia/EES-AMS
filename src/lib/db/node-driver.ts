import type { SqlDriver, SqlParam } from './driver';
import { looksLikeSqlite } from './driver';
import { appError } from './error';
import { randomUUID } from 'node:crypto';
import { readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';

/**
 * `node:sqlite` driver — for Vitest only.
 *
 * Same `SqlDriver` surface as the Worker, so every repo and every query runs
 * real SQL in tests with no browser and no native dependency to install. Never
 * bundled into the app; `client.ts` is the runtime driver.
 */
export class NodeSqlDriver implements SqlDriver {
	private readonly filename: string;
	private db: DatabaseSync;
	/** The temp file a `:memory:` connection was moved onto by `importFile`. */
	private ownedFile: string | null = null;

	constructor(filename = ':memory:') {
		this.filename = filename;
		this.db = new DatabaseSync(filename);
		this.db.exec('PRAGMA foreign_keys = ON');
	}

	async query<T = Record<string, unknown>>(sql: string, params: SqlParam[] = []): Promise<T[]> {
		const statement = this.db.prepare(sql);
		try {
			statement.setAllowBareNamedParameters(true);
			return statement.all(...params) as T[];
		} catch (thrown) {
			throw appError('Database', thrown instanceof Error ? thrown.message : String(thrown));
		}
	}

	async queryOne<T = Record<string, unknown>>(
		sql: string,
		params: SqlParam[] = []
	): Promise<T | undefined> {
		return (await this.query<T>(sql, params))[0];
	}

	async execute(sql: string, params: SqlParam[] = []): Promise<number> {
		const statement = this.db.prepare(sql);
		try {
			const result = statement.run(...params);
			return Number(result.changes);
		} catch (thrown) {
			throw appError('Database', thrown instanceof Error ? thrown.message : String(thrown));
		}
	}

	async script(sql: string): Promise<void> {
		try {
			this.db.exec(sql);
		} catch (thrown) {
			throw appError('Database', thrown instanceof Error ? thrown.message : String(thrown));
		}
	}

	async transaction<T>(fn: () => Promise<T>): Promise<T> {
		await this.execute('BEGIN');
		try {
			const result = await fn();
			await this.execute('COMMIT');
			return result;
		} catch (thrown) {
			await this.execute('ROLLBACK').catch(() => {});
			throw thrown;
		}
	}

	async close(): Promise<void> {
		this.db.close();
		if (this.ownedFile) {
			const file = this.ownedFile;
			this.ownedFile = null;
			await rm(file, { force: true });
		}
	}

	/**
	 * The live database as a `.sqlite` file image.
	 *
	 * `node:sqlite` exposes neither `serialize()` nor `backup()`, so the image
	 * comes from `VACUUM INTO`: a fully-checkpointed copy of every page, with
	 * `user_version` carried over — which is what makes an old archive
	 * migratable after a restore. The temp file is read and removed before this
	 * returns; on the Worker driver this is one `sqlite3_serialize()` call.
	 */
	async exportFile(): Promise<Uint8Array> {
		const target = join(tmpdir(), `ees-ams-export-${process.pid}-${randomUUID()}.sqlite`);
		try {
			this.db.exec(`VACUUM INTO '${target.replace(/'/g, "''")}'`);
			return new Uint8Array(await readFile(target));
		} catch (thrown) {
			throw appError('Database', thrown instanceof Error ? thrown.message : String(thrown));
		} finally {
			await rm(target, { force: true });
		}
	}

	/**
	 * Replace the whole database with a `.sqlite` file image.
	 *
	 * `node:sqlite` has no `deserialize()`, so the bytes become the file this
	 * connection opens: a `:memory:` driver moves onto a temp file (removed by
	 * `close()`), a file-backed one is overwritten in place. Either way the next
	 * query runs against the image, and so does every query after a reopen —
	 * which is the property a restore is judged on.
	 */
	async importFile(bytes: Uint8Array): Promise<void> {
		if (!looksLikeSqlite(bytes)) {
			throw appError('InvalidInput', 'import requires the bytes of a SQLite database file');
		}
		const target =
			this.filename === ':memory:'
				? join(tmpdir(), `ees-ams-import-${process.pid}-${randomUUID()}.sqlite`)
				: this.filename;
		// The handle goes first: it must stop using the file before its bytes are
		// replaced, and a temp-file import outlives this call, so `close()` owns
		// its removal.
		this.db.close();
		try {
			await writeFile(target, bytes);
			this.db = new DatabaseSync(target);
			this.db.exec('PRAGMA foreign_keys = ON');
		} catch (thrown) {
			throw appError('Database', thrown instanceof Error ? thrown.message : String(thrown));
		}
		if (target !== this.filename) {
			const stale = this.ownedFile;
			this.ownedFile = target;
			if (stale) await rm(stale, { force: true });
		}
	}
}
