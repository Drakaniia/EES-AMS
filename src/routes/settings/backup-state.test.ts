import { beforeEach, describe, expect, it, vi } from 'vitest';
import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
import { useApiFixture } from '$lib/api/__tests__/fixture';
import { backupState } from './backup-state.svelte';
import type { Ctx } from './state-context';

vi.mock('@tauri-apps/plugin-opener', () => ({
	openPath: vi.fn(),
	revealItemInDir: vi.fn()
}));

describe('backup-state open folder toast', () => {
	useApiFixture();

	const toasts: string[] = [];
	beforeEach(() => {
		toasts.length = 0;
		vi.mocked(openPath).mockReset();
		vi.mocked(revealItemInDir).mockReset();
		backupState.init({
			toast: (msg: string) => {
				toasts.push(msg);
			},
			reload: async () => {},
			hasUnsavedGlobalSettings: () => false
		} satisfies Ctx);
	});

	it('names the real reason instead of repeating the fallback', async () => {
		vi.mocked(openPath).mockRejectedValue(new Error('no app for folders'));
		vi.mocked(revealItemInDir).mockRejectedValue(new Error('denied'));
		await backupState.onOpenBackupFolder();
		expect(toasts).toHaveLength(1);
		expect(toasts[0]).not.toBe('Failed to open backup folder: Failed to open backup folder');
		expect(toasts[0]).toMatch(/no app for folders/);
	});

	it('confirms when the folder opens', async () => {
		vi.mocked(openPath).mockResolvedValue(undefined);
		await backupState.onOpenBackupFolder();
		expect(toasts).toEqual(['Backup folder opened']);
	});
});
