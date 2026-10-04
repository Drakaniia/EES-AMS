import { describe, expect, it } from 'vitest';
import { buildArchive, readArchive } from '../archive';
import { createBackup, createWorkbooksBackup, pruneBackups } from '../create';
import { formatTimestamp, getBackupStatus, listBackups } from '../list';
import { MANIFEST_FILE_NAME, MANIFEST_FORMAT_VERSION, parseManifest } from '../manifest';
import { ensureDailyBackup } from '../scheduling';
import { DEFAULT_RETENTION_LIMIT, getRetentionLimit, setRetentionLimit } from '../state';
import {
	ROOT,
	expectAppError,
	expectAppErrorSync,
	seedAttendance,
	useBackupFixture
} from './fixture';

const NOW = Date.parse('2026-09-28T14:25:30Z');
const MINUTE = 60 * 1000;

/** The archive name a timestamp produces, in whatever timezone the test runs in. */
const nameAt = (offsetMs: number): string => `EES-AMS-${formatTimestamp(NOW + offsetMs)}.zip`;

describe('archive', () => {
	useBackupFixture();

	it('round-trips named files', () => {
		const built = buildArchive([
			{ path: 'b.txt', bytes: new TextEncoder().encode('second') },
			{ path: 'a.txt', bytes: new TextEncoder().encode('first') }
		]);

		expect(readArchive(built).files.map((file) => file.path)).toEqual(['a.txt', 'b.txt']);
	});

	it('writes the same bytes for the same inputs, whatever order they came in', () => {
		const entries = [
			{ path: 'workbooks/a.xlsx', bytes: new Uint8Array([1, 2, 3]) },
			{ path: 'manifest.json', bytes: new TextEncoder().encode('{}') }
		];
		expect(buildArchive(entries)).toEqual(buildArchive([...entries].reverse()));
	});

	it('refuses a manifest this build cannot honestly restore', () => {
		expectAppErrorSync(() => parseManifest(JSON.stringify({ formatVersion: 99 })), /format 99/);
		expectAppErrorSync(() => parseManifest('{ not json'), /not valid JSON/);
		expectAppErrorSync(
			() =>
				parseManifest(
					JSON.stringify({
						formatVersion: MANIFEST_FORMAT_VERSION,
						createdAt: 'yesterday'
					})
				),
			/unreadable createdAt/
		);
		expectAppErrorSync(
			() => parseManifest(JSON.stringify({ formatVersion: MANIFEST_FORMAT_VERSION })),
			/missing/
		);
	});

	it('refuses a dump-era archive whole, rather than reading its image as text', () => {
		// Format 1 carried `database.sql`; format 2 carries `db.sqlite`. D12 says
		// old zips need not restore, and a clear refusal beats a misread payload.
		expectAppErrorSync(
			() => parseManifest(JSON.stringify({ formatVersion: 1, createdAt: '2026-09-28T14:25:30Z' })),
			/Restore it with the version of the app that took it/
		);
	});
});

describe('listing', () => {
	const fs = useBackupFixture();

	it('returns archives newest first', async () => {
		await seedAttendance();
		await createBackup('manual', NOW);
		await createBackup('auto', NOW + MINUTE);
		await createBackup('manual', NOW + 2 * MINUTE);

		expect((await listBackups()).map((entry) => entry.fileName)).toEqual([
			nameAt(2 * MINUTE),
			nameAt(MINUTE),
			nameAt(0)
		]);
	});

	it('never lets two backups in the same second overwrite each other', async () => {
		await seedAttendance();
		const first = await createBackup('manual', NOW);
		const second = await createBackup('manual', NOW);

		expect(second.fileName).toBe(`${nameAt(0).replace(/\.zip$/, '')}-2.zip`);
		expect(await fs.exists(first.path)).toBe(true);
		expect(await listBackups()).toHaveLength(2);
	});

	it('lists an archive whose manifest cannot be parsed, without believing it', async () => {
		await fs.writeFileAtomic(
			`${ROOT}/backups/EES-AMS-20260101-010101.zip`,
			buildArchive([{ path: MANIFEST_FILE_NAME, bytes: new TextEncoder().encode('broken') }])
		);

		const [summary] = await listBackups();

		expect(summary).toMatchObject({
			kind: 'unknown',
			includesDatabase: false,
			workbookCount: 0,
			createdAt: Math.floor(Date.parse('2026-01-01T01:01:01') / 1000)
		});
	});
});

describe('retention', () => {
	useBackupFixture();

	it('defaults to keeping twenty archives', async () => {
		expect(DEFAULT_RETENTION_LIMIT).toBe(20);
		expect(await getRetentionLimit()).toBe(20);
	});

	it('prunes the oldest archives beyond the limit', async () => {
		await seedAttendance();
		for (let index = 0; index < 5; index += 1) await createBackup('auto', NOW + index * MINUTE);

		await setRetentionLimit(2);
		expect((await pruneBackups()).length).toBe(3);
		expect((await listBackups()).map((entry) => entry.fileName)).toEqual([
			nameAt(4 * MINUTE),
			nameAt(3 * MINUTE)
		]);
	});

	it('applies a lowered limit without waiting for the next backup', async () => {
		await seedAttendance();
		for (let index = 0; index < 4; index += 1) await createBackup('auto', NOW + index * MINUTE);

		await setRetentionLimit(1);
		await pruneBackups();

		expect((await listBackups()).map((entry) => entry.fileName)).toEqual([nameAt(3 * MINUTE)]);
		expect((await getBackupStatus()).retentionLimit).toBe(1);
	});

	it('refuses a retention limit that is not a whole number of archives', async () => {
		await expectAppError(setRetentionLimit(0), /at least 1/);
		await expectAppError(setRetentionLimit(2.5), /at least 1/);
	});
});

describe('workbooks-only backup', () => {
	useBackupFixture();

	it('carries no database, so it does not satisfy the daily check', async () => {
		await seedAttendance();
		const backup = await createWorkbooksBackup(NOW);

		expect(backup.includesDatabase).toBe(false);
		expect((await getBackupStatus()).lastWorkbooksBackupPath).toBe(backup.path);
		// The attendance records have still not been backed up today.
		expect(await ensureDailyBackup(NOW)).toMatchObject({ kind: 'auto', includesDatabase: true });
	});
});

describe('daily guard', () => {
	useBackupFixture();

	it('takes one automatic backup a day, not one per tick', async () => {
		await seedAttendance();
		expect(await ensureDailyBackup(NOW)).toBeDefined();
		expect(await ensureDailyBackup(NOW + MINUTE)).toBeUndefined();
		expect(await ensureDailyBackup(NOW + 2 * MINUTE)).toBeUndefined();
		expect(await listBackups()).toHaveLength(1);
	});
});
