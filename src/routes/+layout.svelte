<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import UpdateNotification from '$lib/components/ui/UpdateNotification.svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import { bootstrapApp } from '$lib/bootstrap';
	import { themeStore } from '$lib/stores/theme.svelte';
	import { onMount } from 'svelte';

	let { children } = $props();

	onMount(() => {
		const initialLoading = document.getElementById('initial-loading');
		if (initialLoading) {
			initialLoading.style.display = 'none';
		}

		themeStore.init();

		// Fire-and-forget: the bindings above are needed before the first read, but
		// nothing here blocks paint, and a backend that is not up yet surfaces as a
		// failed read rather than a blank screen.
		void bootstrapApp();
	});
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<div class="app-container">
	<div class="content-container">
		<UpdateNotification />

		<AppShell>
			{@render children()}
		</AppShell>
	</div>
</div>

<style>
	.app-container {
		height: 100vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.content-container {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		min-height: 0;
	}
</style>
