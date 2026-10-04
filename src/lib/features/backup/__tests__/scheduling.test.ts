import { afterEach, describe, expect, it } from 'vitest';
import { listBackups } from '../list';
import {
	DEFAULT_BACKUP_INTERVAL_MS,
	isBackupScheduled,
	onAppQuit,
	scheduleBackups,
	stopScheduledBackups
} from '../scheduling';
import { seedAttendance, useBackupFixture } from './fixture';

const NOW = Date.parse('2026-09-28T14:25:30Z');

/**
 * D11 replaced the detached Rust thread with an in-page timer, so the thing worth
 * testing is no longer "a thread slept for an hour" but "the disposer really
 * stops it, and starting twice does not leave two timers running".
 */
describe('the scheduler', () => {
	useBackupFixture();
	afterEach(stopScheduledBackups);

	it('defaults to an hourly tick, like the Rust thread it replaces', () => {
		expect(DEFAULT_BACKUP_INTERVAL_MS).toBe(60 * 60 * 1000);
	});

	it('starts and stops through the disposer it returns', () => {
		const dispose = scheduleBackups();
		expect(isBackupScheduled()).toBe(true);

		dispose();
		expect(isBackupScheduled()).toBe(false);
	});

	it('leaves exactly one timer running when started twice', () => {
		scheduleBackups();
		scheduleBackups();

		expect(isBackupScheduled()).toBe(true);
		stopScheduledBackups();
		expect(isBackupScheduled()).toBe(false);
	});
});

describe('onAppQuit', () => {
	useBackupFixture();

	it('writes a snapshot and stops the timer, because it is the last one', async () => {
		await seedAttendance();
		scheduleBackups();

		const summary = await onAppQuit(NOW);

		expect(summary).toMatchObject({ kind: 'auto', includesDatabase: true });
		expect(isBackupScheduled()).toBe(false);
		expect(await listBackups()).toHaveLength(1);
	});

	it('writes one even when today already has a backup', async () => {
		await seedAttendance();
		const { ensureDailyBackup } = await import('../scheduling');
		expect(await ensureDailyBackup(NOW)).toBeDefined();

		expect(await onAppQuit(NOW + 60 * 1000)).toBeDefined();
		expect(await listBackups()).toHaveLength(2);
	});
});
