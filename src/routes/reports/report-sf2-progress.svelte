<script lang="ts">
	import { fade } from 'svelte/transition';
	import { AlertTriangle, CheckCircle2, CircleX } from 'lucide-svelte';

	type Sf2OpenStatus = 'idle' | 'syncing' | 'success' | 'error';

	type Props = {
		status: Sf2OpenStatus;
		error: string | null;
		resultPath: string | null;
		displayMessage: string;
		progressPercent: number;
		showWaitHint: boolean;
		isExcelError?: boolean;
		onRetry?: () => void;
		onKillAndRetry?: () => void;
		onClose?: () => void;
	};

	let {
		status,
		error,
		resultPath,
		displayMessage,
		progressPercent,
		showWaitHint,
		isExcelError = false,
		onRetry,
		onKillAndRetry,
		onClose
	}: Props = $props();

	let showKillConfirm = $state(false);

	function handleKillAndRetry() {
		showKillConfirm = false;
		onKillAndRetry?.();
	}

	function resetKillConfirm() {
		showKillConfirm = false;
	}
</script>

{#if status !== 'idle'}
	<div
		role="dialog"
		aria-modal="true"
		aria-label={status === 'error' ? 'Opening failed' : 'Opening SF2 workbook'}
		class="fixed inset-x-0 top-8 bottom-0 z-[70] flex items-center justify-center bg-background/40 transition-opacity"
		tabindex="-1"
		onkeydown={(e) => {
			if (e.key === 'Escape' && status === 'error') {
				onClose?.();
			}
		}}
	>
		{#if status === 'error'}
			<!-- Error state -->
			<div
				class="flex w-full max-w-sm flex-col items-center gap-5 rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl"
			>
				<div class="flex size-12 items-center justify-center rounded-full bg-red-50 text-red-600">
					<CircleX class="size-6" aria-hidden="true" />
				</div>
				<div class="space-y-2">
					<h3 class="text-base font-semibold text-foreground">Unable to open workbook</h3>
					<p class="text-sm leading-relaxed text-muted-foreground">{error}</p>
				</div>

				{#if isExcelError && onKillAndRetry && showKillConfirm}
					<!-- Kill confirmation — sits above the button row for breathing room -->
					<div class="flex w-full flex-col gap-3">
						<div
							class="flex items-start gap-2 rounded-lg border border-amber-200 bg-amber-50 p-3 text-left"
						>
							<AlertTriangle class="mt-0.5 size-4 shrink-0 text-amber-600" aria-hidden="true" />
							<p class="text-xs leading-relaxed text-amber-800">
								This will close <strong>all</strong> open Excel windows, including any unsaved work in
								other spreadsheets. Make sure you've saved everything in Excel first.
							</p>
						</div>
						<div class="flex justify-center gap-2">
							<button
								type="button"
								onclick={resetKillConfirm}
								class="control-ring rounded-md border border-border bg-background px-3 py-2 text-sm font-medium transition-colors hover:bg-surface"
							>
								Cancel
							</button>
							<button
								type="button"
								onclick={handleKillAndRetry}
								class="control-ring rounded-md bg-red-600 px-3 py-2 text-sm font-semibold text-white transition-colors hover:bg-red-700"
							>
								Kill &amp; Retry
							</button>
						</div>
					</div>
				{/if}

				<div class="flex gap-3">
					<button
						type="button"
						onclick={onClose}
						class="control-ring rounded-md border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
					>
						Close
					</button>
					{#if isExcelError && onKillAndRetry && !showKillConfirm}
						<button
							type="button"
							onclick={() => (showKillConfirm = true)}
							class="control-ring rounded-md border border-amber-300 bg-amber-50 px-4 py-2 text-sm font-semibold text-amber-800 transition-colors hover:bg-amber-100"
						>
							Force kill Excel
						</button>
					{/if}
					<button
						type="button"
						onclick={onRetry}
						class="control-ring rounded-md bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-colors hover:bg-accent"
					>
						Try again
					</button>
				</div>
			</div>
		{:else if status === 'success'}
			<!-- Success state -->
			<div
				class="flex w-full max-w-sm flex-col items-center gap-5 rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl"
				in:fade={{ duration: 200 }}
			>
				<div
					class="flex size-12 items-center justify-center rounded-full bg-emerald-50 text-emerald-600"
				>
					<CheckCircle2 class="size-6" aria-hidden="true" />
				</div>
				<div class="space-y-1">
					<h3 class="text-base font-semibold text-foreground">Workbook opened!</h3>
					<p class="text-xs text-muted-foreground">
						{resultPath ? `Location: ${resultPath}` : ''}
					</p>
				</div>
			</div>
		{:else}
			<!-- Progress state (syncing) -->
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

				<!-- Current friendly message -->
				<div class="space-y-1">
					<p class="text-sm font-semibold text-foreground transition-all duration-500 ease-out">
						{displayMessage}
					</p>
				</div>

				<!-- Determinate progress bar with percentage -->
				<div class="w-full space-y-2">
					<div
						class="h-3 w-full overflow-hidden rounded-pill border border-primary/20 bg-background"
						role="progressbar"
						aria-valuemin="0"
						aria-valuemax="100"
						aria-valuenow={progressPercent}
						aria-valuetext={`${progressPercent} percent`}
					>
						<div
							class="h-full rounded-pill bg-primary transition-all duration-400 ease-out"
							style="width: {progressPercent}%"
						></div>
					</div>
					<div class="label-mono text-xs text-primary">{progressPercent}%</div>
				</div>

				<!-- Subtle "closing soon" hint when at 100% -->
				{#if progressPercent >= 100}
					<p class="text-xs text-muted-foreground">Finalizing…</p>
				{:else if showWaitHint}
					<!-- Descriptive "please wait a little longer" hint shown when the
						 backend has been silent for a few seconds (slow Excel write). -->
					<div
						class="flex items-start justify-center gap-2 rounded-xl border border-primary/15 bg-primary/5 px-3 py-2 text-left text-xs leading-relaxed text-muted-foreground"
						transition:fade={{ duration: 300 }}
					>
						<span class="relative mt-1 flex size-2 shrink-0" aria-hidden="true">
							<span
								class="absolute inline-flex size-full animate-ping rounded-full bg-primary opacity-60"
							></span>
							<span class="relative inline-flex size-2 rounded-full bg-primary"></span>
						</span>
						<span>
							Excel is still working on the workbook in the background — this can take a little
							longer. Please wait a moment and keep this window open.
						</span>
					</div>
				{/if}
			</div>
		{/if}
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
