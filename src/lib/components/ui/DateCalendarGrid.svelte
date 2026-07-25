<script lang="ts">
	/**
	 * Shared calendar grid used by DateRangePicker.
	 * Renders the month navigation + day grid.
	 * The parent handles view-specific header text and action buttons.
	 */
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
		for (let i = 0; i < firstDay; i++) days.push(null);
		for (let i = 1; i <= daysInMonth; i++) {
			days.push(new Date(month.getFullYear(), month.getMonth(), i));
		}
		return days;
	});

	const monthNames = [
		'January', 'February', 'March', 'April', 'May', 'June',
		'July', 'August', 'September', 'October', 'November', 'December'
	] as const;

	const weekDays = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'] as const;
</script>

<div class="space-y-4">
	<!-- Month navigation -->
	<div class="flex items-center justify-between">
		<button
			onclick={() => onNavigate?.(-1)}
			class="p-1 text-muted-foreground transition-colors hover:text-foreground"
			aria-label="Previous month"
		>
			<svg
				class="size-5"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<polyline points="15 18 9 12 15 6"></polyline>
			</svg>
		</button>

		<div class="text-center">
			<div class="font-medium">{monthNames[month.getMonth()]}</div>
			<div class="text-sm text-muted-foreground">{month.getFullYear()}</div>
		</div>

		<button
			onclick={() => onNavigate?.(1)}
			class="p-1 text-muted-foreground transition-colors hover:text-foreground"
			aria-label="Next month"
		>
			<svg
				class="size-5"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<polyline points="9 18 15 12 9 6"></polyline>
			</svg>
		</button>
	</div>

	<!-- Calendar grid -->
	<div class="grid grid-cols-7 gap-1">
		{#each weekDays as day, i (`header-${i}`)}
			<div class="py-2 text-center font-mono text-xs text-muted-foreground">{day}</div>
		{/each}

		{#each calendarDays as day, i (`day-${i}`)}
			{#if day === null}
				<div class="p-2"></div>
			{:else}
				{@const dateDisabled = isDisabled(day)}
				{@const dateSelected = isSelected(day)}
				{@const dateToday = isToday(day)}
				<button
					onclick={() => handleDateSelect(day)}
					disabled={dateDisabled}
					class="relative rounded-md p-2 text-sm transition-colors
						{dateDisabled ? 'cursor-not-allowed text-muted-foreground/30' : ''}
						{!dateDisabled && !dateSelected ? 'cursor-pointer hover:bg-surface' : ''}
						{dateSelected ? 'cursor-default bg-primary text-primary-foreground' : ''}
						{dateToday && !dateSelected ? 'border border-border' : ''}"
				>
					{day.getDate()}
				</button>
			{/if}
		{/each}
	</div>
</div>
