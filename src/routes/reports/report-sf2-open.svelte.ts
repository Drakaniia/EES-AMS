import { SvelteMap } from 'svelte/reactivity';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { errorMessage } from './report-state.svelte';
import { syncAndOpenSf2Workbook } from '$lib/db-rust';
import type { Sf2ExportPreview } from '$lib/db-rust';

// ── Types ──────────────────────────────────────────────────────────────────────

export type Sf2OpenStatus = 'idle' | 'syncing' | 'success' | 'error';

// ── Friendly loading messages that cycle during SF2 open ───────────────────────

export const SF2_OPEN_MESSAGES = [
	'Warming up the workbook…',
	'Reading attendance records…',
	'Writing marks to the workbook…',
	'Almost there, wrapping things up…',
	'Opening in Excel…',
	'Double-checking everything is in order…',
	'Just a moment longer!'
] as const;

// ── Preview cache: eliminates redundant backend calls on month switch ──────────
// When switching to a month that's already been loaded, returns instantly.
// Key format: `${classId}:${reportMonth}`

const previewCache = new SvelteMap<string, Sf2ExportPreview>();

export function cacheKey(classId: string, reportMonth: string): string {
	return `${classId}:${reportMonth}`;
}

export function invalidateCacheForMonth(classId: string, reportMonth: string) {
	previewCache.delete(cacheKey(classId, reportMonth));
}

export function invalidateAllCache() {
	previewCache.clear();
}

export function getPreviewCache(): SvelteMap<string, Sf2ExportPreview> {
	return previewCache;
}

// ── SF2 Open state machine ─────────────────────────────────────────────────────
// Encapsulates all state and lifecycle for the "Open SF2" process including
// progress tracking, cycling friendly messages, and Tauri event listeners.

export type ShowToastFn = (message: string, ok?: boolean) => void;

export function createSf2OpenState() {
	let status = $state<Sf2OpenStatus>('idle');
	let progressCurrent = $state(0);
	let progressTotal = $state(10);
	let error = $state<string | null>(null);
	let resultPath = $state<string | null>(null);
	let cycleIndex = $state(0);
	let lastBackendMsg = $state('');
	let lastBackendTime = $state(0);
	let displayMessage = $state('');

	let cycleTimer: ReturnType<typeof setInterval> | null = null;
	let successTimer: ReturnType<typeof setTimeout> | null = null;
	let unlisten: UnlistenFn | null = null;

	const progressPercent = $derived.by(() => {
		if (progressTotal <= 0) return 0;
		return Math.round((progressCurrent / progressTotal) * 100);
	});

	function updateDisplayMessage() {
		// Backend message has priority for 4 seconds
		if (lastBackendMsg && Date.now() - lastBackendTime < 4000) {
			displayMessage = lastBackendMsg;
			return;
		}
		// Map progress steps to messages when no backend message
		if (progressCurrent > 0 && progressTotal > 0) {
			const progressMessages: Record<number, string> = {
				1: 'Warming up the workbook…',
				2: 'Reading attendance records…',
				3: 'Checking date mappings…',
				4: 'Clearing previous marks…',
				5: 'Computing attendance marks…',
				6: 'Writing marks to the workbook…',
				7: 'Saving workbook changes…',
				8: 'Preparing to open…',
				9: 'Opening in Excel…',
				10: 'Done!'
			};
			displayMessage =
				progressMessages[progressCurrent] || SF2_OPEN_MESSAGES[cycleIndex];
			return;
		}
		displayMessage = SF2_OPEN_MESSAGES[cycleIndex];
	}

	function startMessageCycle() {
		stopMessageCycle();
		updateDisplayMessage();
		cycleTimer = setInterval(() => {
			const now = Date.now();
			// Only cycle if no backend message arrived in the last 3 seconds
			if (now - lastBackendTime > 3000) {
				cycleIndex = (cycleIndex + 1) % SF2_OPEN_MESSAGES.length;
			}
			updateDisplayMessage();
		}, 2500);
	}

	function stopMessageCycle() {
		if (cycleTimer !== null) {
			clearInterval(cycleTimer);
			cycleTimer = null;
		}
	}

	function cleanup() {
		stopMessageCycle();
		if (successTimer !== null) {
			clearTimeout(successTimer);
			successTimer = null;
		}
		if (unlisten) {
			unlisten();
			unlisten = null;
		}
	}

	async function setupListener() {
		cleanup();
		try {
			unlisten = await listen<{
				task: string;
				current: number;
				total: number;
				message: string;
			}>('sf2-progress', (event) => {
				if (event.payload.task === 'open') {
					progressCurrent = event.payload.current;
					progressTotal = event.payload.total;
					if (event.payload.message) {
						lastBackendMsg = event.payload.message;
						lastBackendTime = Date.now();
					}
					updateDisplayMessage();
				}
			});
		} catch {
			// Listener setup failed; continue without it (indeterminate fallback)
		}
	}

	// The open function is hoisted so retry can reference it regardless of
	// declaration order in the return object.
	let open: (
		activeClassId: string,
		preview: Sf2ExportPreview | null,
		showToast: ShowToastFn
	) => Promise<void>;

	open = async (activeClassId, preview, showToast) => {
		if (!activeClassId || !preview?.template || status === 'syncing') return;

		// Reset progress state
		status = 'syncing';
		progressCurrent = 0;
		progressTotal = 10;
		error = null;
		resultPath = null;
		cycleIndex = 0;
		lastBackendMsg = '';
		lastBackendTime = 0;

		// Show first step immediately for responsiveness
		progressCurrent = 1;
		cycleIndex = 0;

		// Set up progress event listener
		await setupListener();
		startMessageCycle();

		try {
			const path = await syncAndOpenSf2Workbook(activeClassId);
			resultPath = path;
			status = 'success';
			stopMessageCycle();
			showToast(`Opened SF2 working copy: ${path}`);

			// Auto-close after 1.5 seconds
			successTimer = setTimeout(() => {
				status = 'idle';
			}, 1500);
		} catch (err) {
			stopMessageCycle();
			const msg = errorMessage(err, 'Failed to update SF2 workbook');
			if (msg.toLowerCase().includes('excel')) {
				error =
					'The SF2 working copy is currently open in Microsoft Excel. ' +
					'Close the workbook in Excel first, then click Open SF2 again.';
			} else {
				error = `Could not sync attendance to the SF2 workbook: ${msg}`;
			}
			status = 'error';
		}
	};

	function close() {
		status = 'idle';
		error = null;
	}

	async function retry(
		activeClassId: string,
		preview: Sf2ExportPreview | null,
		showToast: ShowToastFn
	) {
		status = 'idle';
		error = null;
		// Small delay so the UI resets cleanly before re-triggering
		await new Promise((resolve) => setTimeout(resolve, 50));
		await open(activeClassId, preview, showToast);
	}

	return {
		// Reactive state (read-only accessors)
		get status() {
			return status;
		},
		get progressCurrent() {
			return progressCurrent;
		},
		get progressTotal() {
			return progressTotal;
		},
		get error() {
			return error;
		},
		get resultPath() {
			return resultPath;
		},
		get displayMessage() {
			return displayMessage;
		},
		get progressPercent() {
			return progressPercent;
		},

		// Actions
		open,
		retry,
		close,
		cleanup
	};
}
