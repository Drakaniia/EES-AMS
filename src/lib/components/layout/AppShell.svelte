<script lang="ts">
	import { fullPreviewStore } from '$lib/stores/full-preview.svelte';
	import TitleBar from '$lib/components/ui/TitleBar.svelte';
	import CommandPalette from '$lib/components/ui/CommandPalette.svelte';

	let { children } = $props();
</script>

<div class="app-surface flex h-full min-h-0 flex-col overflow-hidden text-foreground">
	<a
		href="#main-content"
		class="sr-only focus:not-sr-only focus:fixed focus:top-10 focus:left-4 focus:z-[80] focus:rounded-md focus:bg-primary focus:px-4 focus:py-2 focus:text-sm focus:font-semibold focus:text-primary-foreground"
	>
		Skip to content
	</a>

	{#if !fullPreviewStore.isTitleBarHidden}
		<TitleBar />
	{/if}

	<main
		id="main-content"
		class="min-h-0 min-w-0 flex-1 overflow-auto focus:outline-none"
		tabindex="-1"
	>
		{@render children()}
	</main>

	<CommandPalette />
</div>
