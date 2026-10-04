import { describe, expect, it } from 'vitest';
import {
	createClass,
	deleteClass,
	getClass,
	listClasses,
	updateClass
} from '$lib/db/repos/classes';
import { createStudent } from '$lib/db/repos/students';
import { db, useTestDb } from './schema';

useTestDb();

const base = { dayStart: '08:00', dayEnd: '15:00', lateAfter: '08:45', sessions: [], days: [] };

describe('listClasses', () => {
	it('returns every class ordered by name with its JSON columns decoded', async () => {
		await createClass({
			...base,
			name: 'Zara',
			room: 'Room 2',
			sessions: [{ name: 'AM', startTime: '08:00', endTime: '12:00', lateAfter: '08:45' }],
			days: [1, 3, 5]
		});
		await createClass({ ...base, name: 'Ana', room: 'Room 1' });

		const classes = await listClasses();
		expect(classes.map((c) => c.name)).toEqual(['Ana', 'Zara']);
		expect(classes[1].sessions).toEqual([
			{ name: 'AM', startTime: '08:00', endTime: '12:00', lateAfter: '08:45' }
		]);
		expect(classes[1].days).toEqual([1, 3, 5]);
	});

	it('falls back to the school week when days are unreadable', async () => {
		const created = await createClass({ ...base, name: 'Ana', room: 'Room 1' });
		await db().execute('UPDATE classes SET days = ?, sessions = ? WHERE id = ?', [
			'not json',
			null,
			created.id
		]);

		const read = await getClass(created.id);
		expect(read?.days).toEqual([1, 2, 3, 4, 5]);
		expect(read?.sessions).toEqual([]);
	});
});

describe('getClass', () => {
	it('returns undefined for an unknown id', async () => {
		expect(await getClass('missing')).toBeUndefined();
	});
});

describe('updateClass', () => {
	it('applies only the fields it was given', async () => {
		const created = await createClass({
			...base,
			name: 'Ana',
			room: 'Room 1',
			days: [1, 2]
		});

		const updated = await updateClass(created.id, { name: 'Ana B' });

		expect(updated.name).toBe('Ana B');
		expect(updated.room).toBe('Room 1');
		expect(updated.days).toEqual([1, 2]);
		expect(updated.createdAt).toBe(created.createdAt);
	});

	it('cannot clear the room: the column is NOT NULL from migration v2', async () => {
		const created = await createClass({ ...base, name: 'Ana', room: 'Room 1' });

		// The repo normalises a blank room to "no value" and writes NULL, exactly
		// as Rust did — and the schema has refused that since v2 added
		// `room TEXT NOT NULL DEFAULT 'N/A'`. Preserved rather than papered over:
		// class CRUD is read-only in the UI, so nothing reaches this path today.
		await expect(updateClass(created.id, { room: '' })).rejects.toMatchObject({
			kind: 'Database',
			detail: 'NOT NULL constraint failed: classes.room'
		});
		await expect(createClass({ ...base, name: 'Bea' })).rejects.toMatchObject({
			kind: 'Database',
			detail: 'NOT NULL constraint failed: classes.room'
		});
	});

	it('throws ClassNotFound for an unknown id', async () => {
		await expect(updateClass('missing', { name: 'X' })).rejects.toMatchObject({
			kind: 'ClassNotFound',
			detail: 'missing'
		});
	});
});

describe('deleteClass', () => {
	it('detaches students and events instead of cascading, and audits what it touched', async () => {
		const cls = await createClass({ ...base, name: 'Ana', room: 'Room 1' });
		const student = await createStudent({ name: 'Bea', classId: cls.id });
		await db().execute(
			'INSERT INTO events (id, student_id, class_id, event_type, timestamp) VALUES (?, ?, ?, ?, ?)',
			['e1', student.id, cls.id, 'in', 1]
		);
		await db().execute('INSERT INTO sf2_templates (id, active_class_id) VALUES (?, ?)', [
			't1',
			cls.id
		]);
		await db().execute('INSERT INTO attendance_day_status (id, class_id) VALUES (?, ?)', [
			'd1',
			cls.id
		]);

		await deleteClass(cls.id);

		expect(await getClass(cls.id)).toBeUndefined();
		expect(await db().query('SELECT class_id FROM students')).toEqual([{ class_id: null }]);
		expect(await db().query('SELECT class_id FROM events')).toEqual([{ class_id: null }]);
		expect(await db().query('SELECT * FROM sf2_templates')).toHaveLength(0);
		expect(await db().query('SELECT * FROM attendance_day_status')).toHaveLength(0);

		const audit = await db().query<{ metadata_json: string }>(
			"SELECT metadata_json FROM audit_events WHERE entity_type = 'class' AND action = 'delete'"
		);
		expect(JSON.parse(audit[0].metadata_json)).toEqual({
			affectedStudents: 1,
			affectedEvents: 1,
			deletedSf2Templates: 1
		});
	});

	it('throws ClassNotFound for an unknown id', async () => {
		await expect(deleteClass('missing')).rejects.toMatchObject({ kind: 'ClassNotFound' });
	});
});
