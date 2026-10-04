import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { runSf2WorkbookSplit } from '$lib/api';

/**
 * The one thing the app has to say at startup that no route owns: the
 * workbook-split notice (§11).
 */
class Sf2StartupStore {
	message = $state<string | null>(null);
	/** Set only for the E3 prompt, which is the one notice with somewhere to go. */
	actionLabel = $state<string | null>(null);
	onAction: (() => void) | null = null;

	private started = false;
	private timer: ReturnType<typeof setTimeout> | null = null;

	/**
	 * Ask once per launch. Safe to call more than once - the second call is a
	 * no-op, so a re-mounted shell cannot end up asking twice.
	 */
	async start(): Promise<void> {
		if (this.started) return;
		this.started = true;
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

		// A quiet split note is not worth waiting for.
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
