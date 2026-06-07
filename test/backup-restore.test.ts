import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
	backupPathLabel,
	formatBackupBytes,
	googleDriveStatusLabel
} from '../src/lib/features/settings/backup';

const settingsPage = readFileSync('src/routes/settings/+page.svelte', 'utf8');
const settingsNative = readFileSync('src/lib/features/settings/native.ts', 'utf8');
const types = readFileSync('src/lib/types.ts', 'utf8');

test('settings page exposes safe backup and restore controls', () => {
	assert.match(settingsPage, /Back Up Now/);
	assert.match(settingsPage, /Restore Backup/);
	assert.match(settingsPage, /Connect Google Drive/);
	assert.match(settingsPage, /Upload Latest to Drive/);
	assert.match(settingsPage, /Choose Local Sync Folder/);
	assert.match(settingsPage, /Clear Sync Folder/);
	assert.match(settingsPage, /Import JSON Merge/);
	assert.match(settingsPage, /A pre-restore safety backup is created first/);
});

test('backup and restore Tauri wrappers are available to the frontend', () => {
	assert.match(settingsNative, /getBackupStatus/);
	assert.match(settingsNative, /createBackupNow/);
	assert.match(settingsNative, /connectGoogleDriveBackup/);
	assert.match(settingsNative, /uploadLatestBackupToGoogleDrive/);
	assert.match(settingsNative, /chooseRestoreBackup/);
	assert.match(settingsNative, /restoreBackup/);
});

test('backup display helpers format status values outside the settings route', () => {
	assert.equal(formatBackupBytes(512), '512 B');
	assert.equal(formatBackupBytes(1536), '1.5 KB');
	assert.equal(backupPathLabel('C:/Users/Qwenzy/backups/ees.db'), '...backups\\ees.db');
	assert.equal(googleDriveStatusLabel(null), 'OAuth not configured');
	assert.equal(
		googleDriveStatusLabel({
			localBackupDir: 'C:/backups',
			backupCount: 0,
			retentionLimit: 10,
			googleDriveConfigured: true,
			googleDriveConnected: false
		}),
		'Not connected'
	);
});

test('backup and restore TypeScript interfaces include preview counts and safety paths', () => {
	assert.match(types, /export interface BackupPreview/);
	assert.match(types, /studentCount: number/);
	assert.match(types, /sf2TemplateCount: number/);
	assert.match(types, /export interface RestoreResult/);
	assert.match(types, /preRestoreBackupPath: string/);
});
