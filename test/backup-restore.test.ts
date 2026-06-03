import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const settingsPage = readFileSync('src/routes/settings/+page.svelte', 'utf8');
const dbRust = readFileSync('src/lib/db-rust.ts', 'utf8');
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
	assert.match(dbRust, /export async function getBackupStatus\(\): Promise<BackupStatus>/);
	assert.match(dbRust, /export async function createBackupNow\(\): Promise<BackupStatus>/);
	assert.match(dbRust, /export async function connectGoogleDriveBackup\(\): Promise<BackupStatus>/);
	assert.match(
		dbRust,
		/export async function uploadLatestBackupToGoogleDrive\(\): Promise<BackupStatus>/
	);
	assert.match(
		dbRust,
		/export async function chooseRestoreBackup\(\): Promise<BackupPreview \| null>/
	);
	assert.match(
		dbRust,
		/export async function restoreBackup\(sourcePath: string\): Promise<RestoreResult>/
	);
});

test('backup and restore TypeScript interfaces include preview counts and safety paths', () => {
	assert.match(types, /export interface BackupPreview/);
	assert.match(types, /studentCount: number/);
	assert.match(types, /sf2TemplateCount: number/);
	assert.match(types, /export interface RestoreResult/);
	assert.match(types, /preRestoreBackupPath: string/);
});
