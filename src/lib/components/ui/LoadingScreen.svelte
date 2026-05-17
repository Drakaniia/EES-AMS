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

			<!-- Loading Spinner -->
			<div class="spinner-container">
				<div class="custom-spinner"></div>
			</div>

			<!-- Loading Text -->
			<div class="loading-text text-center">
				<p class="animate-pulse text-sm text-muted-foreground">Loading application...</p>
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

		.custom-spinner {
			width: 40px;
			height: 40px;
			border: 4px solid rgba(0, 0, 0, 0.1);
			border-top: 4px solid #f97316;
			border-radius: 50%;
			animation: spin 1s linear infinite;
		}

		@keyframes spin {
			0% {
				transform: rotate(0deg);
			}
			100% {
				transform: rotate(360deg);
			}
		}
	</style>
{/if}
