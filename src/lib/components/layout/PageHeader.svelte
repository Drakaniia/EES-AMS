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

<header
	class="shrink-0 border-b border-border bg-background/94 px-4 py-4 backdrop-blur md:px-8 lg:px-10"
>
	<div
		class="mx-auto flex w-full max-w-[1600px] flex-col gap-4 xl:flex-row xl:items-center xl:justify-between"
	>
		<div class="min-w-0">
			{#if category}
				<div
					class="mb-2 inline-flex max-w-full rounded-pill border border-border bg-surface px-2.5 py-1 font-mono text-[10px] font-bold tracking-wider text-muted-foreground uppercase"
				>
					<span class="truncate">{category}</span>
				</div>
			{/if}
			<h1
				class="text-balance-safe max-w-4xl text-2xl leading-tight font-semibold tracking-normal md:text-3xl"
			>
				{title}
			</h1>
			{#if description}
				<p class="text-balance-safe mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">
					{description}
				</p>
			{/if}
		</div>
		{#if actions}
			<div class="flex shrink-0 flex-wrap items-center gap-2 xl:justify-end">
				{@render actions()}
			</div>
		{/if}
	</div>
</header>
