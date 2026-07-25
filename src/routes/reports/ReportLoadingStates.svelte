<script lang="ts">
	import { resolve } from '$app/paths';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import type { Sf2ExportPreview } from '$lib/db-rust';

	type Props = {
		loading: boolean;
		loadError: string | null;
		preview: Sf2ExportPreview | null;
		onRetry?: () => void;
	};

	let { loading, loadError, preview, onRetry }: Props = $props();
</script>

{#if loading}
	<div class="px-4 py-5 md:px-8 lg:px-10">
		<LoadingBlock rows={4} label="Loading SF2 workbook preview" />
	</div>
{:else if loadError}
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
{:else if !preview?.template}
	<div class="px-4 py-5 md:px-8 lg:px-10">
		<EmptyState
			tone="warning"
			title="No SF2 workbook is ready for review"
			description={preview?.issues[0] ??
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
