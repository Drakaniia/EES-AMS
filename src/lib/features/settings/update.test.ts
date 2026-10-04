import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DownloadEvent } from '@tauri-apps/plugin-updater';

const mocks = vi.hoisted(() => ({
	check: vi.fn(),
	openUrl: vi.fn(),
	emit: vi.fn(),
	getVersion: vi.fn(),
	createBackup: vi.fn()
}));

vi.mock('@tauri-apps/plugin-updater', () => ({ check: mocks.check }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: mocks.openUrl }));
vi.mock('@tauri-apps/api/event', () => ({ emit: mocks.emit }));
vi.mock('@tauri-apps/api/app', () => ({ getVersion: mocks.getVersion }));
vi.mock('$lib/features/backup/create', () => ({ createBackup: mocks.createBackup }));

const STARTED: DownloadEvent = { event: 'Started', data: { contentLength: 1000 } };
const CHUNK: DownloadEvent = { event: 'Progress', data: { chunkLength: 400 } };
const FINISHED: DownloadEvent = { event: 'Finished' };

interface FakeUpdateOptions {
	version?: string;
	body?: string;
	date?: string;
	events?: DownloadEvent[];
	duringDownload?: () => void | Promise<void>;
}

/** Stands in for the plugin's `Update`: metadata plus the three lifecycle calls. */
function fakeUpdate(options: FakeUpdateOptions = {}) {
	return {
		version: options.version ?? '0.9.0',
		body: options.body,
		date: options.date,
		close: vi.fn(async () => {}),
		download: vi.fn(async (onEvent?: (event: DownloadEvent) => void) => {
			for (const event of options.events ?? []) onEvent?.(event);
			await options.duringDownload?.();
		}),
		install: vi.fn(async () => {})
	};
}

/**
 * The staged update lives in module state, so every test needs its own module
 * instance rather than a shared one carrying the previous test's download.
 */
async function loadApi() {
	vi.resetModules();
	return await import('./update');
}

beforeEach(() => {
	vi.clearAllMocks();
	mocks.getVersion.mockResolvedValue('0.8.1');
	mocks.emit.mockResolvedValue(undefined);
	mocks.createBackup.mockResolvedValue({ path: 'C:/backups/pre-update.zip' });
});

describe('checkForUpdates', () => {
	it('reports an available update with its notes and publish date', async () => {
		const update = fakeUpdate({
			version: '0.9.0',
			body: 'Fixed the roster grid.',
			date: '2026-09-01T10:00:00Z'
		});
		mocks.check.mockResolvedValue(update);
		const api = await loadApi();

		expect(await api.checkForUpdates()).toEqual({
			available: true,
			version: '0.9.0',
			notes: 'Fixed the roster grid.',
			pubDate: '2026-09-01T10:00:00Z',
			currentVersion: '0.8.1',
			error: null
		});
		expect(update.close).toHaveBeenCalled();
	});

	it('reports no update without an error when the server has nothing newer', async () => {
		mocks.check.mockResolvedValue(null);
		const api = await loadApi();

		expect(await api.checkForUpdates()).toEqual({
			available: false,
			version: null,
			notes: null,
			pubDate: null,
			currentVersion: '0.8.1',
			error: null
		});
	});

	it('separates a failed check from "up to date" through the error field', async () => {
		mocks.check.mockRejectedValue(new Error('connection refused'));
		const api = await loadApi();

		expect(await api.checkForUpdates()).toEqual({
			available: false,
			version: null,
			notes: null,
			pubDate: null,
			currentVersion: '0.8.1',
			error: 'Could not reach the update server: connection refused'
		});
	});

	it('leaves the notes and date null when the manifest carries neither', async () => {
		mocks.check.mockResolvedValue(fakeUpdate({ version: '0.9.0' }));
		const api = await loadApi();

		const info = await api.checkForUpdates();
		expect(info.notes).toBeNull();
		expect(info.pubDate).toBeNull();
	});
});

describe('downloadUpdate', () => {
	it('emits progress as the chunks arrive, then stages the update', async () => {
		const update = fakeUpdate({
			version: '0.9.0',
			body: 'Fixed the roster grid.',
			date: '2026-09-01T10:00:00Z',
			events: [STARTED, CHUNK, CHUNK, FINISHED]
		});
		mocks.check.mockResolvedValue(update);
		const api = await loadApi();

		await api.downloadUpdate();

		expect(mocks.emit.mock.calls).toEqual([
			['update://progress', { downloaded: 0, total: 1000 }],
			['update://progress', { downloaded: 400, total: 1000 }],
			['update://progress', { downloaded: 800, total: 1000 }],
			['update://progress', { downloaded: 800, total: 1000 }]
		]);
		expect(await api.getUpdateStatus()).toEqual({
			currentVersion: '0.8.1',
			stagedVersion: '0.9.0',
			stagedNotes: 'Fixed the roster grid.',
			stagedPubDate: '2026-09-01T10:00:00Z',
			attendanceWarning: null
		});
	});

	it('keeps the total unknown when the server sends no content length', async () => {
		mocks.check.mockResolvedValue(fakeUpdate({ events: [{ event: 'Started', data: {} }, CHUNK] }));
		const api = await loadApi();

		await api.downloadUpdate();

		expect(mocks.emit).toHaveBeenLastCalledWith('update://progress', {
			downloaded: 400,
			total: null
		});
	});

	it('refuses to download when the server has nothing newer', async () => {
		mocks.check.mockResolvedValue(null);
		const api = await loadApi();

		await expect(api.downloadUpdate()).rejects.toThrow('No update available');
	});

	it('names the check failure behind an unavailable update', async () => {
		mocks.check.mockRejectedValue(new Error('502 Bad Gateway'));
		const api = await loadApi();

		await expect(api.downloadUpdate()).rejects.toThrow('Update check failed: 502 Bad Gateway');
	});

	it('reports a failed download and stages nothing', async () => {
		const update = fakeUpdate();
		update.download.mockRejectedValue(new Error('connection reset'));
		mocks.check.mockResolvedValue(update);
		const api = await loadApi();

		await expect(api.downloadUpdate()).rejects.toThrow('Download failed: connection reset');
		expect(update.close).toHaveBeenCalled();
		expect((await api.getUpdateStatus()).stagedVersion).toBeNull();
	});
});

describe('cancelUpdateDownload', () => {
	it('reports the cancellation the panel maps back to the available state', async () => {
		const update = fakeUpdate({ events: [STARTED, CHUNK], duringDownload: undefined });
		mocks.check.mockResolvedValue(update);
		const api = await loadApi();

		// The plugin cannot abort the transfer, so the cancel is a flag read once
		// the download finishes.
		update.download.mockImplementation(async (onEvent?: (event: DownloadEvent) => void) => {
			onEvent?.(STARTED);
			await api.cancelUpdateDownload();
			onEvent?.(CHUNK);
		});

		await expect(api.downloadUpdate()).rejects.toThrow('Download cancelled');
		expect(update.install).not.toHaveBeenCalled();
		expect((await api.getUpdateStatus()).stagedVersion).toBeNull();
	});

	it('does not affect the next download', async () => {
		const api = await loadApi();
		await api.cancelUpdateDownload();
		const update = fakeUpdate({ version: '0.9.0' });
		mocks.check.mockResolvedValue(update);

		await api.downloadUpdate();

		expect((await api.getUpdateStatus()).stagedVersion).toBe('0.9.0');
	});
});

describe('installStagedUpdate', () => {
	it('snapshots before installing, then clears the staged update', async () => {
		const update = fakeUpdate({ version: '0.9.0' });
		mocks.check.mockResolvedValue(update);
		const api = await loadApi();
		await api.downloadUpdate();

		await api.installStagedUpdate();

		expect(mocks.createBackup).toHaveBeenCalledWith('pre_install');
		expect(update.install).toHaveBeenCalled();
		expect((await api.getUpdateStatus()).stagedVersion).toBeNull();
	});

	it('refuses to install when nothing was downloaded', async () => {
		const api = await loadApi();

		await expect(api.installStagedUpdate()).rejects.toThrow('No staged update found');
		expect(mocks.createBackup).not.toHaveBeenCalled();
	});

	it('refuses to install when the pre-install backup cannot be written', async () => {
		const update = fakeUpdate();
		mocks.check.mockResolvedValue(update);
		mocks.createBackup.mockRejectedValue(new Error('disk full'));
		const api = await loadApi();
		await api.downloadUpdate();

		await expect(api.installStagedUpdate()).rejects.toThrow('Pre-install backup failed: disk full');
		expect(update.install).not.toHaveBeenCalled();
	});

	it('reports a failed install and allows a retry', async () => {
		const update = fakeUpdate();
		update.install.mockRejectedValue(new Error('access denied'));
		mocks.check.mockResolvedValue(update);
		const api = await loadApi();
		await api.downloadUpdate();

		await expect(api.installStagedUpdate()).rejects.toThrow('Install failed: access denied');
		update.install.mockResolvedValue(undefined);

		await api.installStagedUpdate();
		expect(update.install).toHaveBeenCalledTimes(2);
	});

	it('allows one install at a time', async () => {
		const update = fakeUpdate();
		update.install.mockImplementation(() => new Promise<void>(() => {}));
		mocks.check.mockResolvedValue(update);
		const api = await loadApi();
		await api.downloadUpdate();

		const first = api.installStagedUpdate();
		await vi.waitFor(() => expect(update.install).toHaveBeenCalled());
		await expect(api.installStagedUpdate()).rejects.toThrow(
			'An update install is already in progress'
		);
		void first;
	});
});

describe('openExternalUrl', () => {
	it('opens a release-notes link in the system browser', async () => {
		mocks.openUrl.mockResolvedValue(undefined);
		const api = await loadApi();

		await api.openExternalUrl('https://github.com/Drakaniia/EES-AMS/releases/tag/app-v0.9.0');

		expect(mocks.openUrl).toHaveBeenCalledWith(
			'https://github.com/Drakaniia/EES-AMS/releases/tag/app-v0.9.0'
		);
	});

	it('reports a link the system refused', async () => {
		mocks.openUrl.mockRejectedValue(new Error('no handler'));
		const api = await loadApi();

		await expect(api.openExternalUrl('https://example.invalid')).rejects.toThrow(
			'Failed to open link: no handler'
		);
	});
});

describe('the lifecycle the Settings panel drives', () => {
	it('runs idle → available → downloading → staged → installed', async () => {
		const update = fakeUpdate({ version: '0.9.0', events: [STARTED, CHUNK, FINISHED] });
		mocks.check.mockResolvedValue(update);
		const api = await loadApi();

		// idle
		expect(await api.getUpdateStatus()).toMatchObject({ stagedVersion: null });
		// checking → available
		expect(await api.checkForUpdates()).toMatchObject({ available: true, version: '0.9.0' });
		// downloading → staged
		await api.downloadUpdate();
		expect(mocks.emit).toHaveBeenCalled();
		expect(await api.getUpdateStatus()).toMatchObject({ stagedVersion: '0.9.0' });
		// installed
		await api.installStagedUpdate();
		expect(await api.getUpdateStatus()).toMatchObject({
			stagedVersion: null,
			currentVersion: '0.8.1'
		});
	});
});
