import { describeError } from '$lib/db';
import {
	getSf2LaunchMonth,
	listSf2MonthWorkbooks,
	runSf2WorkbookSplit
} from '$lib/features/settings/native';
import {
	sf2MonthWorkbookRows,
	sf2MonthWorkbookSummary,
	sf2SplitIsComplete,
	sf2SplitNeedsAttention,
	sf2SplitSummary,
	type Sf2MonthWorkbookRow
} from '$lib/features/settings/sf2-months';
import type { Sf2LaunchMonth, Sf2SplitOutcome } from '$lib/types';
import type { Ctx } from './state-context';

/**
 * Settings → SF2 Workbook (spec §12.1, D13, D16, D18).
 *
 * ## What this screen is now
 *
 * Three things, replacing the three workflows D18 removed:
 *
 * 1. **Month workbooks** - twelve read-only rows: is the file there, how many X
 *    were last counted in it, when was it last written. Every month is dated
 *    from June, the month classes start, so no start-date setup is needed.
 * 2. **Back up workbooks now** - D13. Lives in Data Management, beside *Back Up
 *    Now*, and already wired; this screen links to it rather than duplicating it.
 * 3. **Re-run the workbook split** - §11, for a month that failed to split.
 *
 * ## What is gone, and why that is safe
 *
 * *Create From Template*, *Import SF2* and their validation flow are gone (D18).
 * Both existed to get a workbook into the app from outside it; the per-month
 * model replaced that with a file per month the app writes itself, and the marks
 * the import used to recover now come back on their own at startup (§8.2). The
 * dialogs and the state they needed are deleted, not left behind: a control with
 * its label removed is worse than no control.
 */
class Sf2State {
	ctx!: Ctx;

	init(ctx: Ctx) {
		this.ctx = ctx;
	}

	// ── State ─────────────────────────────────────────────────────────────────
	/** The month the app would open on, and whether it fell back (D5, E1). */
	launch = $state<Sf2LaunchMonth | null>(null);
	/** The twelve rows, as the backend stores them. Sorted for display below. */
	months = $state<Awaited<ReturnType<typeof listSf2MonthWorkbooks>>>([]);
	monthsLoading = $state(false);
	/** Why the list could not be read, if it could not. Never swallowed. */
	monthsError = $state<string | null>(null);

	splitRunning = $state(false);
	splitOutcome = $state<Sf2SplitOutcome | null>(null);

	// ── Derived ───────────────────────────────────────────────────────────────
	/** Twelve rows, sorted SEPTEMBER → AUGUST, ready to render. */
	monthRows = $derived.by<Sf2MonthWorkbookRow[]>(() => sf2MonthWorkbookRows(this.months));

	/** The one-line state of the year, under the list. */
	monthSummary = $derived(sf2MonthWorkbookSummary(this.monthRows));

	splitSummary = $derived(this.splitOutcome ? sf2SplitSummary(this.splitOutcome) : '');
	splitNeedsAttention = $derived(
		this.splitOutcome ? sf2SplitNeedsAttention(this.splitOutcome) : []
	);
	splitComplete = $derived(this.splitOutcome ? sf2SplitIsComplete(this.splitOutcome) : false);

	// ── Loading ───────────────────────────────────────────────────────────────
	/**
	 * Read the screen. Every read is independent and none of them blocks another,
	 * so they go together and each one's failure is its own message.
	 */
	async load(): Promise<void> {
		await Promise.all([this.loadMonths(), this.loadLaunch()]);
	}

	private async loadLaunch(): Promise<void> {
		try {
			// `classId` left out on purpose: the Settings screen has no class
			// selector, and the backend resolves the only class on record.
			this.launch = await getSf2LaunchMonth();
		} catch {
			// A fresh install with no class is an error on the backend and a
			// non-event here - the list below will say the same thing in plainer
			// words. The E1 banner is an addition to this screen, never a
			// precondition for it.
			this.launch = null;
		}
	}

	private async loadMonths(): Promise<void> {
		this.monthsLoading = true;
		this.monthsError = null;
		try {
			this.months = await listSf2MonthWorkbooks();
		} catch (error) {
			// Shown in place of the list rather than as a toast, because it is a
			// permanent state of the section until it can be read - a toast would
			// have been gone before anyone looked this way.
			this.months = [];
			this.monthsError = this.errorMessage(error, 'the month list could not be read');
		} finally {
			this.monthsLoading = false;
		}
	}

	// ── The split ─────────────────────────────────────────────────────────────
	/**
	 * Re-run §11 for the months that still need it.
	 *
	 * No confirmation, because the split is idempotent and refuses to rebuild a
	 * month that is already split - pressing this twice is harmless, and a
	 * confirmation dialog on a button that cannot do damage is one more thing
	 * between a teacher and fixing a month.
	 */
	async onRunSplit(): Promise<void> {
		if (this.splitRunning) return;
		this.splitRunning = true;
		try {
			const outcome = await runSf2WorkbookSplit();
			this.splitOutcome = outcome;
			this.ctx.toast(
				outcome.needsAttentionCount > 0
					? `${outcome.verifiedCount} months ready, ${outcome.needsAttentionCount} need attention.`
					: `All ${outcome.verifiedCount} months are ready.`
			);
			await Promise.all([this.loadMonths(), this.loadLaunch()]);
		} catch (error) {
			this.ctx.toast(
				`The split could not run: ${this.errorMessage(error, 'unknown error')}`,
				false
			);
		} finally {
			this.splitRunning = false;
		}
	}

	// ── Helpers ───────────────────────────────────────────────────────────────
	private errorMessage(error: unknown, fallback: string): string {
		return describeError(error, fallback);
	}
}

export const sf2State = new Sf2State();
