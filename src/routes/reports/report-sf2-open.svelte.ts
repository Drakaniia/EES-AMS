import { SvelteMap } from 'svelte/reactivity';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { errorMessage } from './report-state.svelte';
import { syncAndOpenSf2Workbook } from '$lib/api';
import type { Sf2ExportPreview, Sf2MonthGridPreview } from '$lib/types';

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

// Reassurance messages cycled while a single progress step stalls (typically
// the slow Excel COM write). Only these are shown so the text never describes
// an earlier phase than the actual progress bar.
export const SF2_STALL_MESSAGES = [
	'Still working on the workbook…',
	'Excel is finishing up in the background…',
	'Double-checking everything is in order…',
	'Just a moment longer!'
] as const;

// ── Month preview cache ─────────────────────────────────────────────────────
// A month read is already cheap, but a teacher flipping between a month and the
// one before it should not pay for the same query twice, and the grid is the
// hot thing on this page. Keyed on all three of the things that identify a
// month file - class, school year, month - because two classes can each have an
// OCTOBER, and one class has an OCTOBER in every school year it has ever been
// used.

const monthCache = new SvelteMap<string, Sf2MonthGridPreview>();

export function monthCacheKey(classId: string, schoolYear: string, reportMonth: string): string {
	return `${classId}:${schoolYear}:${reportMonth}`;
}

export function invalidateMonthCache(classId: string, schoolYear: string, reportMonth: string) {
	monthCache.delete(monthCacheKey(classId, schoolYear, reportMonth));
}

/**
 * Drop every month of one class.
 *
 * Used after something that changes more than one month at a time - a roster
 * sync, an export that rewrote the workbook. Naming one month would leave the
 * other eleven holding pre-change rows, and a stale X mark on screen is worse
 * than a slow one.
 */
export function invalidateClassMonths(classId: string) {
	const prefix = `${classId}:`;
	for (const key of [...monthCache.keys()]) {
		if (key.startsWith(prefix)) monthCache.delete(key);
	}
}

export function invalidateAllMonthCache() {
	monthCache.clear();
}

export function getMonthCache(): SvelteMap<string, Sf2MonthGridPreview> {
	return monthCache;
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
	// Becomes true when no progress event arrives for a while (e.g. the slow
	// Excel COM write phase) so the modal can surface a "please wait" hint and
	// keep cycling friendly messages instead of appearing frozen.
	let showWaitHint = $state(false);

	let cycleTimer: ReturnType<typeof setInterval> | null = null;
	let successTimer: ReturnType<typeof setTimeout> | null = null;
	let unlisten: UnlistenFn | null = null;
	let lastCycleAt = 0;

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
		// While stalled on a long step, cycle the reassurance messages so the UI
		// keeps feeling alive (e.g. "Just a moment longer!").
		if (showWaitHint) {
			displayMessage = SF2_STALL_MESSAGES[cycleIndex % SF2_STALL_MESSAGES.length];
			return;
		}
		// Map progress steps to messages when no backend message. The step is
		// derived from the percentage so this stays correct whether the backend
		// reports on a 10-step scale or the fine-grained 100-point write phase.
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
			const step = Math.min(10, Math.max(1, Math.floor((progressCurrent / progressTotal) * 10)));
			displayMessage = progressMessages[step] || SF2_OPEN_MESSAGES[cycleIndex];
			return;
		}
		displayMessage = SF2_OPEN_MESSAGES[cycleIndex];
	}

	function startMessageCycle() {
		stopMessageCycle();
		updateDisplayMessage();
		cycleTimer = setInterval(() => {
			const now = Date.now();
			// If no progress event arrived in the last 5 seconds, the backend is
			// busy in a long operation (usually the Excel write) — flag it so the
			// UI shows a "please wait" hint and cycles reassurance messages.
			// Checked on every tick (1s) so the hint appears close to the 5s mark.
			showWaitHint = now - lastBackendTime > 5000;
			// Rotate the friendly message roughly every 2.5s, only when no
			// backend message arrived in the last 3 seconds.
			if (now - lastCycleAt >= 2500 && now - lastBackendTime > 3000) {
				cycleIndex = (cycleIndex + 1) % SF2_OPEN_MESSAGES.length;
				lastCycleAt = now;
			}
			updateDisplayMessage();
		}, 1000);
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
						showWaitHint = false;
					}
					updateDisplayMessage();
				}
			});
		} catch {
			// Listener setup failed; continue without it (indeterminate fallback)
		}
	}

	// open is initialized before retry so retry can call it regardless of
	// declaration order in the return object.
	const open: (
		activeClassId: string,
		reportMonth: string,
		preview: Sf2ExportPreview | null,
		showToast: ShowToastFn
	) => Promise<void> = async (activeClassId, reportMonth, preview, showToast) => {
		if (!activeClassId || !reportMonth || !preview?.template || status === 'syncing') return;

		// Reset progress state
		status = 'syncing';
		progressCurrent = 0;
		progressTotal = 10;
		error = null;
		resultPath = null;
		cycleIndex = 0;
		lastCycleAt = 0;
		lastBackendMsg = '';
		lastBackendTime = 0;
		showWaitHint = false;

		// Show first step immediately for responsiveness
		progressCurrent = 1;
		cycleIndex = 0;

		// Set up progress event listener
		await setupListener();
		startMessageCycle();

		try {
			const path = await syncAndOpenSf2Workbook(activeClassId, reportMonth);
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
			// The app never drives Excel, so there is no process of ours to clear.
			// If the teacher has the workbook open the write failed and the file is
			// untouched; the message says what to do about it (spec D14, D15).
			error = `Could not sync attendance to the SF2 workbook: ${errorMessage(
				err,
				'Failed to update SF2 workbook'
			)}`;
			status = 'error';
		}
	};

	function close() {
		status = 'idle';
		error = null;
	}

	async function retry(
		activeClassId: string,
		reportMonth: string,
		preview: Sf2ExportPreview | null,
		showToast: ShowToastFn
	) {
		status = 'idle';
		error = null;
		// Small delay so the UI resets cleanly before re-triggering
		await new Promise((resolve) => setTimeout(resolve, 50));
		await open(activeClassId, reportMonth, preview, showToast);
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
		get showWaitHint() {
			return showWaitHint;
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
