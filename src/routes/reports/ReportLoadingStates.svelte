<script lang="ts">
	import { resolve } from '$app/paths';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';

	type Props = {
		/**
		 * The first read is still in flight. Nothing is wrong yet and nothing is
		 * known, so this renders nothing at all - the grid area shows its own
		 * skeleton instead.
		 */
		loading: boolean;
		loadError: string | null;
		hasGrid: boolean;
		issues: string[];
		onRetry?: () => void;
	};

	let { loading, loadError, hasGrid, issues, onRetry }: Props = $props();

	/**
	 * The two states that genuinely replace the page: something failed, or there
	 * is nothing to show at all.
	 *
	 * There is no third state here any more. This component used to own a
	 * full-page `LoadingBlock` that covered the whole route while a month or a
	 * class loaded, which is why a class switch felt like the app had hung. A
	 * month switch is now a single SQL read and the grid gets its own skeleton
	 * instead - see `ReportGridSkeleton.svelte` - so the sidebar, the month picker
	 * and the class selector never stop responding.
	 */
</script>

{#if loadError}
	<div class="px-4 py-5 md:px-8 lg:px-10">
		<EmptyState tone="warning" title="SF2 reports are unavailable" description={loadError}>
			{#snippet actions()}
				<button
					type="button"
					onclick={onRetry}
					class="control-ring rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-surface"
				>
					Retry
				</button>
			{/snippet}
		</EmptyState>
	</div>
{:else if !loading && !hasGrid}
	<div class="px-4 py-5 md:px-8 lg:px-10">
		<EmptyState
			tone="warning"
			title="No SF2 workbook is ready for review"
			description={issues[0] ??
				'Import an SF2 workbook or create one from the bundled template first.'}
		>
			{#snippet actions()}
				<a
					href={resolve('/settings')}
					class="control-ring inline-flex rounded-pill bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground hover:bg-accent"
				>
					Open SF2 Settings
				</a>
			{/snippet}
		</EmptyState>
	</div>
{/if}
