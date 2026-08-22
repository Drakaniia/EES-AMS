import { beforeEach, describe, expect, it, vi } from 'vitest';
// Type-only import: erased at runtime, so the singleton module is not
// evaluated here (each test re-imports it fresh via resetModules below).
import type { updateStore as UpdateStoreValue } from '$lib/stores/update.svelte';

type Store = typeof UpdateStoreValue;

const mocks = vi.hoisted(() => ({
	checkForUpdates: vi.fn(),
	getUpdateStatus: vi.fn(),
	downloadUpdate: vi.fn(),
	cancelUpdateDownload: vi.fn(),
	installStagedUpdate: vi.fn(),
	listen: vi.fn(async () => vi.fn())
}));

vi.mock('$lib/features/settings/update', () => ({
	checkForUpdates: mocks.checkForUpdates,
	getUpdateStatus: mocks.getUpdateStatus,
	downloadUpdate: mocks.downloadUpdate,
	cancelUpdateDownload: mocks.cancelUpdateDownload,
	installStagedUpdate: mocks.installStagedUpdate
}));

vi.mock('@tauri-apps/api/event', () => ({
	listen: mocks.listen
}));

async function freshStore(): Promise<Store> {
	// Test-only: module reload gives each test a pristine singleton,
	// intentionally exercising the module-loading boundary.
	vi.resetModules();
	const mod = await import('$lib/stores/update.svelte');
	return mod.updateStore;
}

const noError = (currentVersion = '0.5.1') => ({
	available: false,
	version: null,
	notes: null,
	pubDate: null,
	currentVersion,
	error: null
});

describe('update store', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mocks.getUpdateStatus.mockResolvedValue({
			currentVersion: '0.5.1',
			stagedVersion: null,
			stagedNotes: null,
			stagedPubDate: null
		});
		mocks.checkForUpdates.mockResolvedValue(noError());
		mocks.downloadUpdate.mockResolvedValue(undefined);
		mocks.installStagedUpdate.mockResolvedValue(undefined);
		mocks.cancelUpdateDownload.mockResolvedValue(undefined);
	});

	it('auto-checks and reports up to date when no update exists', async () => {
		const store = await freshStore();
		await store.init();

		expect(mocks.checkForUpdates).toHaveBeenCalledTimes(1);
		expect(store.status).toBe('upToDate');
		expect(store.currentVersion).toBe('0.5.1');
		expect(store.badgeVisible).toBe(false);
	});

	it('shows available state and badge when an update exists', async () => {
		mocks.checkForUpdates.mockResolvedValue({
			available: true,
			version: '0.5.2',
			notes: 'Fixes',
			pubDate: '2026-08-01',
			currentVersion: '0.5.1',
			error: null
		});
		const store = await freshStore();
		await store.init();

		expect(store.status).toBe('available');
		expect(store.updateInfo?.version).toBe('0.5.2');
		expect(store.badgeVisible).toBe(true);
	});

	it('restores a staged download without a network check', async () => {
		mocks.getUpdateStatus.mockResolvedValue({
			currentVersion: '0.5.1',
			stagedVersion: '0.5.2',
			stagedNotes: 'Fixes',
			stagedPubDate: '2026-08-01'
		});
		const store = await freshStore();
		await store.init();

		expect(mocks.checkForUpdates).not.toHaveBeenCalled();
		expect(store.status).toBe('readyToRestart');
		expect(store.stagedVersion).toBe('0.5.2');
		expect(store.badgeVisible).toBe(true);
	});

	it('surfaces a check failure as failed, not up to date', async () => {
		mocks.checkForUpdates.mockResolvedValue({
			available: false,
			version: null,
			notes: null,
			pubDate: null,
			currentVersion: '0.5.1',
			error: 'Could not reach the update server'
		});
		const store = await freshStore();
		await store.init();

		expect(store.status).toBe('failed');
		expect(store.failedStage).toBe('check');
		expect(store.error).toContain('update server');
		expect(store.badgeVisible).toBe(false);

		// Retry re-runs the check
		mocks.checkForUpdates.mockResolvedValue(noError());
		store.retry();
		await vi.waitFor(() => expect(store.status).toBe('upToDate'));
		expect(mocks.checkForUpdates).toHaveBeenCalledTimes(2);
	});

	it('transitions to readyToRestart after a successful download', async () => {
		mocks.checkForUpdates.mockResolvedValue({
			available: true,
			version: '0.5.2',
			notes: 'Fixes',
			pubDate: '2026-08-01',
			currentVersion: '0.5.1',
			error: null
		});
		const store = await freshStore();
		await store.init();
		await store.download();

		expect(mocks.downloadUpdate).toHaveBeenCalledTimes(1);
		expect(store.status).toBe('readyToRestart');
		expect(store.stagedVersion).toBe('0.5.2');
	});

	it('returns to available when the download is cancelled', async () => {
		mocks.checkForUpdates.mockResolvedValue({
			available: true,
			version: '0.5.2',
			notes: null,
			pubDate: null,
			currentVersion: '0.5.1',
			error: null
		});
		mocks.downloadUpdate.mockRejectedValue(new Error('Download cancelled'));
		const store = await freshStore();
		await store.init();
		await store.download();

		expect(store.status).toBe('available');
		expect(store.error).toBeNull();
		expect(store.failedStage).toBeNull();
	});

	it('shows a download error with retry that re-downloads', async () => {
		mocks.checkForUpdates.mockResolvedValue({
			available: true,
			version: '0.5.2',
			notes: null,
			pubDate: null,
			currentVersion: '0.5.1',
			error: null
		});
		mocks.downloadUpdate.mockRejectedValueOnce(new Error('Network drop'));
		const store = await freshStore();
		await store.init();
		await store.download();

		expect(store.status).toBe('failed');
		expect(store.failedStage).toBe('download');

		mocks.downloadUpdate.mockResolvedValue(undefined);
		store.retry();
		await vi.waitFor(() => expect(store.status).toBe('readyToRestart'));
		expect(mocks.downloadUpdate).toHaveBeenCalledTimes(2);
	});

	it('debounces manual refreshes to one check within the cooldown', async () => {
		const store = await freshStore();
		await store.refresh();
		await store.refresh();

		expect(mocks.checkForUpdates).toHaveBeenCalledTimes(1);
	});

	it('defers to a subtle state but keeps the badge', async () => {
		mocks.getUpdateStatus.mockResolvedValue({
			currentVersion: '0.5.1',
			stagedVersion: '0.5.2',
			stagedNotes: null,
			stagedPubDate: null
		});
		const store = await freshStore();
		await store.init();
		expect(store.status).toBe('readyToRestart');

		store.later();
		expect(store.status).toBe('deferred');
		expect(store.stagedVersion).toBe('0.5.2');
		expect(store.badgeVisible).toBe(true);
	});

	it('keeps staged state when an install fails and retry re-installs', async () => {
		mocks.getUpdateStatus.mockResolvedValue({
			currentVersion: '0.5.1',
			stagedVersion: '0.5.2',
			stagedNotes: null,
			stagedPubDate: null
		});
		mocks.installStagedUpdate.mockRejectedValueOnce(new Error('Install failed'));
		const store = await freshStore();
		await store.init();

		await store.restart();
		expect(store.status).toBe('failed');
		expect(store.failedStage).toBe('install');
		expect(store.stagedVersion).toBe('0.5.2');

		store.retry();
		await vi.waitFor(() => expect(store.status).toBe('upToDate'));
		expect(mocks.installStagedUpdate).toHaveBeenCalledTimes(2);
		expect(store.stagedVersion).toBeNull();
	});

	it('is a no-op when restarting without a staged update', async () => {
		const store = await freshStore();
		await store.init();
		await store.restart();

		expect(mocks.installStagedUpdate).not.toHaveBeenCalled();
	});
});
