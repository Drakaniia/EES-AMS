/**
 * OPFS failure diagnosis — the testable half of `worker.ts`'s open path.
 *
 * The worker owns the WASM module and the sync access handle; this module owns
 * every *decision* about a failure (which cause, which words, which next step)
 * so Vitest can cover the taxonomy without a browser. The worker calls in with
 * the live globals, tests call in with fakes.
 */

/** The buckets a failed open is sorted into. Never claim `storage-blocked` when the probe passed. */
export type OpfsCause =
	| 'headers'
	| 'webview-old'
	| 'not-isolated'
	| 'storage-proxy'
	| 'storage-blocked'
	| 'unknown';

/** The feature flags `describeMissingOpfs` used to read off `globalThis` inline. */
export type OpfsEnv = {
	hasSharedArrayBuffer: boolean;
	hasAtomics: boolean;
	hasFileSystemHandle: boolean;
	hasDirectoryHandle: boolean;
	hasSyncAccessHandle: boolean;
	hasGetDirectory: boolean;
	/** `'unknown'` when the global is absent (older runtimes). */
	crossOriginIsolated: boolean | 'unknown';
	protocol: string;
};

export function readOpfsEnv(
	g: Record<string, unknown> = globalThis as Record<string, unknown>
): OpfsEnv {
	const nav = g['navigator'] as { storage?: { getDirectory?: unknown } } | undefined;
	const fileHandle = g['FileSystemFileHandle'] as
		| { prototype?: { createSyncAccessHandle?: unknown } }
		| undefined;
	const loc = g['location'] as { protocol?: unknown } | undefined;
	return {
		hasSharedArrayBuffer: typeof SharedArrayBuffer !== 'undefined',
		hasAtomics: typeof (g['Atomics'] as object) !== 'undefined',
		hasFileSystemHandle: typeof g['FileSystemHandle'] !== 'undefined',
		hasDirectoryHandle: typeof g['FileSystemDirectoryHandle'] !== 'undefined',
		hasSyncAccessHandle: typeof fileHandle?.prototype?.createSyncAccessHandle !== 'undefined',
		hasGetDirectory: typeof nav?.storage?.getDirectory !== 'undefined',
		crossOriginIsolated:
			typeof crossOriginIsolated !== 'undefined' ? crossOriginIsolated : 'unknown',
		protocol: typeof loc?.protocol === 'string' ? loc.protocol : 'unknown'
	};
}

/**
 * The static reason for a missing `OpfsDb`, in teacher-actionable terms.
 *
 * Mirrors sqlite-wasm's `vfsInstallationFeatureCheck` (SAB+Atomics, worker
 * context, FileSystem sync-access APIs), whose own failure is swallowed to a
 * `warn` by the module bootstrap — leaving `oo1.OpfsDb` undefined with no
 * reason attached. SAB present + OpfsDb missing means headers are fine and
 * the PC's WebView2 is too old for OPFS sync access handles.
 */
export function classifyMissingOpfs(env: OpfsEnv): { cause: OpfsCause; note: string } {
	if (!env.hasSharedArrayBuffer || !env.hasAtomics) {
		return {
			cause: 'headers',
			note: 'the database could not be opened (SharedArrayBuffer is missing); the app must be served with COOP/COEP headers'
		};
	}
	if (
		!env.hasFileSystemHandle ||
		!env.hasDirectoryHandle ||
		!env.hasSyncAccessHandle ||
		!env.hasGetDirectory
	) {
		return {
			cause: 'webview-old',
			note: 'the database could not be opened (this PC’s WebView2 runtime lacks OPFS sync-access handles); update “Microsoft Edge WebView2 Runtime” to the latest version, then reopen the app'
		};
	}
	if (env.crossOriginIsolated === false) {
		return {
			cause: 'not-isolated',
			note: 'the database could not be opened (the window is not cross-origin isolated); the app must be served with COOP/COEP headers'
		};
	}
	return {
		cause: 'storage-blocked',
		note: 'the database could not be opened (on-device storage is blocked or full on this PC); check disk space and site-storage permissions, then reopen the app'
	};
}

/** Compact runtime facts, appended to every OPFS failure instead of guessing. */
export function envFacts(env: OpfsEnv): string {
	return [
		`protocol=${env.protocol}`,
		`isolated=${env.crossOriginIsolated === 'unknown' ? 'unknown' : String(env.crossOriginIsolated)}`,
		`sab=${env.hasSharedArrayBuffer ? 'yes' : 'no'}`
	].join(' ');
}

export type ProbeFileHandle = {
	createSyncAccessHandle(): Promise<{
		write(buffer: Uint8Array, options?: { at?: number }): number;
		truncate(size: number): void;
		close(): void;
	}>;
};

export type ProbeRoot = {
	getFileHandle(name: string, options?: { create?: boolean }): Promise<ProbeFileHandle>;
	removeEntry(name: string): Promise<void>;
};

/** The slice of `navigator.storage` the probe needs. Passed as the receiver, never detached. */
export type ProbeStorage = {
	getDirectory(): Promise<ProbeRoot>;
};

/**
 * Live OPFS probe, run only on the failure path.
 *
 * `storage` is the live `navigator.storage` object and `getDirectory` is
 * called on it as a method. Detaching it first (`const f = storage.getDirectory;
 * f()`) throws `Illegal invocation` even on a healthy PC — the exact
 * misleading probe result this fix removes.
 */
export async function probeOpfsStorage(storage: ProbeStorage | undefined): Promise<string | null> {
	try {
		if (!storage || typeof storage.getDirectory !== 'function') {
			return 'navigator.storage.getDirectory is missing';
		}
		const root = await storage.getDirectory();
		const file = await root.getFileHandle('.ees-ams-opfs-probe', { create: true });
		const access = await file.createSyncAccessHandle();
		try {
			access.write(new Uint8Array([1, 2, 3, 4]), { at: 0 });
			access.truncate(4);
		} finally {
			access.close();
		}
		await root.removeEntry('.ees-ams-opfs-probe');
		return null;
	} catch (error) {
		return error instanceof Error ? error.message : String(error);
	}
}

/**
 * The static reason plus the live probe, so the message is evidence, not a guess.
 *
 * A passed probe overrides a `storage-blocked` classification: storage works,
 * so the failure is the SQLite proxy worker/headers — never the PC.
 */
export async function diagnoseOpfsFailure(
	note: string,
	cause: OpfsCause,
	env: OpfsEnv,
	storage: ProbeStorage | undefined
): Promise<string> {
	const probe = await probeOpfsStorage(storage);
	if (probe === null && cause === 'storage-blocked') {
		return `the database could not be opened even though on-device storage works (probe passed); the app’s storage setup failed to start — reopen the app, and report the details below if it persists [cause=storage-proxy; ${envFacts(env)}; OPFS storage probe passed]`;
	}
	const storageNote =
		probe === null
			? 'OPFS storage probe passed, so the failure is the SQLite proxy worker/headers rather than blocked storage'
			: `OPFS storage probe failed: ${probe}`;
	return `${note} [cause=${cause}; ${envFacts(env)}; ${storageNote}]`;
}

export type UnavailableSummary = {
	cause: OpfsCause;
	/** One plain sentence: what happened and that on-disk files are untouched. */
	plain: string;
	/** In-app recovery steps, not a bare link. */
	steps: string[];
	technical: string;
};

/** Pull the `cause=` token the worker stamped into the detail. */
export function parseOpfsCause(detail: string): OpfsCause {
	const match = /\[cause=([a-z-]+);/.exec(detail);
	if (match?.[1] === 'webview-old') return 'webview-old';
	if (match?.[1] === 'storage-proxy') return 'storage-proxy';
	if (match?.[1] === 'storage-blocked') return 'storage-blocked';
	if (match?.[1] === 'headers') return 'headers';
	if (match?.[1] === 'not-isolated') return 'not-isolated';
	return 'unknown';
}

const SHARED_PLAIN =
	'The app couldn’t open its on-device database, so this data isn’t available. Your workbooks and backups saved under Documents\\EES-AMS are untouched.';

/**
 * Friendly layer over the worker's technical detail: plain sentence plus
 * cause-specific in-app steps. The technical string stays available under
 * `technical` for the expandable Details block.
 */
export function summarizeUnavailableError(detail: string): UnavailableSummary {
	const cause = parseOpfsCause(detail);
	switch (cause) {
		case 'webview-old':
			return {
				cause,
				plain: `${SHARED_PLAIN} This PC’s viewer component (WebView2) is too old for the app’s storage.`,
				steps: [
					'Install the latest “Microsoft Edge WebView2 Runtime” (Evergreen) on this PC.',
					'Close the app completely, then reopen it.',
					'If reports are still unavailable, use Retry below — your files on disk are unaffected.'
				],
				technical: detail
			};
		case 'storage-proxy':
			return {
				cause,
				plain: `${SHARED_PLAIN} On-device storage itself works — the app’s storage setup failed to start.`,
				steps: [
					'Reopen the app and use Retry below.',
					'If it persists, copy the Details below into a bug report — this looks like an app setup issue, not your PC.'
				],
				technical: detail
			};
		case 'storage-blocked':
			return {
				cause,
				plain: `${SHARED_PLAIN} On-device storage is blocked or the disk is full on this PC.`,
				steps: [
					'Free disk space and make sure the drive isn’t full.',
					'Allow site data / storage for the app (private or data-cleared modes block it), then reopen the app.',
					'Use Retry below once space or permissions are fixed.'
				],
				technical: detail
			};
		case 'headers':
		case 'not-isolated':
			return {
				cause,
				plain: `${SHARED_PLAIN} The app wasn’t served with the storage headers it needs.`,
				steps: [
					'Reopen the app and use Retry below.',
					'If it persists, copy the Details below into a bug report.'
				],
				technical: detail
			};
		default:
			return {
				cause,
				plain: `${SHARED_PLAIN}`,
				steps: [
					'Reopen the app and use Retry below.',
					'If it persists, copy the Details below into a bug report.'
				],
				technical: detail
			};
	}
}
