import type { Sf2HealOutcome } from '$lib/types';

/**
 * What the startup self-heal is worth saying out loud.
 *
 * - `toast` - an interrupting message. One of these and only one: a recovery.
 * - `quiet` - worth knowing, not worth interrupting for. Rendered in the same
 *   place but polite rather than assertive, and it does not steal focus.
 * - `null` - say nothing at all.
 */
export type Sf2HealNotice = {
	kind: 'toast' | 'quiet';
	/**
	 * `true` for a success. The toast component renders `false` as an assertive
	 * alert in the destructive colour, which is right for a failure and wrong for
	 * "Excel was not available, so nothing was checked" - so a quiet notice is
	 * always `ok: true` and carries the caution in its own words.
	 */
	ok: boolean;
	message: string;
};

/**
 * The four outcomes that must never produce a message of any kind.
 *
 * Named as a list rather than left implicit in a `match` so the test can assert
 * the *set*, which is the property that matters: not "this variant is quiet" but
 * "these four, and no others, are the quiet ones". A fifth boring variant added
 * in Rust later shows up here as a failure rather than as a new toast nobody
 * chose.
 */
export const SF2_HEAL_SILENT_STATUSES = [
	'alreadyRan',
	'notApplicable',
	'upToDate',
	'inSync'
] as const satisfies readonly Sf2HealOutcome['status'][];

type Sf2HealStatus = Sf2HealOutcome['status'];

function isSilentStatus(status: Sf2HealStatus): boolean {
	return (SF2_HEAL_SILENT_STATUSES as readonly Sf2HealStatus[]).includes(status);
}

/**
 * The §8.2 step 6 toast, or `null` when there is nothing to say.
 *
 * The mirror of `Sf2HealOutcome::toast()` on the Rust side, and deliberately so:
 * "only a recovery toasts" is a decision both ends have to agree on, and two
 * implementations of it that disagree produce a toast nobody can explain.
 *
 * ## Why the boring outcomes are silent
 *
 * `UpToDate` and `InSync` are the *expected* state of a healthy install - they
 * are what four launches out of five produce. `AlreadyRan` means the latch did
 * its job. `NotApplicable` means there is no month row yet, which is every month
 * before the split has run. A toast for any of them is a message on every single
 * launch, and a teacher who dismisses a message on every launch has stopped
 * reading messages from this app - which is precisely the message that matters
 * when a real recovery happens. Silence is the only way the one real toast gets
 * read.
 */
export function sf2HealToast(outcome: Sf2HealOutcome): string | null {
	if (isSilentStatus(outcome.status)) return null;
	if (outcome.status !== 'recovered') return null;
	// The backend already worded this exactly as §8.2 step 6 does. Fall back to
	// composing it only if it arrived empty, rather than inventing a second
	// wording that could drift from the Rust one.
	return outcome.toast.trim() || `Recovered ${outcome.imported} X marks from ${outcome.fileName}`;
}

/**
 * A notice that is worth showing without interrupting anyone.
 *
 * Two variants land here, and both are states §9.1's guard already refuses a write
 * on, so the app is not at risk - it just could not check:
 *
 * - `excelUnavailable` - no Excel, the file is open in Excel, or the layout is
 *   not what the scanner expects.
 * - `workbookMissing` - the month's row exists but the file is gone (E4).
 *
 * Neither is a modal. A modal here would be a dialog asking a teacher to
 * acknowledge that the app could not do something, at the moment they are trying
 * to open the app and start work.
 */
export function sf2HealQuietNotice(outcome: Sf2HealOutcome): string | null {
	switch (outcome.status) {
		case 'excelUnavailable':
			return outcome.reason
				? `Could not check the SF2 workbook: ${outcome.reason}`
				: 'Could not check the SF2 workbook.';
		case 'workbookMissing':
			return outcome.reason || 'An SF2 month workbook is missing. Restore it from a backup.';
		default:
			return null;
	}
}

/**
 * The whole decision, in one call: what - if anything - the launch heal is worth
 * showing. A recovery outranks a quiet notice, and a recovery can only ever be
 * one thing, so the two never contend.
 */
export function sf2HealNotice(outcome: Sf2HealOutcome): Sf2HealNotice | null {
	const toast = sf2HealToast(outcome);
	if (toast) return { kind: 'toast', ok: true, message: toast };

	const quiet = sf2HealQuietNotice(outcome);
	if (quiet) return { kind: 'quiet', ok: true, message: quiet };

	return null;
}

/**
 * The E3 prompt, as a sentence the Settings screen can show beside the empty
 * *Classes started on* field.
 *
 * Rust holds the same words in `SCHOOL_START_DATE_PROMPT`; they are kept in step
 * by hand, and the field shows the copy rather than inventing a second phrasing
 * beside it.
 */
export const SF2_SCHOOL_START_DATE_PROMPT =
	"Enter the date classes started so each month's SF2 can be dated automatically.";
