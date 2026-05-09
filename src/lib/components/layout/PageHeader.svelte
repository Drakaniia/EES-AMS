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

<header class="px-6 md:px-12 pt-10 md:pt-16 pb-8 border-b border-border">
	<div class="flex flex-col md:flex-row md:items-end md:justify-between gap-6">
		<div>
			{#if step}
				<div class="label-mono mb-3">{step}</div>
			{/if}
			<h2 class="display-lg max-w-2xl">{title}</h2>
			{#if description}
				<p class="mt-4 text-base text-muted-foreground max-w-xl">{description}</p>
			{/if}
		</div>
		{#if actions}
			<div class="flex gap-2 flex-wrap">
				{@render actions()}
			</div>
		{/if}
	</div>
</header>
