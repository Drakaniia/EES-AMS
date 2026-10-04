import { describe, expect, it } from 'vitest';
import {
	classifyMissingOpfs,
	diagnoseOpfsFailure,
	parseOpfsCause,
	probeOpfsStorage,
	summarizeUnavailableError,
	type OpfsEnv,
	type ProbeStorage
} from './opfs-diagnosis';

const healthyEnv: OpfsEnv = {
	hasSharedArrayBuffer: true,
	hasAtomics: true,
	hasFileSystemHandle: true,
	hasDirectoryHandle: true,
	hasSyncAccessHandle: true,
	hasGetDirectory: true,
	crossOriginIsolated: true,
	protocol: 'https:'
};

/**
 * Storage whose `getDirectory` refuses an unbound call, exactly like the real
 * `navigator.storage`. A probe that detaches the method gets `Illegal
 * invocation`; a probe that calls it as a method passes.
 */
function strictStorage(): ProbeStorage & { calls: number } {
	const storage = {
		calls: 0,
		async getDirectory(this: unknown) {
			if (this !== storage)
				throw new Error(`Failed to execute 'getDirectory' on 'StorageManager': Illegal invocation`);
			storage.calls += 1;
			return {
				async getFileHandle() {
					return {
						async createSyncAccessHandle() {
							return {
								write() {
									return 4;
								},
								truncate() {},
								close() {}
							};
						}
					};
				},
				async removeEntry() {}
			};
		}
	};
	return storage;
}

describe('opfs probe binding', () => {
	it('passes against storage that rejects unbound calls', async () => {
		const storage = strictStorage();
		await expect(probeOpfsStorage(storage)).resolves.toBeNull();
		expect(storage.calls).toBe(1);
	});

	it('reports a missing getDirectory instead of throwing', async () => {
		await expect(probeOpfsStorage(undefined)).resolves.toBe(
			'navigator.storage.getDirectory is missing'
		);
	});

	it('passes the real storage error through', async () => {
		const storage = {
			async getDirectory() {
				throw new Error('QuotaExceededError');
			}
		};
		await expect(probeOpfsStorage(storage)).resolves.toBe('QuotaExceededError');
	});
});

describe('missing-OPFS taxonomy', () => {
	it('blames an old WebView2 when sync-access handles are missing', () => {
		const { cause, note } = classifyMissingOpfs({ ...healthyEnv, hasSyncAccessHandle: false });
		expect(cause).toBe('webview-old');
		expect(note).toContain('WebView2');
	});

	it('does not blame SharedArrayBuffer: the sahpool VFS does not need it', () => {
		const { cause } = classifyMissingOpfs({
			...healthyEnv,
			hasSharedArrayBuffer: false,
			hasAtomics: false,
			crossOriginIsolated: false
		});
		expect(cause).toBe('storage-blocked');
	});

	it('blames blocked storage only when every OPFS API is present', () => {
		const { cause } = classifyMissingOpfs(healthyEnv);
		expect(cause).toBe('storage-blocked');
	});
});

describe('failure diagnosis', () => {
	it('keeps the startup reason and never claims blocked storage when the probe passed', async () => {
		const detail = await diagnoseOpfsFailure(
			'could not start the on-device storage engine (boom)',
			'storage-blocked',
			healthyEnv,
			strictStorage()
		);
		expect(detail).toContain('cause=storage-proxy');
		expect(detail).toContain('boom');
		expect(detail).not.toContain('cause=storage-blocked');
	});

	it('keeps the blocked-storage cause when the probe really failed', async () => {
		const failing = {
			async getDirectory() {
				throw new Error('denied');
			}
		};
		const detail = await diagnoseOpfsFailure('note', 'storage-blocked', healthyEnv, failing);
		expect(detail).toContain('cause=storage-blocked');
		expect(detail).toContain('OPFS storage probe failed: denied');
	});
});

describe('teacher-facing summary', () => {
	it('maps every cause to steps and keeps the technical string', () => {
		for (const cause of ['webview-old', 'storage-proxy', 'storage-blocked']) {
			const summary = summarizeUnavailableError(
				`detail [cause=${cause}; protocol=https: isolated=true sab=yes]`
			);
			expect(summary.cause).toBe(cause);
			expect(summary.steps.length).toBeGreaterThan(0);
			expect(summary.technical).toContain(`cause=${cause}`);
		}
	});

	it('falls back gracefully without a cause token', () => {
		expect(parseOpfsCause('database error: something old')).toBe('unknown');
		expect(summarizeUnavailableError('database error: something old').steps.length).toBeGreaterThan(
			0
		);
	});
});
