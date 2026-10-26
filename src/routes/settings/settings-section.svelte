<script lang="ts">
	import type { Snippet } from 'svelte';

	/** Pane heading, one short line of purpose, then the groups. */
	let {
		title,
		description,
		actions,
		children
	}: {
		title: string;
		description?: string;
		actions?: Snippet;
		children: Snippet;
	} = $props();
</script>

<header class="flex items-end justify-between gap-4">
	<div class="min-w-0">
		<h2 class="text-[17px] leading-6 font-semibold tracking-[-0.018em]">{title}</h2>
		{#if description}
			<p class="mt-1 max-w-prose text-[12.5px] leading-5 text-muted-foreground">{description}</p>
		{/if}
	</div>
	{#if actions}
		<div class="flex shrink-0 items-center gap-2">{@render actions()}</div>
	{/if}
</header>

<div class="mt-4 space-y-3">
	{@render children()}
</div>
