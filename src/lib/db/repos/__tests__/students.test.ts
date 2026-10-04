import { describe, expect, it } from 'vitest';
import {
	createStudent,
	createStudents,
	deleteStudent,
	getStudent,
	listStudents,
	saveStudent,
	updateStudent
} from '$lib/db/repos/students';
import { db, useTestDb } from './schema';

useTestDb();

describe('listStudents', () => {
	it('returns every student ordered by name', async () => {
		await createStudent({ name: 'Zara' });
		await createStudent({ name: 'Ana' });

		const students = await listStudents();

		expect(students.map((s) => s.name)).toEqual(['Ana', 'Zara']);
	});

	it('filters by class id', async () => {
		await createStudent({ name: 'Ana', classId: 'c1' });
		await createStudent({ name: 'Zara', classId: 'c2' });

		expect((await listStudents('c1')).map((s) => s.name)).toEqual(['Ana']);
	});

	it('exposes the DepEd learner id the SF2 roster owns', async () => {
		const student = await createStudent({ name: 'Ana' });
		await db().execute('UPDATE students SET sf2_learner_id = ? WHERE id = ?', [
			'LRN-1',
			student.id
		]);

		expect((await getStudent(student.id)).sf2LearnerId).toBe('LRN-1');
	});
});

describe('createStudent', () => {
	it('trims text and treats blank as absent', async () => {
		const student = await createStudent({ name: 'Ana', classId: '  c1  ' });

		expect(student.classId).toBe('c1');
		expect(student.gender).toBeUndefined();
		expect(student.sf2LearnerId).toBeUndefined();
	});

	it('round-trips the stored row through the ISO contract', async () => {
		const created = await createStudent({
			name: 'Ana',
			gender: 'female',
			classId: 'c1'
		});

		const read = await getStudent(created.id);
		expect(read).toEqual(created);
		expect(read.gender).toBe('female');
		expect(new Date(read.createdAt).toISOString()).toBe(read.createdAt);
	});
});

describe('getStudent', () => {
	it('throws StudentNotFound for an unknown id', async () => {
		await expect(getStudent('missing')).rejects.toMatchObject({
			kind: 'StudentNotFound',
			detail: 'missing'
		});
	});
});

describe('updateStudent', () => {
	it('leaves sf2_learner_id and created_at alone', async () => {
		const created = await createStudent({ name: 'Ana' });
		await db().execute('UPDATE students SET sf2_learner_id = ? WHERE id = ?', [
			'LRN-1',
			created.id
		]);

		const updated = await updateStudent(created.id, { name: 'Ana Maria' });

		expect(updated.name).toBe('Ana Maria');
		expect(updated.sf2LearnerId).toBe('LRN-1');
		expect(updated.createdAt).toBe(created.createdAt);
	});

	it('throws StudentNotFound for an unknown id', async () => {
		await expect(updateStudent('missing', { name: 'X' })).rejects.toMatchObject({
			kind: 'StudentNotFound'
		});
	});
});

describe('saveStudent', () => {
	it('creates when there is no id and updates when there is', async () => {
		// `id: ''` is how "no id" reaches `saveStudent`: `Student.id` is a required
		// `string`, and the upsert dispatches on truthiness - an absent id means
		// create. A real UUID here would take the update path and fail to find a row.
		const created = await saveStudent({ id: '', name: 'Ana', createdAt: new Date().toISOString() });
		expect(await listStudents()).toHaveLength(1);

		const updated = await saveStudent({ ...created, name: 'Ana Maria' });
		expect(updated.id).toBe(created.id);
		expect(updated.name).toBe('Ana Maria');
		expect(await listStudents()).toHaveLength(1);
	});
});

describe('createStudents', () => {
	it('creates the whole batch and audits each row', async () => {
		const students = await createStudents([{ name: 'Ana' }, { name: 'Zara', classId: 'c1' }]);

		expect(students).toHaveLength(2);
		const audit = await db().query<{ entity_id: string; action: string }>(
			"SELECT entity_id, action FROM audit_events WHERE entity_type = 'student' ORDER BY entity_id"
		);
		expect(audit.map((row) => row.action)).toEqual(['create', 'create']);
	});
});

describe('deleteStudent', () => {
	it('removes the student, their events and their SF2 mappings, and audits the count', async () => {
		const student = await createStudent({ name: 'Ana', classId: 'c1' });
		await db().execute(
			'INSERT INTO events (id, student_id, class_id, event_type, timestamp) VALUES (?, ?, ?, ?, ?)',
			['e1', student.id, 'c1', 'in', 1]
		);
		await db().execute('INSERT INTO sf2_student_mappings (id, student_id) VALUES (?, ?)', [
			'm1',
			student.id
		]);
		await db().execute(
			`INSERT INTO sf2_month_student_mappings
			   (template_id, student_id, workbook_name, normalized_name, row_index)
			 VALUES ('t1', ?, 'ANA', 'ana', 8)`,
			[student.id]
		);

		await deleteStudent(student.id);

		expect(await listStudents()).toHaveLength(0);
		expect(await db().query('SELECT * FROM events')).toHaveLength(0);
		expect(await db().query('SELECT * FROM sf2_student_mappings')).toHaveLength(0);
		// The month mappings too: the roster sync that follows a delete rebuilds the
		// roster from `students`, and a mapping left behind would put the row back.
		expect(await db().query('SELECT * FROM sf2_month_student_mappings')).toHaveLength(0);
		const audit = await db().query<{ action: string; metadata_json: string }>(
			"SELECT action, metadata_json FROM audit_events WHERE entity_type = 'student' AND action = 'delete'"
		);
		expect(audit).toHaveLength(1);
		expect(JSON.parse(audit[0].metadata_json)).toEqual({ deletedEvents: 1 });
	});

	it('throws StudentNotFound for an unknown id', async () => {
		await expect(deleteStudent('missing')).rejects.toMatchObject({ kind: 'StudentNotFound' });
	});
});
