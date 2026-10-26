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
	class="sticky top-0 z-40 shrink-0 border-b border-border bg-background px-5 py-4 md:px-8 md:py-5 lg:px-10"
>
	<div
		class="mx-auto flex w-full max-w-[1560px] flex-col gap-3 xl:flex-row xl:items-end xl:justify-between"
	>
		<div class="min-w-0">
			{#if category}
				<p class="mb-1 text-[11px] font-semibold tracking-[0.01em] text-muted-foreground">
					{category}
				</p>
			{/if}
			<h1 class="text-[23px] leading-[1.15] font-semibold tracking-[-0.024em] text-foreground">
				{title}
			</h1>
			{#if description}
				<p class="text-balance-safe mt-1 max-w-2xl text-[13px] leading-5 text-muted-foreground">
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
