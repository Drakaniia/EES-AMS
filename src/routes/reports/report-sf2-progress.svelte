<script lang="ts">
	import { CheckCircle2, CircleX } from 'lucide-svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';

	type Sf2OpenStatus = 'idle' | 'syncing' | 'success' | 'error';

	type Props = {
		status: Sf2OpenStatus;
		error: string | null;
		resultPath: string | null;
		displayMessage: string;
		progressPercent: number;
		showWaitHint: boolean;
		onRetry?: () => void;
		onClose?: () => void;
	};

	let {
		status,
		error,
		resultPath,
		displayMessage,
		progressPercent,
		showWaitHint,
		onRetry,
		onClose
	}: Props = $props();
</script>

{#if status !== 'idle'}
	<div
		role="dialog"
		aria-modal="true"
		aria-label={status === 'error' ? 'Opening failed' : 'Opening SF2 workbook'}
		class="fixed inset-x-0 top-[46px] bottom-0 z-[70] flex items-center justify-center p-4"
		tabindex="-1"
		onkeydown={(e) => {
			if (e.key === 'Escape' && status === 'error') {
				onClose?.();
			}
		}}
	>
		<div class="sf2-scrim absolute inset-0" aria-hidden="true"></div>

		<div
			class="relative flex w-full max-w-[340px] flex-col gap-4 rounded-2xl border border-border bg-card p-6 text-center shadow-[var(--shadow-lifted)]"
			role={status === 'syncing' ? 'status' : undefined}
			aria-live={status === 'syncing' ? 'polite' : undefined}
		>
			{#if status === 'error'}
				<div
					class="mx-auto grid size-9 place-items-center rounded-full bg-destructive/10 text-destructive"
				>
					<CircleX class="size-[18px]" aria-hidden="true" strokeWidth={1.9} />
				</div>
				<div class="space-y-1">
					<h3 class="text-[14px] font-semibold">Couldn’t open the workbook</h3>
					<p class="text-[12px] leading-5 text-muted-foreground">{error}</p>
				</div>
				<div class="flex justify-center gap-2">
					<button type="button" onclick={onClose} class="btn btn-secondary">Close</button>
					<button type="button" onclick={onRetry} class="btn btn-primary">Try again</button>
				</div>
			{:else if status === 'success'}
				<div
					class="mx-auto grid size-9 place-items-center rounded-full bg-emerald-500/10 text-emerald-700 dark:text-emerald-400"
				>
					<CheckCircle2 class="size-[18px]" aria-hidden="true" strokeWidth={1.9} />
				</div>
				<div class="space-y-1">
					<h3 class="text-[14px] font-semibold">Workbook opened</h3>
					{#if resultPath}
						<p class="text-[11px] leading-4 break-all text-muted-foreground">{resultPath}</p>
					{/if}
				</div>
			{:else}
				<div class="flex items-center justify-center gap-2">
					<Spinner class="text-primary" />
					<span class="text-[13px] font-semibold">{displayMessage}</span>
				</div>

				<div class="space-y-1.5">
					<div
						class="progress-track"
						role="progressbar"
						aria-valuemin="0"
						aria-valuemax="100"
						aria-valuenow={progressPercent}
						aria-valuetext={`${progressPercent} percent`}
					>
						<div class="progress-fill" style="width: {progressPercent}%"></div>
					</div>
					<p class="text-[11px] font-medium text-muted-foreground tabular-nums">
						{progressPercent}%
					</p>
				</div>

				{#if progressPercent >= 100}
					<p class="text-[11.5px] text-muted-foreground">Finishing…</p>
				{:else if showWaitHint}
					<p class="rounded-lg bg-surface px-3 py-2 text-[11.5px] leading-5 text-muted-foreground">
						Excel is still writing the workbook. Keep this window open — it usually finishes in a
						few seconds.
					</p>
				{/if}
			{/if}
		</div>
	</div>
{/if}

<style>
	.sf2-scrim {
		background: var(--scrim);
	}
</style>
