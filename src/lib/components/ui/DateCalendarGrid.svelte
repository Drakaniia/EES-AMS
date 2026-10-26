<script lang="ts">
	/**
	 * Shared calendar grid used by DateRangePicker.
	 * Renders the month navigation + day grid.
	 * The parent handles view-specific header text and action buttons.
	 */
	import { ChevronLeft, ChevronRight } from 'lucide-svelte';

	type Props = {
		month: Date;
		selectedDate: Date | null;
		min?: string;
		max?: string;
		view: 'from' | 'to';
		fromDate?: string | null;
		onSelect?: (date: Date) => void;
		onNavigate?: (direction: number) => void;
	};

	let {
		month,
		selectedDate,
		min,
		max,
		view = 'from',
		fromDate,
		onSelect,
		onNavigate
	}: Props = $props();

	function getDaysInMonth(date: Date): number {
		return new Date(date.getFullYear(), date.getMonth() + 1, 0).getDate();
	}

	function getFirstDayOfMonth(date: Date): number {
		return new Date(date.getFullYear(), date.getMonth(), 1).getDay();
	}

	function fmtDate(date: Date): string {
		const year = date.getFullYear();
		const month = String(date.getMonth() + 1).padStart(2, '0');
		const day = String(date.getDate()).padStart(2, '0');
		return `${year}-${month}-${day}`;
	}

	function isToday(date: Date): boolean {
		const today = new Date();
		return (
			date.getDate() === today.getDate() &&
			date.getMonth() === today.getMonth() &&
			date.getFullYear() === today.getFullYear()
		);
	}

	function isSelected(date: Date): boolean {
		if (!selectedDate) return false;
		return (
			date.getDate() === selectedDate.getDate() &&
			date.getMonth() === selectedDate.getMonth() &&
			date.getFullYear() === selectedDate.getFullYear()
		);
	}

	function isDisabled(date: Date): boolean {
		const dateStr = fmtDate(date);
		if (min && dateStr < min) return true;
		if (max && dateStr > max) return true;
		if (view === 'to' && fromDate && dateStr < fromDate) return true;
		return false;
	}

	function handleDateSelect(date: Date) {
		if (isDisabled(date)) return;
		onSelect?.(date);
	}

	const calendarDays = $derived.by(() => {
		const daysInMonth = getDaysInMonth(month);
		const firstDay = getFirstDayOfMonth(month);
		const days: (Date | null)[] = [];
		// Start the week on Monday, the way a school week reads.
		const leading = (firstDay + 6) % 7;
		for (let i = 0; i < leading; i++) days.push(null);
		for (let i = 1; i <= daysInMonth; i++) {
			days.push(new Date(month.getFullYear(), month.getMonth(), i));
		}
		return days;
	});

	const monthNames = [
		'January',
		'February',
		'March',
		'April',
		'May',
		'June',
		'July',
		'August',
		'September',
		'October',
		'November',
		'December'
	] as const;

	const weekDays = ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'] as const;
</script>

<div class="space-y-3">
	<div class="flex items-center justify-between">
		<button onclick={() => onNavigate?.(-1)} class="icon-btn" aria-label="Previous month">
			<ChevronLeft class="size-4" aria-hidden="true" />
		</button>

		<div class="text-center">
			<div class="text-[14px] font-semibold tracking-[-0.01em]">
				{monthNames[month.getMonth()]}
			</div>
			<div class="text-[11px] font-medium text-muted-foreground">{month.getFullYear()}</div>
		</div>

		<button onclick={() => onNavigate?.(1)} class="icon-btn" aria-label="Next month">
			<ChevronRight class="size-4" aria-hidden="true" />
		</button>
	</div>

	<div class="grid grid-cols-7 gap-y-1">
		{#each weekDays as day, i (`header-${i}`)}
			<div class="pb-1 text-center text-[10.5px] font-semibold text-muted-foreground">{day}</div>
		{/each}

		{#each calendarDays as day, i (`day-${i}`)}
			{#if day === null}
				<div></div>
			{:else}
				{@const dateDisabled = isDisabled(day)}
				{@const dateSelected = isSelected(day)}
				{@const dateToday = isToday(day)}
				<button
					onclick={() => handleDateSelect(day)}
					disabled={dateDisabled}
					class="calendar-day relative mx-auto grid size-8 place-items-center rounded-md text-[12.5px] font-medium transition-colors
						{dateDisabled ? 'cursor-not-allowed text-muted-foreground/35' : ''}
						{!dateDisabled && !dateSelected ? 'hover:bg-surface' : ''}
						{dateSelected ? 'bg-primary font-semibold text-primary-foreground' : ''}
						{dateToday && !dateSelected ? 'ring-1 ring-border' : ''}"
					aria-pressed={dateSelected}
				>
					{day.getDate()}
				</button>
			{/if}
		{/each}
	</div>
</div>
