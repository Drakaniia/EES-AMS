import { describe, expect, it } from 'vitest';
import { CURRENT_SCHEMA_VERSION } from '$lib/db/migrations';
import {
	dumpDatabase,
	exportDatabaseImage,
	readSchemaVersion,
	replaceDatabaseFromImage
} from '../database';
import { getDriver, looksLikeSqlite } from '$lib/db';
import { seedAttendance, useBackupFixture } from './fixture';

describe('the database image', () => {
	useBackupFixture();

	it('round-trips every row through the same driver', async () => {
		await seedAttendance();
		const image = await exportDatabaseImage();

		await replaceDatabaseFromImage(image);

		expect(await names()).toEqual(['Dela Cruz, Juan', 'Reyes, Maria']);
		expect(await absences()).toBe(1);
	});

	it('is a real SQLite file, not text', async () => {
		await seedAttendance();

		const image = await exportDatabaseImage();

		expect(looksLikeSqlite(image)).toBe(true);
	});

	it('carries the schema version, so an old archive still gets migrated', async () => {
		await seedAttendance();
		// What a v17-era archive carries in its header. The restore path then runs
		// the chain from there; proving the chain itself against a real v17
		// database is the migration suite's job, not this one's.
		await getDriver().script('PRAGMA user_version = 17');

		await getDriver().importFile(await exportDatabaseImage());

		expect(await readSchemaVersion()).toBe(17);
	});

	it('leaves no rows behind, so a restore cannot merge two databases', async () => {
		await seedAttendance();
		const image = await exportDatabaseImage();
		// A row written after the snapshot, which is exactly what a restore has to
		// destroy rather than keep.
		await getDriver().execute(
			"INSERT INTO students (id, name, class_id, created_at) VALUES ('s3', 'Reyes, Juan', 'c1', 1)"
		);

		await replaceDatabaseFromImage(image);

		expect(await names()).toEqual(['Dela Cruz, Juan', 'Reyes, Maria']);
	});

	it('carries text exactly, apostrophe included', async () => {
		await getDriver().execute(
			"INSERT INTO students (id, name, class_id, created_at) VALUES ('s9', 'O''Brien, Ana', NULL, 1)"
		);

		await replaceDatabaseFromImage(await exportDatabaseImage());

		expect(await names()).toContain("O'Brien, Ana");
	});

	it('reports the version of the live database', async () => {
		await seedAttendance();
		expect(await readSchemaVersion()).toBe(CURRENT_SCHEMA_VERSION);
	});
});

describe('the SQL dump', () => {
	useBackupFixture();

	it('opens with the schema stamp and one insert per row', async () => {
		await seedAttendance();

		const dump = await dumpDatabase();

		expect(dump).toMatch(/^PRAGMA user_version = \d+;$/m);
		expect(dump).toContain('INSERT INTO "students" VALUES');
		expect(dump).toContain('INSERT INTO "classes" VALUES');
	});
});

async function names(): Promise<string[]> {
	return (await getDriver().query<{ name: string }>('SELECT name FROM students ORDER BY name')).map(
		(row) => row.name
	);
}

async function absences(): Promise<number> {
	const row = await getDriver().queryOne<{ n: number }>(
		"SELECT COUNT(*) AS n FROM events WHERE event_type = 'absent'"
	);
	return Number(row?.n ?? 0);
}
