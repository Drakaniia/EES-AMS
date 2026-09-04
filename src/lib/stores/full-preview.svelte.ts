/**
 * Shared reactive state for signalling full-preview mode from the reports page
 * to the AppShell layout. When active, AppShell hides its TitleBar and bottom
 * nav so the SF2 attendance grid occupies the full window width.
 *
 * Usage (reports/+page.svelte):
 *   $effect(() => { fullPreviewStore.isActive = page.fullReviewOpen; });
 *
 * Usage (AppShell.svelte):
 *   {#if !fullPreviewStore.isTitleBarHidden}
 *     <TitleBar />
 *   {/if}
 */

let isActive = $state(false);

export const fullPreviewStore = {
	get isActive() {
		return isActive;
	},
	set isActive(v: boolean) {
		isActive = v;
	},
	/** Hide all chrome (TitleBar + bottom nav) in full-preview mode. */
	get isTitleBarHidden() {
		return isActive;
	}
};
