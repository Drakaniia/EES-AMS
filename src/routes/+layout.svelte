<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import UpdateNotification from '$lib/components/ui/UpdateNotification.svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import ClickSpark from '$lib/components/ui/ClickSpark.svelte';
	import { bootstrapApp } from '$lib/bootstrap';
	import { onMount } from 'svelte';

	let { children } = $props();

	onMount(() => {
		const initialLoading = document.getElementById('initial-loading');
		if (initialLoading) {
			initialLoading.style.display = 'none';
		}

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

		<ClickSpark sparkColor="#FF8A4C" sparkSize={8} sparkRadius={12} sparkCount={6} duration={300}>
			<AppShell>
				{@render children()}
			</AppShell>
		</ClickSpark>
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
