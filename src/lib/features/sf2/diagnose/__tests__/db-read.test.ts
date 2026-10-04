import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useDriver, type SqlDriver } from '$lib/db';
import { NodeSqlDriver } from '$lib/db/node-driver';
import { activeClassId, anchorTemplate, readSnapshot, schoolYear } from '../db-read';
import { classAbsences, matchRosterByName, rosterFor } from '../roster';
import { DIAGNOSTIC_STATEMENTS } from '../sql';
import {
	CLASS_ID,
	createSchema,
	DEFAULT_ABSENCES,
	localDateOf,
	seedInstall,
	snapshotTables,
	TEMPLATE_ID
} from './install-fixture';

/**
 * Every database read the diagnostic makes.
 *
 * Ported from `db_read.rs` together with the two Rust tests that covered it: the
 * `no_write` module, which proved a full run leaves the database byte-for-byte
 * identical and that the read side contains no writing statement. Neither carries over
 * literally - see `./sql` for why the read-only *handle* cannot - so what is asserted
 * here is the property that does carry over: the rows are identical afterwards, and
 * every statement is a read.
 */

let driver: SqlDriver;

beforeEach(() => {
	driver = new NodeSqlDriver();
	useDriver(driver);
});

afterEach(async () => {
	useDriver(null);
	await driver.close();
});

/** The four absences in the fixture, read back as the SQL derived them. */
async function absencesOfInstall() {
	await seedInstall(driver);
	const snapshot = await readSnapshot();
	return snapshot;
}

describe('reading the install', () => {
	it('reads the schema version, the templates and the events', async () => {
		const snapshot = await absencesOfInstall();

		expect(snapshot.schemaVersion).toBe(19);
		expect(snapshot.legacyTemplates).toHaveLength(1);
		expect(snapshot.monthTemplates).toHaveLength(1);
		expect(snapshot.monthTemplates[0]?.reportYear).toBe(2026);
	});

	it('anchors on the legacy template row, which is the one that names the workbook', async () => {
		const snapshot = await absencesOfInstall();

		expect(anchorTemplate(snapshot)?.id).toBe(TEMPLATE_ID);
		expect(anchorTemplate(snapshot)?.reportMonth).toBe('SEPTEMBER');
		expect(activeClassId(snapshot)).toBe(CLASS_ID);
		expect(schoolYear(snapshot)).toBe('2026 - 2027');
	});

	it("derives each absence's local day the way the app's writer does", async () => {
		const snapshot = await absencesOfInstall();

		expect(snapshot.absentEvents.map((record) => record.date)).toEqual(
			DEFAULT_ABSENCES.map((absence) => localDateOf(absence.date))
		);
	});

	it('counts the absent events and leaves the present event out of it', async () => {
		const snapshot = await absencesOfInstall();

		expect(snapshot.totalAbsentEvents).toBe(4);
		expect(snapshot.eventCounts).toContainEqual({ eventType: 'absent', rows: 4 });
		expect(snapshot.eventCounts).toContainEqual({ eventType: 'in', rows: 1 });
	});

	it('reads the students it needs to attribute an absence to a class', async () => {
		const snapshot = await absencesOfInstall();

		expect(snapshot.students).toHaveLength(4);
		expect(snapshot.students.find((student) => student.id === 's1')?.classId).toBe(CLASS_ID);
		expect(snapshot.students.find((student) => student.id === 's9')?.classId).toBe('other-class');
	});
});

describe('the mapping tables', () => {
	it('counts the rows of all six tables', async () => {
		const snapshot = await absencesOfInstall();

		expect(snapshot.tables.legacy).toEqual([
			{ table: 'sf2_templates', exists: true, rows: 1 },
			{ table: 'sf2_student_mappings', exists: true, rows: 3 },
			{ table: 'sf2_date_mappings', exists: true, rows: 2 }
		]);
		expect(snapshot.tables.perMonth).toEqual([
			{ table: 'sf2_month_templates', exists: true, rows: 1 },
			{ table: 'sf2_month_student_mappings', exists: true, rows: 3 },
			{ table: 'sf2_month_date_mappings', exists: true, rows: 2 }
		]);
	});

	it('reports a table a pre-v19 database has never heard of as missing, not as empty', async () => {
		await seedInstall(driver, { withMonthTables: false, schemaVersion: 18 });

		const snapshot = await readSnapshot();

		// `None` is not `0`. Reporting a missing table as zero would say "the backfill
		// copied nothing" when the truth is "the backfill cannot have run yet".
		expect(snapshot.schemaVersion).toBe(18);
		expect(snapshot.tables.perMonth).toEqual([
			{ table: 'sf2_month_templates', exists: false, rows: undefined },
			{ table: 'sf2_month_student_mappings', exists: false, rows: undefined },
			{ table: 'sf2_month_date_mappings', exists: false, rows: undefined }
		]);
		expect(snapshot.monthTemplates).toEqual([]);
		expect(snapshot.tables.monthDateMappingGrids).toEqual([]);
	});

	it('summarises both stored grids through the one shared reader', async () => {
		const snapshot = await absencesOfInstall();

		// One reader serves both files, so both column lists have to match it. A
		// mismatch fails as a column *type* error about a column the caller never asked
		// for, which is how this went wrong twice while the Rust was written.
		expect(snapshot.tables.legacyDateMappingSheets).toEqual([
			{
				sheetName: 'SEPTEMBER 2026',
				yearMonth: '2026-09',
				firstDate: '2026-09-01',
				lastDate: '2026-09-02',
				dayColumns: 2
			}
		]);
		expect(snapshot.tables.monthDateMappingGrids).toEqual([
			{
				sheetName: '',
				yearMonth: '2026-09',
				firstDate: '2026-09-01',
				lastDate: '2026-09-02',
				dayColumns: 2
			}
		]);
	});
});

describe('rosterFor', () => {
	it('prefers the per-month roster, because the legacy one predates a split', async () => {
		const snapshot = await absencesOfInstall();

		const { rows, source } = rosterFor(snapshot, TEMPLATE_ID);

		expect(source).toBe('perMonthTables');
		expect(rows).toHaveLength(3);
		expect(rows.map((row) => row.rowIndex)).toEqual([8, 9, 30]);
	});

	it('falls back to the legacy roster when the per-month one is empty', async () => {
		await seedInstall(driver);
		await driver.execute('DELETE FROM sf2_month_student_mappings');

		const { rows, source } = rosterFor(await readSnapshot(), TEMPLATE_ID);

		expect(source).toBe('legacyTables');
		expect(rows).toHaveLength(3);
	});

	it('reports no source when neither table has a row', async () => {
		await seedInstall(driver);
		await driver.execute('DELETE FROM sf2_month_student_mappings');
		await driver.execute('DELETE FROM sf2_student_mappings');

		expect(rosterFor(await readSnapshot(), TEMPLATE_ID)).toEqual({
			rows: [],
			source: 'none'
		});
	});
});

describe('which absences belong to this class', () => {
	it('counts an absence the event names the class for', async () => {
		await seedInstall(driver, {
			absences: [{ id: 'e1', studentId: 's1', classId: CLASS_ID, date: '2026-09-01' }]
		});

		expect(classAbsences(await readSnapshot(), CLASS_ID)).toHaveLength(1);
	});

	it('counts an absence against another class when the student is in this one', async () => {
		await seedInstall(driver, {
			absences: [{ id: 'e1', studentId: 's1', classId: 'other-class', date: '2026-09-01' }]
		});

		// Same test as the app's `event_belongs_to_class`: the event names the class, or
		// the student is in it.
		expect(classAbsences(await readSnapshot(), CLASS_ID)).toHaveLength(1);
	});

	it('counts an absence with no class at all when the student is in this one', async () => {
		await seedInstall(driver, {
			absences: [{ id: 'e1', studentId: 's1', date: '2026-09-01' }]
		});

		expect(classAbsences(await readSnapshot(), CLASS_ID)).toHaveLength(1);
	});

	it('leaves out an absence for a student of another class with no class of its own', async () => {
		await seedInstall(driver, {
			absences: [{ id: 'e1', studentId: 's9', classId: 'other-class', date: '2026-09-01' }]
		});

		expect(classAbsences(await readSnapshot(), CLASS_ID)).toHaveLength(0);
	});
});

describe('matching a workbook roster by name', () => {
	it('places a learner whose name normalises to a student row', async () => {
		const snapshot = await absencesOfInstall();

		const matched = matchRosterByName(
			[{ studentId: '', workbookName: 'ALVARADO, ZYRON JAY  E.', rowIndex: 8 }],
			snapshot
		);

		expect(matched).toEqual([
			{ studentId: 's1', workbookName: 'ALVARADO, ZYRON JAY  E.', rowIndex: 8 }
		]);
	});

	it('places nothing for a name the database does not hold', async () => {
		const snapshot = await absencesOfInstall();

		const matched = matchRosterByName(
			[
				{ studentId: '', workbookName: 'ALVARADO, ZYRON JAY  E.', rowIndex: 8 },
				{ studentId: '', workbookName: 'WHOEVER, STRANGER', rowIndex: 9 }
			],
			snapshot
		);

		expect(matched).toHaveLength(1);
	});
});

// ── zero writes ───────────────────────────────────────────────────────────

describe('zero writes', () => {
	it('leaves every row of every table exactly as it was', async () => {
		await seedInstall(driver);
		const before = await snapshotTables(driver);

		await readSnapshot();

		expect(await snapshotTables(driver)).toEqual(before);
	});

	it('contains no statement that writes', () => {
		for (const statement of DIAGNOSTIC_STATEMENTS) {
			const body = statement
				.split('\n')
				.filter((line) => !line.trimStart().startsWith('--'))
				.join('\n')
				.toUpperCase();
			for (const forbidden of [
				'INSERT',
				'UPDATE ',
				'DELETE',
				'DROP ',
				'ALTER ',
				'REPLACE',
				'CREATE',
				'VACUUM',
				'REINDEX',
				'ATTACH',
				'DETACH',
				'PRAGMA USER_VERSION =',
				'JOURNAL_MODE'
			]) {
				expect(body, statement).not.toContain(forbidden);
			}
		}
	});

	it('covers the whole read side, so the check keeps meaning something', async () => {
		// Twelve statements: schema version, table exists, row count, both template
		// reads, both roster reads, both grid summaries, absences, event counts, students.
		// If this falls over, a statement was added next to its reader and left out of
		// the list the no-write check walks.
		expect(DIAGNOSTIC_STATEMENTS).toHaveLength(12);
	});

	it('reports no schema version when the database has never been stamped', async () => {
		await createSchema(driver);

		// `PRAGMA user_version` on a fresh file reads 0, and the reader keeps that as a
		// version rather than inventing one.
		expect((await readSnapshot()).schemaVersion).toBe(0);
	});
});
