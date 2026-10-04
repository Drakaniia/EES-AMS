import { beforeEach, describe, expect, it } from 'vitest';
import type { SettingsRecord } from '$lib/domain/settings';
import { clearAuditEvents, listAuditEvents } from '../audit';
import { getSettings, saveSettings } from '../settings';
import { db, useTestDb } from './schema';
import { SETTINGS_SCHEMA } from './schema-fixture';

/**
 * The settings row and the audit trail.
 *
 * Two claims carry the weight here. A save from the Settings page must not touch
 * a column the settings model does not own — `sf2_split_completed_at` going NULL
 * re-runs the whole 12-month split and rewrites every month file, and that bug
 * shipped as `INSERT OR REPLACE`. And clearing the trail must report how many
 * entries it destroyed, because the Records page tells the teacher it did.
 */

useTestDb();

const base: SettingsRecord = {
	id: 'app',
	dayStart: '08:00',
	dayEnd: '15:00',
	lateAfter: '08:45',
	quarter: '1st Quarter',
	attendanceMode: 'manual'
};

beforeEach(async () => {
	await db().script(SETTINGS_SCHEMA);
});

async function seedSettings(overrides: Partial<SettingsRecord> = {}): Promise<void> {
	await db().execute(
		`INSERT INTO settings (id, day_start, day_end, late_after, quarter, attendance_mode)
		 VALUES ('app', '08:00', '15:00', '08:45', '1st Quarter', 'manual')`
	);
	if (Object.keys(overrides).length > 0) await saveSettings({ ...base, ...overrides });
}

describe('getSettings', () => {
	it('reads the Rust defaults when no settings row exists', async () => {
		const settings = await getSettings();
		expect(settings.id).toBe('app');
		expect(settings.dayStart).toBe('08:00');
		expect(settings.lateAfter).toBe('08:45');
		expect(settings.quarter).toBe('1st Quarter');
	});

	it('replaces an unrecognised quarter with 3rd Quarter and normalises the mode', async () => {
		// 'card_reader' is the legacy mode stored by older installs; it reads as manual now.
		await db().execute(
			`INSERT INTO settings (id, day_start, day_end, late_after, quarter, attendance_mode)
			 VALUES ('app', '08:00', '15:00', '08:45', 'Midterm', 'card_reader')`
		);
		const settings = await getSettings();
		expect(settings.quarter).toBe('3rd Quarter');
		expect(settings.attendanceMode).toBe('manual');
	});
});

describe('saveSettings', () => {
	it('leaves a column the settings model does not carry alone', async () => {
		await seedSettings();
		await db().execute('UPDATE settings SET sf2_split_completed_at = 1712345678');

		await saveSettings({ ...base, lateAfter: '09:00' });

		const row = await db().queryOne<{ sf2_split_completed_at: number; late_after: string }>(
			'SELECT sf2_split_completed_at, late_after FROM settings WHERE id = ?',
			['app']
		);
		expect(row?.sf2_split_completed_at).toBe(1712345678);
		expect(row?.late_after).toBe('09:00');
	});

	it('writes every field it owns and returns the saved record', async () => {
		await saveSettings({
			...base,
			quarter: '2nd Quarter',
			q1Start: '2025-07-01',
			schoolName: 'Espiritu Elementary',
			adviserName: 'Dela Cruz'
		});

		const settings = await getSettings();
		expect(settings.quarter).toBe('2nd Quarter');
		expect(settings.attendanceMode).toBe('manual');
		expect(settings.q1Start).toBe('2025-07-01');
		expect(settings.schoolName).toBe('Espiritu Elementary');
		expect(settings.adviserName).toBe('Dela Cruz');
	});

	it('records a before and after payload in the trail', async () => {
		await seedSettings();
		await saveSettings({ ...base, lateAfter: '09:15' });

		const [event] = await listAuditEvents();
		expect(event.entityType).toBe('settings');
		expect(event.action).toBe('update');
		expect(event.actor).toBe('admin');
		expect(event.beforeJson).toContain('08:45');
		expect(event.afterJson).toContain('09:15');
	});

	it('rolls back and leaves nothing behind when the insert fails', async () => {
		await db().execute(
			`INSERT INTO settings (id, day_start, day_end, late_after, quarter)
			 VALUES ('other', '08:00', '15:00', '08:45', '1st Quarter')`
		);
		await db().execute('DROP TABLE audit_events');
		await expect(saveSettings({ ...base, lateAfter: '09:00' })).rejects.toBeTruthy();

		const rows = await db().query<{ id: string }>('SELECT id FROM settings');
		expect(rows.map((row) => row.id)).toEqual(['other']);
	});
});

describe('audit trail', () => {
	async function seedTrail(): Promise<void> {
		await db().execute(
			`INSERT INTO audit_events (id, entity_type, action, summary, created_at, actor)
			 VALUES ('a', 'settings', 'update', 'first', 100, 'admin'),
				('b', 'data_export', 'export', 'second', 200, 'admin'),
				('c', 'database', 'wipe', 'third', 150, 'admin')`
		);
	}

	it('lists newest first and clamps the limit', async () => {
		await seedTrail();
		expect((await listAuditEvents()).map((event) => event.id)).toEqual(['b', 'c', 'a']);
		expect((await listAuditEvents(2)).map((event) => event.id)).toEqual(['b', 'c']);
		expect(await listAuditEvents(0)).toHaveLength(1);
		expect(await listAuditEvents(99_999)).toHaveLength(3);
	});

	it('converts the stored unix seconds to an ISO timestamp', async () => {
		await seedTrail();
		const [event] = await listAuditEvents();
		expect(event.createdAt).toBe('1970-01-01T00:03:20.000Z');
	});

	it('returns how many entries clear destroyed', async () => {
		await seedTrail();
		expect(await clearAuditEvents()).toBe(3);
		expect(await clearAuditEvents()).toBe(0);
		expect(await listAuditEvents()).toEqual([]);
	});
});
