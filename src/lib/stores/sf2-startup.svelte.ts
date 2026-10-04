import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { getSf2LaunchMonth, runSf2WorkbookSplit } from '$lib/api';
import { SF2_SCHOOL_START_DATE_PROMPT } from '$lib/features/settings/sf2-heal-toast';

/**
 * The one thing the app has to say at startup that no route owns: the E3 prompt
 * (spec �11.1). "Enter the date classes started" is a question about the whole
 * install, asked until it is answered, and it is decided by
 * `getSf2LaunchMonth`'s `needsSchoolStartDate` - a read that needs no new backend
 * command and no workbook write.
 *
 * ## Why it tolerates silence
 *
 * `start()` is fire-and-forget and never rejects the app. A launch that produces
 * no message is a normal launch, not a failed one.
 */
class Sf2StartupStore {
	message = $state<string | null>(null);
	/** Set only for the E3 prompt, which is the one notice with somewhere to go. */
	actionLabel = $state<string | null>(null);
	onAction: (() => void) | null = null;

	private started = false;
	private timer: ReturnType<typeof setTimeout> | null = null;

	/**
	 * Ask for the start date, once per launch, until it is answered.
	 *
	 * Read-only and cheap: `getSf2LaunchMonth` is a SQL read with no workbook
	 * write, and a fresh install with no class is an error on the backend - which
	 * is not a reason to prompt about a workbook that does not exist yet, so the
	 * error is swallowed and the launch is left as it was.
	 */
	private async askForSchoolStartDate(): Promise<void> {
		try {
			const launch = await getSf2LaunchMonth();
			if (!launch.needsSchoolStartDate) return;
			this.show({
				message: SF2_SCHOOL_START_DATE_PROMPT,
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

	/**
	 * Ask once per launch. Safe to call more than once - the second call is a
	 * no-op, so a re-mounted shell cannot end up asking twice.
	 */
	async start(): Promise<void> {
		if (this.started) return;
		this.started = true;
		void this.askForSchoolStartDate();
		void this.ensureSplit();
	}

	/**
	 * Silent auto-run of the workbook split (§11). Idempotent: months already
	 * split are left alone, so this is a no-op on a healthy install. Speaks
	 * only when months need attention - success stays silent.
	 */
	private async ensureSplit(): Promise<void> {
		try {
			const outcome = await runSf2WorkbookSplit();
			if (outcome.needsAttentionCount > 0) {
				this.show({
					message: `${outcome.verifiedCount} months ready, ${outcome.needsAttentionCount} need attention. See Settings → SF2 Workbook.`,
					actionLabel: 'Review',
					onAction: () => {
						void goto(resolve('/settings') + '#settings-sf2');
					}
				});
			}
		} catch {
			// No workbook yet, or the database is not readable. The Settings
			// screen says the same thing in plainer words when it is opened.
		}
	}

	private show(notice: {
		message: string;
		actionLabel: string | null;
		onAction: (() => void) | null;
	}): void {
		this.message = notice.message;
		this.actionLabel = notice.actionLabel;
		this.onAction = notice.onAction;

		// A quiet note about the start date is not worth waiting for.
		if (this.timer) clearTimeout(this.timer);
		this.timer = setTimeout(() => this.dismiss(), 10_000);
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
