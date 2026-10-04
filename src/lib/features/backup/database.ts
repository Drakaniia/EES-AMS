/**
 * Getting the database into an archive and putting it back.
 *
 * This replaces the Rust `Connection::backup` / `Connection::restore` pair, and
 * it now does what they did: the archive carries the **real database file
 * image**, not a re-playable dump. The bytes come from one
 * `sqlite3_serialize()` call inside the worker (the `export` op), and a restore
 * hands them back through `import`, which makes them the OPFS file itself — so
 * an archive holds a file any SQLite client can open, and a restore cannot
 * half-apply: the image either lands as the database or the old one stays.
 *
 * The schema stamp lives in the image's header (`user_version`), which is what
 * still lets an archive written by an older build come back and be migrated by
 * the current chain.
 *
 * `dumpDatabase()` remains for the *other* direction — the `.sql` export a
 * teacher can read in a text editor — where text is the point.
 */

import { getDriver, internal, type SqlDriver, type SqlValue } from '$lib/db';
import { migrate } from '$lib/db/migrations';
import type { ManifestCounts } from './manifest';

/** Tables whose rows are counted into the manifest. A missing table counts 0. */
const COUNTED_TABLES = [
	'students',
	'classes',
	'events',
	'settings',
	'sf2_templates',
	'sf2_month_templates'
] as const;

/**
 * The live database as a `.sqlite` file image — the archive's `db.sqlite`.
 *
 * On the Worker driver this is a single `sqlite3_serialize()` round trip; the
 * Node driver behind tests answers the same contract, so the round trip
 * (backup → wipe → restore) is exercised against real bytes either way.
 */
export async function exportDatabaseImage(): Promise<Uint8Array> {
	return getDriver().exportFile();
}

/**
 * Replace the live database with an image, then bring it up to the current
 * schema.
 *
 * The swap is the driver's `import` op — the image *becomes* the database file
 * — and the migration chain runs afterwards, which is what lets an archive
 * from an older schema come back without a data conversion: it arrives exactly
 * as it left, then walks forward like any other database.
 */
export async function replaceDatabaseFromImage(image: Uint8Array): Promise<number> {
	const driver = getDriver();
	await driver.importFile(image);
	await migrate(driver);
	return readSchemaVersion();
}

/**
 * The whole database as SQL statements — the `.sql` **export**, not the archive.
 *
 * One `INSERT` per row, tables in name order, no `BEGIN`/`COMMIT`: a caller
 * that pastes the dump into a client wraps it in its own transaction, and a
 * dump that opened one would nest a transaction inside it and be rejected by
 * SQLite.
 *
 * The leading `PRAGMA user_version` is what makes the dump loadable against an
 * old schema. `user_version` is database state rather than table state, so a
 * dump that omitted it would replay an older schema's rows into the *current*
 * tables and then read `CURRENT` back — the migration chain would find nothing
 * to do, and a v17 dump would come back missing every column v18 to v25 added.
 */
export async function dumpDatabase(): Promise<string> {
	const driver = getDriver();
	const statements = [`PRAGMA user_version = ${await readSchemaVersion()};`];
	for (const table of await tableNames(driver)) {
		const quoted = quoteIdentifier(table);
		const rows = await driver.query<Record<string, SqlValue>>(`SELECT * FROM ${quoted}`);
		for (const row of rows) {
			const values = Object.values(row).map(sqlLiteral).join(', ');
			statements.push(`INSERT INTO ${quoted} VALUES (${values});`);
		}
	}
	return `${statements.join('\n')}\n`;
}

/** `PRAGMA user_version` of the live database. */
export async function readSchemaVersion(): Promise<number> {
	const row = await getDriver().queryOne<{ user_version: number }>('PRAGMA user_version');
	return Number(row?.user_version ?? 0);
}

/** The row counts a restore preview compares a workbook's X marks against. */
export async function readDatabaseCounts(): Promise<ManifestCounts> {
	const driver = getDriver();
	const counts = {} as Record<string, number>;
	for (const table of COUNTED_TABLES) {
		counts[table] = await countTableRows(driver, table);
	}
	return {
		students: counts.students,
		classes: counts.classes,
		events: counts.events,
		absent: await countAbsentEvents(driver),
		settings: counts.settings,
		sf2Templates: counts.sf2_templates,
		sf2MonthTemplates: counts.sf2_month_templates
	};
}

async function countAbsentEvents(driver: SqlDriver): Promise<number> {
	if (!(await tableExists(driver, 'events'))) return 0;
	const row = await driver.queryOne<{ total: number }>(
		"SELECT COUNT(*) AS total FROM events WHERE event_type = 'absent'"
	);
	return Number(row?.total ?? 0);
}

async function countTableRows(driver: SqlDriver, table: string): Promise<number> {
	if (!(await tableExists(driver, table))) return 0;
	const row = await driver.queryOne<{ total: number }>(
		`SELECT COUNT(*) AS total FROM ${quoteIdentifier(table)}`
	);
	return Number(row?.total ?? 0);
}

async function tableExists(driver: SqlDriver, table: string): Promise<boolean> {
	const row = await driver.queryOne<{ total: number }>(
		"SELECT COUNT(*) AS total FROM sqlite_master WHERE type = 'table' AND name = ?",
		[table]
	);
	return Number(row?.total ?? 0) > 0;
}

/** Every user table, in name order, so a dump is byte-stable for the same data. */
export async function tableNames(driver: SqlDriver): Promise<string[]> {
	const rows = await driver.query<{ name: string }>(
		"SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
	);
	return rows.map((row) => row.name);
}

/**
 * Table names cannot be bound parameters, so they are quoted after a character
 * check rather than escaped.
 */
function quoteIdentifier(name: string): string {
	if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
		throw internal(`refusing to quote the table name ${name}`);
	}
	return `"${name}"`;
}

function sqlLiteral(value: SqlValue): string {
	if (value === null) return 'NULL';
	if (typeof value === 'bigint' || typeof value === 'number') return String(value);
	if (ArrayBuffer.isView(value)) {
		const bytes = new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
		return `X'${[...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('')}'`;
	}
	return `'${value.replace(/'/g, "''")}'`;
}
