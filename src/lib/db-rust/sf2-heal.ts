import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Sf2HealOutcome } from '../types';
export type { Sf2HealOutcome } from '../types';

/**
 * The event the startup self-heal publishes its outcome on.
 *
 * ## Why an event and not the command
 *
 * The heal is started by `app_lib::run`'s `setup`, on its own thread, *before* the
 * webview exists - and it is never joined, because a COM pass over forty
 * learners is seconds of Excel and app launch may not block on it. That means
 * there is no caller to hand the outcome to: the thread has no upward, and the
 * frontend cannot `await` a result nobody is holding.
 *
 * So the outcome is published. The frontend subscribes once at startup, and the
 * toast of §8.2 step 6 fires when - and only when - the run actually recovered
 * something. See [`sf2HealToast`]($lib/features/settings/sf2-heal-toast) for why
 * four of the seven variants are silent.
 */
export const SF2_HEAL_OUTCOME_EVENT = 'sf2-heal-outcome';

/**
 * Run the heal now, on the UI thread, and wait for its outcome.
 *
 * Explicit and repeatable - this is the command entry point, and it is the right
 * one for a "check this month again" button. It is **not** the launch path: the
 * launch heal has already been started by `setup` and holds the once-per-launch
 * latch, so calling this at startup would run a second COM pass against the same
 * file and race the first one. For the launch, subscribe to
 * [`SF2_HEAL_OUTCOME_EVENT`] instead.
 *
 * Every outcome is a value, including "could not measure" - see
 * [`Sf2HealOutcome`]($lib/types).
 */
export async function healCurrentMonthWorkbook(): Promise<Sf2HealOutcome> {
	return await invoke('heal_current_month_workbook');
}

/**
 * Subscribe to the launch heal's outcome.
 *
 * Safe to call before the heal finishes, and safe to call when it never finishes
 * at all: the handler simply never runs. Excel absent, the file locked, the
 * month's row missing - each of those is either a silent variant or a quiet
 * notice, and none of them is a reason to block the UI on a result.
 *
 * Returns the unsubscribe function, as Tauri's `listen` does.
 */
export async function onSf2HealOutcome(
	handler: (outcome: Sf2HealOutcome) => void
): Promise<UnlistenFn> {
	return await listen<Sf2HealOutcome>(SF2_HEAL_OUTCOME_EVENT, (event) => {
		handler(event.payload);
	});
}
