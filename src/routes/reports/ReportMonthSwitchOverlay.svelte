<script lang="ts">
	import { CircleX } from 'lucide-svelte';

	type Props = {
		monthSwitchLoading: boolean;
		monthSwitchMessage: string;
		monthSwitchError: string | null;
		onDismissError?: () => void;
	};

	let { monthSwitchLoading, monthSwitchMessage, monthSwitchError, onDismissError }: Props =
		$props();
</script>

{#if monthSwitchLoading}
	<div
		role="dialog"
		aria-modal="true"
		aria-label="Switching SF2 report month"
		class="fixed inset-0 z-[70] flex items-center justify-center bg-background/40 backdrop-blur-[2px]"
	>
		<div
			class="flex w-full max-w-sm flex-col items-center gap-5 rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl"
			role="status"
			aria-live="polite"
		>
			<!-- Animated bouncing dots -->
			<div class="flex items-center gap-1" aria-hidden="true">
				<span class="loading-dot size-2.5 rounded-full bg-primary"></span>
				<span class="loading-dot size-2.5 rounded-full bg-primary" style="animation-delay: 200ms"
				></span>
				<span class="loading-dot size-2.5 rounded-full bg-primary" style="animation-delay: 400ms"
				></span>
			</div>

			<!-- Rotating friendly message -->
			<div class="space-y-1">
				<p class="text-sm font-semibold text-foreground transition-all duration-500 ease-out">
					{monthSwitchMessage}
				</p>
				<p class="text-xs text-muted-foreground transition-opacity duration-300">
					Updating workbook calendar and attendance marks
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
		class="fixed inset-0 z-[70] flex items-center justify-center bg-background/40 backdrop-blur-[2px]"
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
