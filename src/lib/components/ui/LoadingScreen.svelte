<script lang="ts">
	import { onMount } from 'svelte';

	let isVisible = $state(true);

	onMount(() => {
		// Hide loading screen when component mounts (app is ready)
		setTimeout(() => {
			isVisible = false;
		}, 500);
	});
</script>

{#if isVisible}
	<div
		class="loading-screen fixed inset-0 z-50 flex flex-col items-center justify-center bg-background"
	>
		<div class="loading-content flex flex-col items-center gap-6">
			<!-- App Logo/Branding -->
			<div class="branding text-center">
				<h1 class="mb-2 text-3xl font-bold text-foreground">EES</h1>
				<p class="text-sm text-muted-foreground">Attendance Management System</p>
			</div>

			<div class="h-2 w-36 overflow-hidden rounded-pill border border-primary/20 bg-surface">
				<div class="h-full w-2/5 rounded-pill bg-primary"></div>
			</div>

			<div class="loading-text text-center">
				<p class="text-sm text-muted-foreground">Loading application...</p>
			</div>
		</div>
	</div>

	<style>
		.loading-screen {
			background-color: var(--color-background);
			color: var(--color-foreground);
		}

		.loading-content {
			animation: fadeIn 0.3s ease-in-out;
		}

		@keyframes fadeIn {
			from {
				opacity: 0;
				transform: translateY(10px);
			}
			to {
				opacity: 1;
				transform: translateY(0);
			}
		}
	</style>
{/if}
