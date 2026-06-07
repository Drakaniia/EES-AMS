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
	class="sticky top-0 z-40 shrink-0 border-b border-border bg-background/88 px-4 py-4 shadow-sm backdrop-blur-xl md:px-8 md:py-5 lg:px-10"
>
	<div
		class="mx-auto flex w-full max-w-[1600px] flex-col gap-4 xl:flex-row xl:items-center xl:justify-between"
	>
		<div class="min-w-0">
			{#if category}
				<div class="chip mb-2">
					<span class="truncate">{category}</span>
				</div>
			{/if}
			<h1
				class="text-balance-safe max-w-4xl text-2xl leading-tight font-black tracking-normal text-foreground md:text-3xl"
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
