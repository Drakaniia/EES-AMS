<script lang="ts">
	import { CalendarDays, ChevronLeft, ChevronRight } from 'lucide-svelte';
	import DatePickerDialog from '$lib/components/ui/DatePickerDialog.svelte';

	let {
		selectedDate,
		dateLoading,
		isProcessing,
		displayDateLabel,
		onDateOffset,
		onDateSelect
	}: {
		selectedDate: string;
		dateLoading: boolean;
		isProcessing: boolean;
		displayDateLabel: string;
		onDateOffset: (offset: number) => void;
		onDateSelect: (date: string) => void;
	} = $props();

	let datePickerOpen = $state(false);
</script>

<div
	class="inline-flex items-center rounded-pill border border-border bg-background p-0.5 shadow-sm"
>
	<button
		type="button"
		onclick={() => onDateOffset(-1)}
		disabled={dateLoading || isProcessing}
		class="flex size-9 cursor-pointer items-center justify-center rounded-pill text-muted-foreground transition-colors hover:bg-surface hover:text-foreground disabled:opacity-40"
		aria-label="Previous day"
	>
		<ChevronLeft class="size-4" />
	</button>

	<button
		type="button"
		onclick={() => (datePickerOpen = true)}
		disabled={dateLoading || isProcessing}
		class="inline-flex h-9 cursor-pointer items-center gap-2 rounded-pill px-3 text-sm font-semibold transition-colors hover:bg-surface disabled:opacity-60"
		aria-haspopup="dialog"
		aria-expanded={datePickerOpen}
	>
		{#if dateLoading}
			<span class="size-2 animate-pulse rounded-full bg-primary" aria-hidden="true"></span>
		{:else}
			<CalendarDays class="size-4 text-primary" aria-hidden="true" />
		{/if}
		<span class="font-mono text-xs md:text-sm">{displayDateLabel}</span>
	</button>

	<button
		type="button"
		onclick={() => onDateOffset(1)}
		disabled={dateLoading || isProcessing}
		class="flex size-9 cursor-pointer items-center justify-center rounded-pill text-muted-foreground transition-colors hover:bg-surface hover:text-foreground disabled:opacity-40"
		aria-label="Next day"
	>
		<ChevronRight class="size-4" />
	</button>
</div>

<DatePickerDialog
	open={datePickerOpen}
	value={selectedDate}
	onClose={() => (datePickerOpen = false)}
	onSelect={({ date }) => {
		onDateSelect(date);
	}}
/>
