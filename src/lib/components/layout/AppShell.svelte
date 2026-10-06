<script lang="ts">
	import { databaseStatus } from '$lib/stores/database-status.svelte';
	import TitleBar from '$lib/components/ui/TitleBar.svelte';
	import CommandPalette from '$lib/components/ui/CommandPalette.svelte';
	import DatabaseRecovery from '$lib/components/ui/DatabaseRecovery.svelte';
	import { onMount } from 'svelte';
	// What the app has to say at startup: the SF2 self-heal's outcome (spec §8.2
	// step 6, acceptance #15) and the E3 "classes started on" prompt. Both are
	// started by Rust before the webview exists or asked of the whole install, so
	// neither can be surfaced from a route.
	import Sf2StartupToast from '$lib/components/ui/Sf2StartupToast.svelte';

	let { children } = $props();

	onMount(() => {
		// First-open probe: every route reads through the same driver, so one
		// probe here puts up the recovery gate no matter which page is showing.
		void databaseStatus.probe();
	});
</script>

<div class="app-surface flex h-full min-h-0 flex-col overflow-hidden text-foreground">
	<a
		href="#main-content"
		class="sr-only focus:not-sr-only focus:fixed focus:top-10 focus:left-4 focus:z-[80] focus:rounded-md focus:bg-primary focus:px-4 focus:py-2 focus:text-sm focus:font-semibold focus:text-primary-foreground"
	>
		Skip to content
	</a>

	<TitleBar />

	{#if databaseStatus.state === 'temporary'}
		<DatabaseRecovery compact />
	{/if}

	{#if databaseStatus.state === 'unavailable'}
		<div class="shrink-0 px-4 pt-4 md:px-8 lg:px-10">
			<DatabaseRecovery />
		</div>
	{/if}

	<main id="main-content" class="min-h-0 min-w-0 flex-1 overflow-auto focus:outline-none" tabindex="-1">
		{@render children()}
	</main>

	<CommandPalette />
	<Sf2StartupToast />
</div>
