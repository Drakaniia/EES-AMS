import { describe, expect, it } from 'vitest';
import { deleteStudent, getStudent, listStudents, saveStudent, uid } from '$lib/api/students';
import { addEvent } from '$lib/api/events';
import { db, useApiFixture, seedRoster } from './fixture';

useApiFixture();

describe('listStudents', () => {
	it('returns the seeded rows ordered by name', async () => {
		await seedRoster();

		expect((await listStudents()).map((student) => student.name)).toEqual([
			'Bautista, Ana',
			'Dela Cruz, Juan',
			'Reyes, Maria'
		]);
	});

	it('filters by class id and returns nothing for a class with no students', async () => {
		await seedRoster();
		await db().execute('UPDATE students SET class_id = ? WHERE id = ?', ['class-2', 's1']);

		expect((await listStudents('class-1')).map((student) => student.name)).toEqual([
			'Bautista, Ana',
			'Reyes, Maria'
		]);
		expect(await listStudents('class-2')).toHaveLength(1);
	});
});

describe('saveStudent', () => {
	it('creates a row when the student has no id, and the row is readable back', async () => {
		await seedRoster();

		const created = await saveStudent({
			id: '',
			name: 'Roxas, Liza',
			gender: 'female',
			classId: 'class-1',
			createdAt: ''
		});

		expect(created.id).not.toBe('');
		const stored = await getStudent(created.id);
		expect(stored).toMatchObject({
			name: 'Roxas, Liza',
			gender: 'female',
			classId: 'class-1'
		});
		expect(new Date(stored.createdAt).toISOString()).toBe(stored.createdAt);
	});

	it('updates the same row when the student has an id', async () => {
		await seedRoster();

		const saved = await saveStudent({
			id: 's2',
			name: 'Reyes, Maria Jr.',
			gender: 'female',
			classId: 'class-1',
			createdAt: ''
		});

		expect(saved.name).toBe('Reyes, Maria Jr.');
		expect((await getStudent('s2')).name).toBe('Reyes, Maria Jr.');
		expect(await listStudents()).toHaveLength(3);
	});
});

describe('deleteStudent', () => {
	it('removes the student and the attendance that belonged to them', async () => {
		await seedRoster();
		await addEvent({ studentId: 's1', classId: 'class-1', type: 'absent' });

		await deleteStudent('s1');

		expect((await listStudents()).map((student) => student.name)).toEqual([
			'Bautista, Ana',
			'Reyes, Maria'
		]);
		const rows = await db().query<{ total: number }>(
			'SELECT COUNT(*) AS total FROM events WHERE student_id = ?',
			['s1']
		);
		expect(Number(rows[0]?.total)).toBe(0);
	});

	it('rejects for a student that is not on file', async () => {
		await expect(deleteStudent('nobody')).rejects.toMatchObject({ kind: 'StudentNotFound' });
	});
});

describe('uid', () => {
	it('mints a distinct id each call', () => {
		expect(uid()).not.toBe(uid());
	});
});
