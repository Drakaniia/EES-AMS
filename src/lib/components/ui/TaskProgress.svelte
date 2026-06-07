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
		class="rounded-xl border border-primary/25 bg-primary/10 p-4"
		role="status"
		aria-live="polite"
	>
		<div class="flex items-start gap-3">
			<span
				class="mt-0.5 grid size-9 shrink-0 place-items-center rounded-lg border border-primary/30 bg-background text-primary"
				aria-hidden="true"
			>
				{#if simple}
					<span class="size-2 rounded-full bg-primary"></span>
				{:else}
					<span class="size-4 rounded-full border-2 border-primary/25 bg-primary"></span>
				{/if}
			</span>

			<div class="min-w-0 flex-1">
				<div class="flex items-center justify-between gap-3">
					<div class="text-sm font-semibold text-foreground">{title}</div>
					<div class="label-mono text-[10px] text-primary">
						{hasMeasuredProgress ? `${Math.round(progressPercent)}%` : 'In progress'}
					</div>
				</div>

				{#if description}
					<p class="mt-1 text-xs leading-5 text-muted-foreground">{description}</p>
				{/if}

				{#if hasMeasuredProgress}
					<div
						class="mt-3 h-2 overflow-hidden rounded-pill border border-primary/20 bg-background"
						role="progressbar"
						aria-valuemin="0"
						aria-valuemax={max}
						aria-valuenow={Number(value)}
						aria-valuetext={`${Math.round(progressPercent)} percent`}
					>
						<div class="h-full rounded-pill bg-primary" style={`width: ${progressPercent}%`}></div>
					</div>
				{:else if !simple}
					<div
						class="mt-3 h-2 rounded-pill border border-primary/20 bg-background"
						role="progressbar"
						aria-valuetext={title}
					>
						<div class="h-full w-2/5 rounded-pill bg-primary/70"></div>
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}
