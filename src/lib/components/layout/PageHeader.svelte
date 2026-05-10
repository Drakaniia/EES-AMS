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

<header class="border-border border-b px-6 pt-10 pb-8 md:px-12 md:pt-16">
	<div class="flex flex-col gap-6 md:flex-row md:items-end md:justify-between">
		<div>
			{#if category}
				<div
					class="bg-surface border-border text-muted-foreground mb-4 inline-block rounded-full border px-3 py-1 font-mono text-[10px] font-bold tracking-wider uppercase"
				>
					{category}
				</div>
			{/if}
			<h2 class="display-lg max-w-2xl">{title}</h2>
			{#if description}
				<p class="text-muted-foreground mt-4 max-w-xl text-base">{description}</p>
			{/if}
		</div>
		{#if actions}
			<div class="flex flex-wrap gap-2">
				{@render actions()}
			</div>
		{/if}
	</div>
</header>
