import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { getSf2LaunchMonth, onSf2HealOutcome } from '$lib/db-rust';
import { sf2HealNotice, SF2_SCHOOL_START_DATE_PROMPT } from '$lib/features/settings/sf2-heal-toast';
import type { Sf2HealOutcome } from '$lib/types';

/**
 * The two things the app has to say at startup that no route owns.
 *
 * ## Why this is a singleton and not route state
 *
 * Both are, by construction, not tied to a screen:
 *
 * 1. **The self-heal's outcome** (spec §8.2 step 6, acceptance #15). The heal is a
 *    Rust thread started from `setup`, before the webview exists, and it is never
 *    joined - so it has no caller to return to and cannot be awaited by whatever
 *    route happens to be open when its COM pass finishes. A recovery found while
 *    the teacher is on the Dashboard has to be able to say so there.
 * 2. **The E3 prompt** (spec §11.1). "Enter the date classes started" is a
 *    question about the whole install, asked until it is answered, and it is
 *    decided by `get_sf2_launch_month`'s `needsSchoolStartDate` - a read that
 *    needs no new backend command and no Excel.
 *
 * One store, one message, so the two can never overlap on screen. The recovery
 * wins: it is the only one of the two that is about marks the teacher believes
 * they took, and it is the one they will want to read.
 *
 * ## Why it tolerates silence
 *
 * `start()` is fire-and-forget and never rejects the app. The heal may not finish
 * for seconds, and it may never recover anything at all. A launch that produces
 * no message is a normal launch, not a failed one.
 */
class Sf2StartupStore {
	message = $state<string | null>(null);
	ok = $state(true);
	/** Polite rather than assertive: a quiet note must not steal the teacher's place. */
	quiet = $state(false);
	/** Set only for the E3 prompt, which is the one notice with somewhere to go. */
	actionLabel = $state<string | null>(null);
	onAction: (() => void) | null = null;

	private started = false;
	private timer: ReturnType<typeof setTimeout> | null = null;

	/**
	 * Subscribe to the launch heal and ask the E3 question. Safe to call more than
	 * once - the second call is a no-op, so a re-mounted shell cannot end up with
	 * two listeners racing to show the same message.
	 */
	async start(): Promise<void> {
		if (this.started) return;
		this.started = true;

		try {
			await onSf2HealOutcome((outcome) => this.applyHeal(outcome));
		} catch (error) {
			// `listen` rejects when the webview is tearing down. Nothing to do, and
			// nothing to tell the user: the app is on its way out.
			console.debug('SF2 self-heal listener unavailable', error);
		}

		void this.askForSchoolStartDate();
	}

	/** Fold one heal outcome into the message, or say nothing. See `sf2HealNotice`. */
	applyHeal(outcome: Sf2HealOutcome): void {
		const notice = sf2HealNotice(outcome);
		if (!notice) return;

		this.show({
			message: notice.message,
			ok: notice.ok,
			kind: notice.kind,
			// A recovery never carries an action: there is nothing for the teacher
			// to do about a mark the app has already put back.
			actionLabel: null,
			onAction: null
		});
	}

	/**
	 * Ask for the start date, once per launch, until it is answered.
	 *
	 * Read-only and cheap: `get_sf2_launch_month` is a SQL read with no Excel, and
	 * a fresh install with no class is an error on the backend - which is not a
	 * reason to prompt about a workbook that does not exist yet, so the error is
	 * swallowed and the launch is left as it was.
	 */
	private async askForSchoolStartDate(): Promise<void> {
		try {
			const launch = await getSf2LaunchMonth();
			if (!launch.needsSchoolStartDate) return;
			this.show({
				message: SF2_SCHOOL_START_DATE_PROMPT,
				ok: true,
				kind: 'quiet',
				actionLabel: 'Set the date',
				onAction: () => {
					void goto(resolve('/settings') + '#settings-sf2');
				}
			});
		} catch {
			// No class, or the database is not readable. Neither is this prompt's
			// business, and the Settings screen asks the same question the moment
			// the app is usable.
		}
	}

	private show(notice: {
		message: string;
		ok: boolean;
		kind: 'toast' | 'quiet';
		actionLabel: string | null;
		onAction: (() => void) | null;
	}): void {
		this.message = notice.message;
		this.ok = notice.ok;
		this.quiet = notice.kind === 'quiet';
		this.actionLabel = notice.actionLabel;
		this.onAction = notice.onAction;

		// A recovery is worth reading slowly; a quiet note is not worth waiting for.
		if (this.timer) clearTimeout(this.timer);
		this.timer = setTimeout(() => this.dismiss(), notice.kind === 'toast' ? 8000 : 10_000);
	}

	dismiss(): void {
		if (this.timer) {
			clearTimeout(this.timer);
			this.timer = null;
		}
		this.message = null;
		this.actionLabel = null;
		this.onAction = null;
	}
}

export const sf2StartupStore = new Sf2StartupStore();
