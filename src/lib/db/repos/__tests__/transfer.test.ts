import { beforeEach, describe, expect, it } from 'vitest';
import type { ExportDataRecord } from '$lib/domain/settings';
import { getSettings } from '../settings';
import { exportAll, importAll, wipeAll } from '../transfer';
import { db, useTestDb } from './schema';
import { SETTINGS_SCHEMA } from './schema-fixture';

/**
 * Export, import and wipe.
 *
 * The claim that matters is the round-trip. `Export JSON` → `wipe all` →
 * `Import JSON` used to rewrite every recorded absence as a present, because the
 * import hardcoded `event_type = 'in'`; absence is the only record of an X mark,
 * so that was a one-click data-loss path. These tests pin the whole snapshot -
 * absences included - and pin that a wipe empties every table it claims to.
 */

useTestDb();

beforeEach(async () => {
	await db().script(SETTINGS_SCHEMA);
});

async function seedRoster(): Promise<void> {
	await db().execute(
		`INSERT INTO students (id, name, gender, card_serial, class_id, sf2_learner_id, created_at)
		 VALUES ('s1', 'Dela Cruz, Jr.', 'male', 'CARD-1', 'c1', 'LRN-7', 1700000000),
		        ('s2', 'Santos', 'female', NULL, NULL, NULL, 1700000100)`
	);
	await db().execute(
		`INSERT INTO classes (id, name, room, day_start, day_end, late_after, sessions, days, created_at)
		 VALUES ('c1', 'Mabini, Grade 1', 'Room 1', '08:00', '15:00', '08:45', '[]', '[1,2,3,4,5]', 1700000000)`
	);
	await db().execute(
		`INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
		 VALUES ('e1', 's1', 'c1', 'in', 1700000200, NULL, '2023-11-15|c1|day', NULL, NULL),
		        ('e2', 's1', 'c1', 'absent', 1700000300, 'Sick', '2023-11-15|c1|day', 'excused', NULL),
		        ('e3', 's2', NULL, 'in', 1700000400, NULL, NULL, NULL, NULL)`
	);
	await db().execute(
		`INSERT INTO attendance_event_audit (id, student_id, action, reason, created_at)
		 VALUES ('aa1', 's1', 'create_override', 'card not read', 1700000000)`
	);
	await db().execute(`INSERT INTO sf2_templates (id) VALUES ('t1')`);
}

describe('exportAll', () => {
	it('snapshots every table with the Rust ordering and shapes', async () => {
		await seedRoster();
		const snapshot = await exportAll();

		expect(snapshot.students.map((student) => student.id)).toEqual(['s1', 's2']);
		expect(snapshot.students[0].sf2LearnerId).toBe('LRN-7');
		expect(snapshot.students[0].createdAt).toBe('2023-11-14T22:13:20.000Z');
		expect(snapshot.classes.map((schoolClass) => schoolClass.id)).toEqual(['c1']);
		expect(snapshot.classes[0].days).toEqual([1, 2, 3, 4, 5]);
		// Events newest first, and `type` keeps its meaning.
		expect(snapshot.events.map((event) => event.id)).toEqual(['e3', 'e2', 'e1']);
		expect(snapshot.events[1].type).toBe('absent');
		expect(snapshot.events[1].note).toBe('Sick');
		expect(snapshot.settings).toHaveLength(1);
		expect(snapshot.auditEvents).toEqual([]);
		expect(typeof snapshot.exportedAt).toBe('number');
	});

	it('defaults an unparseable class week to Monday-Friday', async () => {
		await db().execute(
			`INSERT INTO classes (id, name, day_start, day_end, late_after, sessions, days, created_at)
			 VALUES ('c9', 'Broken', '08:00', '15:00', '08:45', 'not json', 'also not json', 1)`
		);
		const snapshot = await exportAll();
		expect(snapshot.classes[0].sessions).toEqual([]);
		expect(snapshot.classes[0].days).toEqual([1, 2, 3, 4, 5]);
	});
});

describe('importAll', () => {
	it('round-trips a wipe: absences stay absences', async () => {
		await seedRoster();
		const snapshot = await exportAll();

		await wipeAll();
		expect(await exportAll()).toMatchObject({ students: [], classes: [], events: [] });

		await importAll(snapshot);

		const restored = await exportAll();
		expect(restored.students.map((student) => student.id)).toEqual(['s1', 's2']);
		expect(restored.events.map((event) => [event.id, event.type])).toEqual([
			['e3', 'in'],
			['e2', 'absent'],
			['e1', 'in']
		]);
		expect(restored.students[0].sf2LearnerId).toBe('LRN-7');
	});

	it('derives a missing session key from the local date and class', async () => {
		const timestamp = '2023-11-15T02:00:00.000Z';
		const payload: ExportDataRecord = {
			students: [],
			classes: [],
			events: [
				{
					id: 'e1',
					studentId: 's1',
					classId: ' c1 ',
					type: 'in',
					timestamp
				}
			],
			settings: [],
			exportedAt: 1
		};
		await db().execute(
			`INSERT INTO students (id, name, created_at) VALUES ('s1', 'Dela Cruz, Jr.', 1)`
		);

		await importAll(payload);

		const row = await db().queryOne<{ session_key: string }>(
			'SELECT session_key FROM events WHERE id = ?',
			['e1']
		);
		const local = new Date(timestamp);
		const day = `${local.getFullYear()}-${`${local.getMonth() + 1}`.padStart(2, '0')}-${`${local.getDate()}`.padStart(2, '0')}`;
		expect(row?.session_key).toBe(`${day}|c1|day`);
	});

	it('upserts rather than duplicating, and normalises an unknown quarter', async () => {
		await seedRoster();
		const snapshot = await exportAll();
		snapshot.settings[0].quarter = 'Midterm';
		snapshot.students[0].name = 'Dela Cruz III';

		await importAll(snapshot);
		await importAll(snapshot);

		const students = await db().query<{ id: string; name: string }>(
			'SELECT id, name FROM students'
		);
		expect(students).toEqual([
			{ id: 's1', name: 'Dela Cruz III' },
			{ id: 's2', name: 'Santos' }
		]);
		expect((await getSettings()).quarter).toBe('3rd Quarter');
	});

	it('rejects a payload whose timestamps cannot be read', async () => {
		await expect(
			importAll({
				students: [{ id: 's1', name: 'X', createdAt: 'not a date' }],
				classes: [],
				events: [],
				settings: [],
				exportedAt: 1
			})
		).rejects.toMatchObject({ kind: 'InvalidInput' });
		const students = await db().query('SELECT id FROM students');
		expect(students).toEqual([]);
	});
});

describe('wipeAll', () => {
	it('empties every table it claims to and reports the counts', async () => {
		await seedRoster();
		await db().execute(
			`INSERT INTO settings (id, day_start, day_end, late_after, quarter)
			 VALUES ('app', '08:00', '15:00', '08:45', '1st Quarter')`
		);
		await db().execute(
			`INSERT INTO audit_events (id, entity_type, action, summary, created_at, actor)
			 VALUES ('a1', 'students', 'create', 'Added Dela Cruz', 100, 'admin')`
		);

		const outcome = await wipeAll('C:/Documents/EES-AMS/backups/pre-wipe.zip');

		expect(outcome).toEqual({
			deletedStudents: 2,
			deletedClasses: 1,
			deletedEvents: 3,
			preWipeBackupPath: 'C:/Documents/EES-AMS/backups/pre-wipe.zip'
		});
		for (const table of [
			'students',
			'classes',
			'events',
			'attendance_event_audit',
			'sf2_templates'
		]) {
			const rows = await db().query(`SELECT * FROM ${table}`);
			expect(rows, table).toEqual([]);
		}
		// The trail is the record of the wipe, so it survives and gains an entry.
		const trail = await db().query<{ action: string; metadata_json: string }>(
			'SELECT action, metadata_json FROM audit_events'
		);
		expect(trail.map((row) => row.action)).toContain('wipe');
		expect(trail.find((row) => row.action === 'wipe')?.metadata_json).toContain('pre-wipe.zip');
	});

	it('leaves a fresh settings row for the school day', async () => {
		await db().execute(
			`INSERT INTO settings (id, day_start, day_end, late_after, quarter)
			 VALUES ('app', '07:00', '16:00', '08:00', '1st Quarter')`
		);

		await wipeAll();

		expect(await getSettings()).toMatchObject({
			id: 'app',
			dayStart: '08:30',
			dayEnd: '15:30',
			lateAfter: '08:45',
			quarter: '1st Quarter',
			attendanceMode: 'manual'
		});
	});
});
