import { beforeEach, describe, expect, it, vi } from 'vitest';
import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
import { openBackupFolder } from '../ui';
import { ROOT, expectAppError, useBackupFixture } from './fixture';

vi.mock('@tauri-apps/plugin-opener', () => ({
	openPath: vi.fn(),
	revealItemInDir: vi.fn()
}));

describe('openBackupFolder', () => {
	useBackupFixture();

	beforeEach(() => {
		vi.mocked(openPath).mockReset();
		vi.mocked(revealItemInDir).mockReset();
	});

	it('opens the backup folder with the OS', async () => {
		vi.mocked(openPath).mockResolvedValue(undefined);
		await expect(openBackupFolder()).resolves.toBe(`${ROOT}/backups`);
		expect(revealItemInDir).not.toHaveBeenCalled();
	});

	it('reveals the folder in its parent when opening it directly fails', async () => {
		vi.mocked(openPath).mockRejectedValue(new Error('no app for folders'));
		vi.mocked(revealItemInDir).mockResolvedValue(undefined);
		await expect(openBackupFolder()).resolves.toBe(`${ROOT}/backups`);
		expect(revealItemInDir).toHaveBeenCalledWith(`${ROOT}/backups`);
	});

	it('keeps both reasons when neither opener works', async () => {
		vi.mocked(openPath).mockRejectedValue(new Error('no app for folders'));
		vi.mocked(revealItemInDir).mockRejectedValue(new Error('denied'));
		await expectAppError(openBackupFolder(), /no app for folders.*denied/s);
	});
});
