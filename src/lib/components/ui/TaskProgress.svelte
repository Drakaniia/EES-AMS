<script lang="ts">
	type Props = {
		active?: boolean;
		title: string;
		description?: string;
		simple?: boolean;
		value?: number | null;
		max?: number;
	};

	let {
		active = false,
		title,
		description,
		simple = false,
		value = null,
		max = 100
	}: Props = $props();

	const hasMeasuredProgress = $derived(value !== null && Number.isFinite(value));
	const progressPercent = $derived.by(() => {
		if (!hasMeasuredProgress) return 0;
		const safeMax = max > 0 ? max : 100;
		return Math.min(100, Math.max(0, (Number(value) / safeMax) * 100));
	});
</script>

{#if active}
	<div
		class="rounded-xl border border-border bg-card p-4 shadow-[var(--shadow-quiet)]"
		role="status"
		aria-live="polite"
	>
		<div class="flex items-start gap-3">
			<span
				class="mt-1.5 size-2 shrink-0 rounded-full bg-primary"
				class:animate-pulse={!hasMeasuredProgress}
				aria-hidden="true"
			></span>

			<div class="min-w-0 flex-1">
				<div class="flex items-baseline justify-between gap-3">
					<div class="text-[13px] font-semibold text-foreground">{title}</div>
					<div class="text-[11px] font-medium text-muted-foreground tabular-nums">
						{hasMeasuredProgress ? `${Math.round(progressPercent)}%` : 'Working…'}
					</div>
				</div>

				{#if description}
					<p class="mt-1 text-[12px] leading-5 text-muted-foreground">{description}</p>
				{/if}

				{#if hasMeasuredProgress}
					<div
						class="progress-track mt-3"
						role="progressbar"
						aria-valuemin="0"
						aria-valuemax={max}
						aria-valuenow={Number(value)}
						aria-valuetext={`${Math.round(progressPercent)} percent`}
					>
						<div class="progress-fill" style={`width: ${progressPercent}%`}></div>
					</div>
				{:else if !simple}
					<div class="progress-track mt-3" role="progressbar" aria-valuetext={title}>
						<div class="progress-fill animate-pulse" style="width: 38%"></div>
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}
