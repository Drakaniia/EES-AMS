<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * One label/description + control row. Rows sit directly inside a
	 * `SettingsGroup`, which draws the hairlines between them.
	 */
	let {
		label,
		description,
		tone = 'default',
		children
	}: {
		label: string;
		description?: string;
		tone?: 'default' | 'danger';
		children?: Snippet;
	} = $props();
</script>

<div class="flex items-center justify-between gap-4 px-4 py-3">
	<div class="min-w-0">
		<p class="text-[13px] leading-5 font-medium" class:text-destructive={tone === 'danger'}>
			{label}
		</p>
		{#if description}
			<p class="mt-0.5 text-xs leading-4 text-muted-foreground">{description}</p>
		{/if}
	</div>
	{#if children}
		<div class="flex shrink-0 items-center gap-2">{@render children()}</div>
	{/if}
</div>
