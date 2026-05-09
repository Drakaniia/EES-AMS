<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		step,
		title,
		description,
		actions
	}: {
		step?: string;
		title: string;
		description?: string;
		actions?: Snippet;
	} = $props();
</script>

<header class="border-border border-b px-6 pt-10 pb-8 md:px-12 md:pt-16">
	<div class="flex flex-col gap-6 md:flex-row md:items-end md:justify-between">
		<div>
			{#if step}
				<div class="label-mono mb-3">{step}</div>
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
