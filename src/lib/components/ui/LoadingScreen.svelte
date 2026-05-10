<script lang="ts">
	import { Spinner } from 'flowbite-svelte';
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
		class="loading-screen bg-background fixed inset-0 z-50 flex flex-col items-center justify-center"
	>
		<div class="loading-content flex flex-col items-center gap-6">
			<!-- App Logo/Branding -->
			<div class="branding text-center">
				<h1 class="text-foreground mb-2 text-3xl font-bold">EES</h1>
				<p class="text-muted-foreground text-sm">Attendance Management System</p>
			</div>

			<!-- Loading Spinner -->
			<div class="spinner-container">
				<Spinner type="bars" color="orange" />
			</div>

			<!-- Loading Text -->
			<div class="loading-text text-center">
				<p class="text-muted-foreground animate-pulse text-sm">Loading application...</p>
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

		.animate-pulse {
			animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
		}

		@keyframes pulse {
			0%,
			100% {
				opacity: 1;
			}
			50% {
				opacity: 0.5;
			}
		}
	</style>
{/if}
