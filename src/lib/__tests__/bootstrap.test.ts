import { beforeEach, describe, expect, it, vi } from 'vitest';

/**
 * The quit hook is the only code that can close the window, because it calls
 * `preventDefault()` on every close request. So "the backup failed" must never
 * mean "the app cannot be closed" — that regression is invisible in dev (the
 * window closes anyway once the backup resolves) and fatal in the release .exe,
 * where a wedged database worker leaves the quit backup pending forever.
 */
const mocks = vi.hoisted(() => ({
	handler: null as null | ((event: { preventDefault: () => void }) => Promise<void>),
	destroy: vi.fn(async () => {}),
	onAppQuit: vi.fn<() => Promise<unknown>>(async () => undefined),
	stopScheduledBackups: vi.fn(),
	state: { value: 'ready' as string }
}));

vi.mock('@tauri-apps/api/window', () => ({
	getCurrentWindow: () => ({
		onCloseRequested: async (handler: typeof mocks.handler) => {
			mocks.handler = handler;
			return () => {};
		},
		destroy: mocks.destroy
	})
}));
vi.mock('$lib/platform/tauri-fs', () => ({ TauriFileSystem: class {} }));
vi.mock('$lib/features/sf2/preview', () => ({ registerSf2Preview: vi.fn() }));
vi.mock('$lib/features/backup', () => ({
	onAppQuit: mocks.onAppQuit,
	scheduleBackups: vi.fn(),
	stopScheduledBackups: mocks.stopScheduledBackups
}));
vi.mock('$lib/stores/database-status.svelte', () => ({
	databaseStatus: {
		get state() {
			return mocks.state.value;
		},
		startAutoRetry: vi.fn()
	}
}));

/** Boot the app and return the close handler it registered. */
async function registerQuitHook(): Promise<
	(event: { preventDefault: () => void }) => Promise<void>
> {
	vi.resetModules();
	const { bootstrapApp } = await import('../bootstrap');
	await bootstrapApp();
	const handler = mocks.handler;
	if (!handler) throw new Error('no close-requested handler was registered');
	return handler;
}

describe('the quit hook', () => {
	beforeEach(() => {
		mocks.destroy.mockClear();
		mocks.stopScheduledBackups.mockClear();
		mocks.onAppQuit.mockReset().mockResolvedValue(undefined);
		mocks.state.value = 'ready';
		mocks.handler = null;
	});

	it('closes the window even when the quit backup never finishes', async () => {
		vi.useFakeTimers();
		// A wedged database worker: `send()` waits on a postMessage reply that
		// never comes, so the backup promise never settles.
		mocks.onAppQuit.mockImplementation(() => new Promise<void>(() => {}));
		const handler = await registerQuitHook();

		// Deliberately not awaited: the handler may never settle, and the point
		// is that `destroy()` runs anyway.
		void handler({ preventDefault: vi.fn() });
		await vi.advanceTimersByTimeAsync(10_000);

		expect(mocks.destroy).toHaveBeenCalled();
		vi.useRealTimers();
	});

	it('closes the window when the quit backup throws', async () => {
		mocks.onAppQuit.mockRejectedValue(new Error('OPFS is gone'));
		const handler = await registerQuitHook();

		await handler({ preventDefault: vi.fn() });

		expect(mocks.destroy).toHaveBeenCalled();
	});

	it('does not ask a database that failed to open for an image', async () => {
		mocks.state.value = 'unavailable';
		const handler = await registerQuitHook();

		await handler({ preventDefault: vi.fn() });

		expect(mocks.onAppQuit).not.toHaveBeenCalled();
		expect(mocks.destroy).toHaveBeenCalled();
	});
});
