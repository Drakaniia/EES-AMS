import { describeError, getDriver, summarizeUnavailableError, WorkerSqlDriver } from '$lib/db';
import { scheduleBackups, stopScheduledBackups } from '$lib/features/backup/scheduling';

/**
 * Global database availability: the one place the app knows whether OPFS is
 * down or the teacher is working in temporary (in-memory) mode.
 *
 * Pages keep their own `loadError` panels; this store drives the app-shell
 * gate and banner, so a failed first open is recoverable from any route. The
 * driver itself stays the `WorkerSqlDriver` singleton — temporary mode is a
 * server-side (worker-side) switch, which is why repos need no changes.
 */

export type DatabaseState = 'unknown' | 'ready' | 'unavailable' | 'temporary';

/** Silent re-probe at most this often while the gate is up. */
const AUTO_RETRY_DEBOUNCE_MS = 10_000;

function asWorkerDriver(): WorkerSqlDriver | null {
	const driver = getDriver();
	return driver instanceof WorkerSqlDriver ? driver : null;
}

class DatabaseStatusStore {
	state = $state<DatabaseState>('unknown');
	/** The raw worker detail (with the `[cause=…]` token); `summary` is the friendly layer. */
	detail = $state<string | null>(null);
	retrying = $state(false);
	private probing = false;
	private lastAutoRetry = 0;
	private focusStop: (() => void) | null = null;

	get summary() {
		return this.detail ? summarizeUnavailableError(this.detail) : null;
	}

	get isDown(): boolean {
		return this.state === 'unavailable' || this.state === 'temporary';
	}

	reportFailure(thrown: unknown, fallback: string): void {
		// Temporary mode works: a failure there is a page-level error, not the gate.
		if (this.state === 'temporary') return;
		this.detail = describeError(thrown, fallback);
		this.state = 'unavailable';
	}

	reportReady(): void {
		this.state = 'ready';
		this.detail = null;
	}

	/** First-open probe, run once from the app shell. Page reads dedupe behind it. */
	async probe(): Promise<void> {
		// No Worker in tests/SSR: probing would only manufacture a failure.
		if (typeof Worker === 'undefined') return;
		if (this.state === 'ready' || this.state === 'temporary' || this.probing) return;
		const driver = asWorkerDriver();
		if (!driver) return;
		this.probing = true;
		try {
			await driver.open();
			this.reportReady();
		} catch (thrown) {
			this.reportFailure(thrown, 'The database could not be opened.');
		} finally {
			this.probing = false;
		}
	}

	/** Manual Retry: re-open OPFS; on success leave temporary mode behind. */
	async retry(): Promise<boolean> {
		const driver = asWorkerDriver();
		if (!driver || this.retrying) return false;
		this.retrying = true;
		try {
			if (this.state === 'temporary') {
				if (await this.hasTemporaryData()) return false;
				await driver.recover();
			} else {
				await driver.open();
			}
			this.reportReady();
			scheduleBackups();
			return true;
		} catch (thrown) {
			this.reportFailure(thrown, 'The database could not be opened.');
			return false;
		} finally {
			this.retrying = false;
		}
	}

	/**
	 * Explicit, teacher-chosen entry into throwaway mode. Never silent: the
	 * banner on every page says changes will not persist.
	 */
	async enterTemporary(): Promise<boolean> {
		const driver = asWorkerDriver();
		if (!driver || this.retrying) return false;
		this.retrying = true;
		try {
			await driver.openTemporary();
			this.state = 'temporary';
			this.detail = null;
			// A scheduled zip of throwaway state would masquerade as a real
			// backup in retention; manual exports keep working.
			stopScheduledBackups();
			return true;
		} catch (thrown) {
			this.reportFailure(thrown, 'Temporary mode could not be started.');
			return false;
		} finally {
			this.retrying = false;
		}
	}

	/** True when the temporary database holds anything worth exporting first. */
	async hasTemporaryData(): Promise<boolean> {
		if (this.state !== 'temporary') return false;
		try {
			const driver = getDriver();
			const [students, events] = await Promise.all([
				driver
					.queryOne<{ total: number }>('SELECT COUNT(*) AS total FROM students')
					.catch(() => undefined),
				driver
					.queryOne<{ total: number }>('SELECT COUNT(*) AS total FROM events')
					.catch(() => undefined)
			]);
			return Number(students?.total ?? 0) + Number(events?.total ?? 0) > 0;
		} catch {
			return false;
		}
	}

	/**
	 * Silent re-probe on window focus while the gate is up — not in temporary
	 * mode, where a surprise recovery would mask the banner. Debounced; safe to
	 * call repeatedly (tests, non-Tauri previews: no `window`, no listener).
	 */
	startAutoRetry(): void {
		if (this.focusStop || typeof window === 'undefined') return;
		const onFocus = () => {
			if (this.state !== 'unavailable' || this.retrying) return;
			const now = Date.now();
			if (now - this.lastAutoRetry < AUTO_RETRY_DEBOUNCE_MS) return;
			this.lastAutoRetry = now;
			void this.retry().catch(() => {});
		};
		window.addEventListener('focus', onFocus);
		this.focusStop = () => window.removeEventListener('focus', onFocus);
	}

	stopAutoRetry(): void {
		this.focusStop?.();
		this.focusStop = null;
	}
}

export const databaseStatus = new DatabaseStatusStore();
