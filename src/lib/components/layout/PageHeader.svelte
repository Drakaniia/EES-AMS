<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		category,
		title,
		description,
		actions
	}: {
		category?: string;
		title: string;
		description?: string;
		actions?: Snippet;
	} = $props();
</script>

<header class="shrink-0 border-b border-border bg-background/95 px-6 py-5 md:px-12 md:py-6">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
		<div class="min-w-0">
			{#if category}
				<div
					class="mb-2 inline-flex rounded-full border border-border bg-surface px-2.5 py-1 font-mono text-[10px] font-bold tracking-wider text-muted-foreground uppercase"
				>
					{category}
				</div>
			{/if}
			<h1 class="max-w-3xl text-3xl leading-tight font-semibold tracking-normal md:text-[34px]">
				{title}
			</h1>
			{#if description}
				<p class="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground md:text-[15px]">
					{description}
				</p>
			{/if}
		</div>
		{#if actions}
			<div class="flex shrink-0 flex-wrap gap-2">
				{@render actions()}
			</div>
		{/if}
	</div>
</header>
