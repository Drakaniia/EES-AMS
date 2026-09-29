import {
	getSf2LaunchMonth,
	getSf2SchoolCalendarSettings,
	listSf2MonthWorkbooks,
	runSf2WorkbookSplit,
	setSf2SchoolStartDate
} from '$lib/features/settings/native';
import {
	isSchoolStartDateValid,
	normalizeSchoolStartDate,
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
 * Four things, replacing the three workflows D18 removed:
 *
 * 1. **Classes started on** - the one input every month's `first_school_day` is
 *    derived from. It is empty until the teacher types it, and there is no
 *    default: a guessed start date silently mis-dates every month file in the
 *    school year, which is the exact class of bug this model exists to eliminate.
 * 2. **Month workbooks** - twelve read-only rows: is the file there, how many X
 *    were last counted in it, when was it last written.
 * 3. **Back up workbooks now** - D13. Lives in Data Management, beside *Back Up
 *    Now*, and already wired; this screen links to it rather than duplicating it.
 * 4. **Re-run the workbook split** - §11, for a month that failed to split.
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

	/** `YYYY-MM-DD`, or `''` when the teacher has never entered one. */
	schoolStartDate = $state('');
	/** True while the setting is still `NULL` in the database (edge case E3). */
	needsSchoolStartDate = $state(false);
	schoolStartDateLoading = $state(false);
	schoolStartDateSaving = $state(false);

	splitRunning = $state(false);
	splitOutcome = $state<Sf2SplitOutcome | null>(null);

	// ── Derived ───────────────────────────────────────────────────────────────
	/** Twelve rows, sorted SEPTEMBER → AUGUST, ready to render. */
	monthRows = $derived.by<Sf2MonthWorkbookRow[]>(() => sf2MonthWorkbookRows(this.months));

	/** The one-line state of the year, under the list. */
	monthSummary = $derived(sf2MonthWorkbookSummary(this.monthRows));

	/**
	 * The E3 prompt, shown only while the start date is genuinely unset.
	 *
	 * Not shown because the field is empty - the field is empty on first render
	 * and until the read lands. Shown because the *database* says `NULL`, which is
	 * the only answer that means "nobody has been asked yet".
	 */
	showSchoolStartDatePrompt = $derived(this.needsSchoolStartDate);

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
		await Promise.all([this.loadSchoolStartDate(), this.loadMonths(), this.loadLaunch()]);
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

	private async loadSchoolStartDate(): Promise<void> {
		this.schoolStartDateLoading = true;
		try {
			const settings = await getSf2SchoolCalendarSettings();
			this.applySchoolStartDate(settings.schoolStartDate);
			this.needsSchoolStartDate = !settings.schoolStartDate;
		} catch (error) {
			this.ctx.toast(
				`Could not read "Classes started on": ${this.errorMessage(error, 'unknown error')}`,
				false
			);
		} finally {
			this.schoolStartDateLoading = false;
		}
	}

	private applySchoolStartDate(value: string | null): void {
		this.schoolStartDate = value ?? '';
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

	// ── "Classes started on" ───────────────────────────────────────────────────
	/**
	 * Save the typed start date, or clear it.
	 *
	 * An invalid date is refused before the invoke, so a typo cannot reach the
	 * column: every month's `first_school_day` is derived from this value, and a
	 * bad one mis-dates the whole year. Clearing is allowed and is not a mistake -
	 * the app goes back to asking (E3).
	 */
	async saveSchoolStartDate(): Promise<void> {
		if (this.schoolStartDateSaving || this.schoolStartDateLoading) return;
		if (!isSchoolStartDateValid(this.schoolStartDate)) {
			this.ctx.toast('Enter the date as YYYY-MM-DD, or clear the field to unset it.', false);
			return;
		}

		this.schoolStartDateSaving = true;
		const value = normalizeSchoolStartDate(this.schoolStartDate);
		try {
			await setSf2SchoolStartDate(value);
			this.applySchoolStartDate(value);
			this.needsSchoolStartDate = value === null;
			this.ctx.toast(
				value
					? 'Saved. Each month will be dated from this.'
					: 'Cleared. The app will ask again until this is entered.'
			);
			// The months' own `first_school_day` is derived from this value, and
			// the list shows whether each one is dated yet - so it is now stale.
			await this.loadMonths();
		} catch (error) {
			this.ctx.toast(
				`Could not save "Classes started on": ${this.errorMessage(error, 'unknown error')}`,
				false
			);
		} finally {
			this.schoolStartDateSaving = false;
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
		if (error instanceof Error) return error.message;
		if (typeof error === 'string') return error;
		return fallback;
	}
}

export const sf2State = new Sf2State();
