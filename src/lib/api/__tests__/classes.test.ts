import { describe, expect, it } from 'vitest';
import { getClass, listClasses } from '$lib/api/classes';
import { CLASS_ID, db, seedRoster, useApiFixture } from './fixture';

useApiFixture();

describe('listClasses', () => {
	it('returns the seeded class with its JSON columns decoded', async () => {
		await seedRoster();

		const classes = await listClasses();

		expect(classes).toEqual([
			{
				id: CLASS_ID,
				name: 'Grade 3 - Matapat',
				room: 'Room 1',
				dayStart: '08:00',
				dayEnd: '15:00',
				lateAfter: '08:45',
				sessions: [],
				days: [1, 2, 3, 4, 5],
				createdAt: new Date(1000 * 1000).toISOString()
			}
		]);
	});

	it('falls back to the school week when a stored day list is unreadable', async () => {
		await seedRoster();
		await db().execute(`UPDATE classes SET days = 'not json', sessions = NULL WHERE id = ?`, [
			CLASS_ID
		]);

		const cls = await getClass(CLASS_ID);

		expect(cls?.days).toEqual([1, 2, 3, 4, 5]);
		expect(cls?.sessions).toEqual([]);
	});
});

describe('getClass', () => {
	it('reports undefined for a class that is not on file', async () => {
		expect(await getClass('class-nope')).toBeUndefined();
	});
});
