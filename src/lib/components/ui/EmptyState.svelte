<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Inbox, TriangleAlert } from 'lucide-svelte';

	type Props = {
		title: string;
		description?: string;
		tone?: 'neutral' | 'warning';
		actions?: Snippet;
	};

	let { title, description, tone = 'neutral', actions }: Props = $props();
	const Icon = $derived(tone === 'warning' ? TriangleAlert : Inbox);
</script>

<div
	class="flex min-h-52 w-full flex-col items-center justify-center rounded-2xl border border-dashed px-6 py-10 text-center {tone ===
	'warning'
		? 'border-destructive/25 bg-destructive/[0.035]'
		: 'border-border bg-card/40'}"
	role="status"
>
	<div
		class="grid size-10 place-items-center rounded-full {tone === 'warning'
			? 'bg-destructive/10 text-destructive'
			: 'bg-surface text-muted-foreground'}"
		aria-hidden="true"
	>
		<Icon class="size-[18px]" strokeWidth={1.8} />
	</div>
	<h3
		class="text-balance-safe mt-4 max-w-md text-[15px] font-semibold tracking-[-0.01em] text-foreground"
	>
		{title}
	</h3>
	{#if description}
		<p class="text-balance-safe mt-1.5 max-w-md text-[13px] leading-5 text-muted-foreground">
			{description}
		</p>
	{/if}
	{#if actions}
		<div class="mt-5 flex flex-wrap justify-center gap-2">
			{@render actions()}
		</div>
	{/if}
</div>
