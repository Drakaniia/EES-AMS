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
	class="surface-panel flex min-h-48 w-full flex-col items-center justify-center border-dashed p-6 text-center"
	role="status"
>
	<div
		class="grid size-12 place-items-center rounded-xl border {tone === 'warning'
			? 'border-destructive/20 bg-destructive/10 text-destructive'
			: 'border-border bg-background text-muted-foreground'}"
		aria-hidden="true"
	>
		<Icon class="size-5" />
	</div>
	<h3 class="text-balance-safe mt-4 max-w-md text-base font-black text-foreground">{title}</h3>
	{#if description}
		<p class="text-balance-safe mt-1 max-w-md text-sm leading-6 text-muted-foreground">
			{description}
		</p>
	{/if}
	{#if actions}
		<div class="mt-5 flex flex-wrap justify-center gap-2">
			{@render actions()}
		</div>
	{/if}
</div>
