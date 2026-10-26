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
	class="inline-flex items-center gap-0.5 rounded-lg border border-border bg-card p-0.5 shadow-[var(--shadow-quiet)]"
>
	<button
		type="button"
		onclick={() => onDateOffset(-1)}
		disabled={dateLoading || isProcessing}
		class="grid size-8 cursor-pointer place-items-center rounded-md text-muted-foreground transition-colors hover:bg-surface hover:text-foreground active:scale-95 disabled:opacity-40"
		aria-label="Previous day"
	>
		<ChevronLeft class="size-4" />
	</button>

	<button
		type="button"
		onclick={() => (datePickerOpen = true)}
		disabled={dateLoading || isProcessing}
		class="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-md px-2.5 text-[12.5px] font-semibold transition-colors hover:bg-surface disabled:opacity-60"
		aria-haspopup="dialog"
		aria-expanded={datePickerOpen}
	>
		<CalendarDays class="size-3.5 text-muted-foreground" aria-hidden="true" />
		<span class="tabular-nums" class:opacity-55={dateLoading}>{displayDateLabel}</span>
	</button>

	<button
		type="button"
		onclick={() => onDateOffset(1)}
		disabled={dateLoading || isProcessing}
		class="grid size-8 cursor-pointer place-items-center rounded-md text-muted-foreground transition-colors hover:bg-surface hover:text-foreground active:scale-95 disabled:opacity-40"
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
