<script lang="ts">
	import { CircleX } from 'lucide-svelte';

	type Props = {
		monthSwitchLoading: boolean;
		monthSwitchMessage: string;
		monthSwitchError: string | null;
		monthSwitchProgressPercent: number;
		onDismissError?: () => void;
	};

	let {
		monthSwitchLoading,
		monthSwitchMessage,
		monthSwitchError,
		monthSwitchProgressPercent = 0,
		onDismissError
	}: Props = $props();
</script>

{#if monthSwitchLoading}
	<div
		role="dialog"
		aria-modal="true"
		aria-label="Switching SF2 report month"
		class="fixed inset-x-0 top-8 bottom-0 z-[70] flex items-center justify-center bg-background/40"
	>
		<div
			class="flex w-full max-w-sm flex-col items-center gap-5 rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl"
			role="status"
			aria-live="polite"
		>
			<!-- Animated bouncing dots (hide when progress bar is visible) -->
			{#if monthSwitchProgressPercent <= 0}
				<div class="flex items-center gap-1" aria-hidden="true">
					<span class="loading-dot size-2.5 rounded-full bg-primary"></span>
					<span class="loading-dot size-2.5 rounded-full bg-primary" style="animation-delay: 200ms"
					></span>
					<span class="loading-dot size-2.5 rounded-full bg-primary" style="animation-delay: 400ms"
					></span>
				</div>
			{/if}

			<!-- Progress bar (visible when backend reports progress) -->
			{#if monthSwitchProgressPercent > 0}
				<div class="w-full space-y-2">
					<div
						class="h-2 w-full overflow-hidden rounded-full bg-muted"
						role="progressbar"
						aria-valuenow={monthSwitchProgressPercent}
						aria-valuemin="0"
						aria-valuemax="100"
						aria-valuetext={`${monthSwitchProgressPercent} percent`}
					>
						<div
							class="h-full rounded-full bg-primary transition-all duration-500 ease-out"
							style="width: {monthSwitchProgressPercent}%"
						></div>
					</div>
				</div>
			{/if}

			<!-- Rotating / real progress message -->
			<div class="space-y-1">
				<p class="text-sm font-semibold text-foreground transition-all duration-500 ease-out">
					{monthSwitchMessage}
				</p>
				<p class="text-xs text-muted-foreground transition-opacity duration-300">
					{#if monthSwitchProgressPercent >= 100}
						Almost done — loading preview…
					{:else if monthSwitchProgressPercent > 0}
						Updating the SF2 workbook for the new month
					{:else}
						Updating workbook calendar and attendance marks
					{/if}
				</p>
			</div>
		</div>
	</div>
{/if}

{#if monthSwitchError}
	<div
		role="dialog"
		aria-modal="true"
		aria-label="Month switch failed"
		class="fixed inset-x-0 top-8 bottom-0 z-[70] flex items-center justify-center bg-background/40"
		tabindex="-1"
		onkeydown={(e) => {
			if (e.key === 'Escape') onDismissError?.();
		}}
	>
		<div
			class="flex w-full max-w-sm flex-col items-center gap-5 rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl"
		>
			<div class="flex size-12 items-center justify-center rounded-full bg-red-50 text-red-600">
				<CircleX class="size-6" aria-hidden="true" />
			</div>
			<div class="space-y-2">
				<h3 class="text-base font-semibold text-foreground">Could not switch month</h3>
				<p class="text-sm leading-relaxed text-muted-foreground">{monthSwitchError}</p>
			</div>
			<button
				type="button"
				onclick={onDismissError}
				class="control-ring rounded-md bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-colors hover:bg-accent"
			>
				Dismiss
			</button>
		</div>
	</div>
{/if}

<style>
	.loading-dot {
		animation: dot-bounce 1.4s ease-in-out infinite both;
	}

	@keyframes dot-bounce {
		0%,
		80%,
		100% {
			transform: scale(0.4);
			opacity: 0.3;
		}
		40% {
			transform: scale(1);
			opacity: 1;
		}
	}
</style>
