import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { DatabaseSync } from 'node:sqlite';
import type { SqlDriver, SqlParam } from './driver';
import { appError } from './error';
import { CURRENT_SCHEMA_VERSION, migrate, storedSchemaVersion } from './migrations';
import { useDriver } from './index';

/**
 * `NodeSqlDriver` without the `statement.finalize()` calls: `node:sqlite`'s
 * `StatementSync` has no such method, so every query through it throws
 * `statement.finalize is not a function`. The shared driver owns that fix -
 * this copy exists so the migration chain can be verified against real SQLite
 * in the meantime, and is deleted in its favour.
 */
class TestDriver implements SqlDriver {
	private readonly db = new DatabaseSync(':memory:');

	private static fail(thrown: unknown): never {
		throw appError('Database', thrown instanceof Error ? thrown.message : String(thrown));
	}

	async query<T = Record<string, unknown>>(sql: string, params: SqlParam[] = []): Promise<T[]> {
		try {
			const statement = this.db.prepare(sql);
			statement.setAllowBareNamedParameters(true);
			return statement.all(...params) as T[];
		} catch (thrown) {
			return TestDriver.fail(thrown);
		}
	}

	async queryOne<T = Record<string, unknown>>(
		sql: string,
		params: SqlParam[] = []
	): Promise<T | undefined> {
		return (await this.query<T>(sql, params))[0];
	}

	async execute(sql: string, params: SqlParam[] = []): Promise<number> {
		try {
			return Number(this.db.prepare(sql).run(...params).changes);
		} catch (thrown) {
			return TestDriver.fail(thrown);
		}
	}

	async script(sql: string): Promise<void> {
		try {
			this.db.exec(sql);
		} catch (thrown) {
			TestDriver.fail(thrown);
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

	async exportFile(): Promise<Uint8Array> {
		throw appError('Internal', 'the migration suite does not move database images');
	}

	async importFile(_bytes: Uint8Array): Promise<void> {
		throw appError('Internal', 'the migration suite does not move database images');
	}

	async close(): Promise<void> {
		this.db.close();
	}
}

/**
 * The migration chain against real SQLite (`node:sqlite`), which is the point:
 * these assertions are about the SQL actually running, not about a mock.
 */
describe('migrate', () => {
	let driver: TestDriver;

	beforeEach(() => {
		driver = new TestDriver();
		useDriver(driver);
	});

	afterEach(async () => {
		useDriver(null);
		await driver.close();
	});

	const tables = async (): Promise<string[]> => {
		const rows = await driver.query<{ name: string }>(
			"SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name"
		);
		return rows.map((row) => row.name);
	};

	const columns = async (table: string): Promise<string[]> => {
		const rows = await driver.query<{ name: string }>(
			`SELECT name FROM pragma_table_info('${table}')`
		);
		return rows.map((row) => row.name);
	};

	it('leaves a fresh database at the current schema version', async () => {
		expect(await storedSchemaVersion(driver)).toBe(0);

		await migrate(driver);

		expect(await storedSchemaVersion(driver)).toBe(CURRENT_SCHEMA_VERSION);
	});

	it('is a no-op the second time', async () => {
		await migrate(driver);
		const after = await storedSchemaVersion(driver);
		const before = await tables();

		await migrate(driver);

		expect(await storedSchemaVersion(driver)).toBe(after);
		expect(await tables()).toEqual(before);
	});

	it('builds the tables the repos read', async () => {
		await migrate(driver);

		const created = await tables();
		for (const table of [
			'students',
			'classes',
			'events',
			'settings',
			'sf2_month_templates',
			'sf2_month_student_mappings',
			'sf2_month_date_mappings',
			'audit_events'
		]) {
			expect(created).toContain(table);
		}

		// `events` is the record of every attendance mark: its identity columns
		// plus everything v11 and v17's rebuilds carried across.
		expect(await columns('events')).toEqual(
			expect.arrayContaining([
				'id',
				'student_id',
				'class_id',
				'event_type',
				'timestamp',
				'note',
				'session_key',
				'override_reason',
				'updated_at'
			])
		);
		expect(await columns('students')).toEqual(
			expect.arrayContaining(['id', 'name', 'card_serial', 'class_id', 'gender', 'sf2_learner_id'])
		);
		// v24 restored the sheet dimension to the per-month date grid.
		expect(await columns('sf2_month_date_mappings')).toContain('sheet_name');
	});

	it('normalises a school year typed the way the DepEd form prints it', async () => {
		await migrate(driver);
		await driver.execute("UPDATE settings SET school_year = '2026 - 2027' WHERE id = 'app'");
		await driver.script('PRAGMA user_version = 22');

		await migrate(driver);

		const row = await driver.queryOne<{ school_year: string }>(
			"SELECT school_year FROM settings WHERE id = 'app'"
		);
		expect(row?.school_year).toBe('2026-2027');
		expect(await storedSchemaVersion(driver)).toBe(CURRENT_SCHEMA_VERSION);
	});
});
