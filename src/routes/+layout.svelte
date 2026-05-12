<script lang="ts">
	import '../app.css';
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import UpdateNotification from '../components/ui/UpdateNotification.svelte';
	import ClickSpark from '$lib/components/ui/ClickSpark.svelte';
	import TitleBar from '$lib/components/ui/TitleBar.svelte';
	import { onMount } from 'svelte';

	let { children } = $props();

	onMount(() => {
		// Hide the initial loading screen once SvelteKit is ready
		const initialLoading = document.getElementById('initial-loading');
		if (initialLoading) {
			initialLoading.style.display = 'none';
		}
	});
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<div class="app-container">
	<TitleBar />

	<div class="content-container">
		<UpdateNotification />

		<ClickSpark sparkColor="#FF8A4C" sparkSize={8} sparkRadius={12} sparkCount={6} duration={300}>
			{@render children()}
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
