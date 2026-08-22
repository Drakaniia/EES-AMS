import { updateStore } from '$lib/stores/update.svelte';
import type { Ctx } from './state-context';

/**
 * Software Update section adapter.
 * Bridges the shared update store to the settings page: kicks off the update
 * lifecycle on mount, wires transient errors to the page toast, and guards
 * "Restart to Update" against unsaved Global Settings edits.
 */
class UpdateSectionState {
	ctx!: Ctx;
	restartConfirmOpen = $state(false);

	init(ctx: Ctx) {
		this.ctx = ctx;
	}

	/** Kicks off the staged-check → auto-check flow (called by the orchestrator). */
	start() {
		void updateStore.init();
	}

	/** Restart requested: confirm first if Global Settings have unsaved edits. */
	onRestartRequested() {
		if (this.ctx.hasUnsavedGlobalSettings()) {
			this.restartConfirmOpen = true;
			return;
		}
		void this.confirmRestart();
	}

	async confirmRestart() {
		this.restartConfirmOpen = false;
		await updateStore.restart();
		if (updateStore.error) {
			this.ctx.toast(`Update failed: ${updateStore.error}`, false);
		}
	}
}

export const updateSectionState = new UpdateSectionState();
