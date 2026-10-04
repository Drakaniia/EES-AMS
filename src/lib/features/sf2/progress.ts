/**
 * Progress reporting for the long SF2 operations.
 *
 * ## Why a callback and not an event
 *
 * Rust emitted a Tauri event because the work happened on a Tauri thread pool
 * thread with no caller to return to. In TypeScript the work is a `Promise` the
 * caller already awaits, so the observer is a plain callback passed in. The
 * *shape* is unchanged — `task`, `current`, `total`, `message` — so
 * `report-sf2-open.svelte.ts` keeps the same ten messages and the same
 * `progressMessages` table, and a caller that still wants a Tauri event can emit
 * one from a reporter in three lines.
 *
 * ## The two scales
 *
 * The outer flow is **10 steps** (`SF2_OPEN_STEPS`), because that is the table the
 * UI maps to its friendly strings. The write phase inside step 6 is reported on a
 * **100-point** scale between 61 and 69, so the bar crawls through the part that
 * actually takes time instead of sitting at 60% while a thousand cells are
 * written. Both are the Rust numbers; only the transport changed.
 */

/** The only task the app reports on. Kept so a UI filter cannot drift. */
type Sf2ProgressTask = 'open';

export type Sf2ProgressUpdate = {
	task: Sf2ProgressTask;
	current: number;
	total: number;
	message: string;
};

export type Sf2ProgressReporter = (update: Sf2ProgressUpdate) => void;

/** For callers with no UI to show progress to (roster sync, background sync). */
export const NO_PROGRESS: Sf2ProgressReporter = () => {};

/**
 * The ten outer steps of "Open SF2", word for word what the Rust emitted.
 *
 * `report-sf2-open.svelte.ts` maps a step number to the same strings, so the two
 * cannot disagree about what step 4 is.
 */
export const SF2_OPEN_STEPS: readonly string[] = [
	'Loading workbook details…',
	'Reading student data…',
	'Checking date mappings…',
	'Clearing previous marks…',
	'Computing attendance marks…',
	'Writing marks to workbook…',
	'Saving workbook changes…',
	'Preparing to open…',
	'Opening in Microsoft Excel…',
	'Done!'
];

/** How many outer steps the Open SF2 modal's bar is divided into. */
export const SF2_OPEN_TOTAL = SF2_OPEN_STEPS.length;

/** The first percentage of the 100-point scale the fine-grained write phase uses. */
const WRITE_PHASE_START = 61;

/** How many percentage points the write phase spans (61 → 69). */
const WRITE_PHASE_SPAN = 8;

/**
 * Report one outer step, 1-based.
 *
 * A step outside the table is still reported with a blank message rather than
 * dropped, because a caller that reached step 11 has a bug the UI should not hide.
 */
export function emitSf2Progress(
	reporter: Sf2ProgressReporter,
	step: number,
	message?: string
): void {
	reporter({
		task: 'open',
		current: step,
		total: SF2_OPEN_TOTAL,
		message: message ?? SF2_OPEN_STEPS[step - 1] ?? ''
	});
}

/**
 * Report progress inside the write phase, on the 61–69 slice of the 100-point
 * scale.
 *
 * The bar is moved once per chunk rather than once per phase, so a teacher
 * watching it sees the write happen. `totalUnits` must equal the sum of every
 * phase's chunk count below it, or the bar will not land on 69.
 */
export function emitSf2WriteStep(
	reporter: Sf2ProgressReporter,
	unitsDone: number,
	totalUnits: number,
	message: string
): void {
	const total = Math.max(totalUnits, 1);
	const offset = Math.min(Math.ceil((unitsDone * WRITE_PHASE_SPAN) / total), WRITE_PHASE_SPAN);
	reporter({ task: 'open', current: WRITE_PHASE_START + offset, total: 100, message });
}
