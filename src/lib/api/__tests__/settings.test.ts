import { describe, expect, it } from 'vitest';
import { clearAuditEvents, getSettings, listAuditEvents, saveSettings } from '$lib/api/settings';
import { saveStudent } from '$lib/api/students';
import { db, seedRoster, seedSettings, useApiFixture } from './fixture';

useApiFixture();

describe('getSettings', () => {
	it('reads the seeded row through, quarters and all', async () => {
		await seedSettings();
		await db().execute(`UPDATE settings SET q2_start = '2026-12-01' WHERE id = 'app'`);

		expect(await getSettings()).toMatchObject({
			id: 'app',
			dayStart: '08:00',
			dayEnd: '15:00',
			lateAfter: '08:45',
			quarter: '1st Quarter',
			attendanceMode: 'manual',
			q2Start: '2026-12-01'
		});
	});

	it('falls back to the product defaults for a database with no settings row', async () => {
		expect(await getSettings()).toMatchObject({
			id: 'app',
			dayStart: '08:00',
			quarter: '1st Quarter'
		});
	});
});

describe('saveSettings', () => {
	it('normalises an unknown quarter', async () => {
		await seedSettings();

		const saved = await saveSettings({
			id: 'app',
			dayStart: '08:30',
			dayEnd: '15:30',
			lateAfter: '08:45',
			quarter: 'Fourth Quarter',
			attendanceMode: 'card_reader'
		});

		expect(saved.quarter).toBe('3rd Quarter');
		expect((await getSettings()).dayStart).toBe('08:30');
		expect((await getSettings()).attendanceMode).toBe('card_reader');
	});

	it('writes an audit entry for the save', async () => {
		await seedSettings();

		await saveSettings({
			id: 'app',
			dayStart: '08:30',
			dayEnd: '15:30',
			lateAfter: '08:45',
			quarter: '2nd Quarter',
			attendanceMode: 'manual'
		});

		const entries = await listAuditEvents();
		expect(entries[0]).toMatchObject({ entityType: 'settings', action: 'update' });
	});
});

describe('listAuditEvents', () => {
	it('returns newest first and honours the limit', async () => {
		await seedRoster();
		await saveStudent({ id: '', name: 'Ana', classId: 'class-1', createdAt: '' });
		await saveStudent({ id: '', name: 'Zara', classId: 'class-1', createdAt: '' });

		const one = await listAuditEvents(1);

		expect(one).toHaveLength(1);
		expect(await listAuditEvents()).toHaveLength(2);
	});

	it('defaults to a limit rather than returning the whole trail', async () => {
		expect(await listAuditEvents()).toEqual([]);
	});
});

describe('clearAuditEvents', () => {
	it('reports how many entries it destroyed', async () => {
		await seedRoster();
		await saveStudent({ id: '', name: 'Ana', classId: 'class-1', createdAt: '' });

		expect(await clearAuditEvents()).toBe(1);
		expect(await listAuditEvents()).toEqual([]);
	});
});
