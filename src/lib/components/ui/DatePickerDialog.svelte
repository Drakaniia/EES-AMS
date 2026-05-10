<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Dialog from './Dialog.svelte';

	type Props = {
		open: boolean;
		value?: string;
		min?: string;
		max?: string;
	};

	let { open, value = '', min, max }: Props = $props();

	const dispatch = createEventDispatcher();

	// Calendar state
	let currentMonth = $state(new Date());
	let selectedDate = $state<string | null>(null);

	// Sync selectedDate with value prop
	$effect(() => {
		selectedDate = value || null;
	});

	// Helper functions
	function getDaysInMonth(date: Date): number {
		return new Date(date.getFullYear(), date.getMonth() + 1, 0).getDate();
	}

	function getFirstDayOfMonth(date: Date): number {
		return new Date(date.getFullYear(), date.getMonth(), 1).getDay();
	}

	function formatDate(date: Date): string {
		const year = date.getFullYear();
		const month = String(date.getMonth() + 1).padStart(2, '0');
		const day = String(date.getDate()).padStart(2, '0');
		return `${year}-${month}-${day}`;
	}

	function parseDate(dateString: string): Date {
		const [year, month, day] = dateString.split('-').map(Number);
		return new Date(year, month - 1, day);
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
		const selected = parseDate(selectedDate);
		return (
			date.getDate() === selected.getDate() &&
			date.getMonth() === selected.getMonth() &&
			date.getFullYear() === selected.getFullYear()
		);
	}

	function isDisabled(date: Date): boolean {
		const dateStr = formatDate(date);
		if (min && dateStr < min) return true;
		if (max && dateStr > max) return true;
		return false;
	}

	function handleDateSelect(date: Date) {
		if (isDisabled(date)) return;
		selectedDate = formatDate(date);
	}

	function handleConfirm() {
		if (selectedDate) {
			dispatch('select', { date: selectedDate });
		}
		dispatch('close');
	}

	function handleClear() {
		selectedDate = null;
		dispatch('select', { date: '' });
		dispatch('close');
	}

	function handleToday() {
		const today = new Date();
		if (!isDisabled(today)) {
			selectedDate = formatDate(today);
		}
	}

	function navigateMonth(direction: number) {
		currentMonth = new Date(currentMonth.getFullYear(), currentMonth.getMonth() + direction, 1);
	}

	// Generate calendar days
	const calendarDays = $derived.by(() => {
		const daysInMonth = getDaysInMonth(currentMonth);
		const firstDay = getFirstDayOfMonth(currentMonth);
		const days: (Date | null)[] = [];

		// Empty cells for days before month starts
		for (let i = 0; i < firstDay; i++) {
			days.push(null);
		}

		// Days of the month
		for (let i = 1; i <= daysInMonth; i++) {
			days.push(new Date(currentMonth.getFullYear(), currentMonth.getMonth(), i));
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
	];

	const weekDays = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'];
</script>

<Dialog {open} title="Select Date" on:close={() => dispatch('close')}>
	<!-- Month navigation -->
	<div class="mb-4 flex items-center justify-between">
		<button
			onclick={() => navigateMonth(-1)}
			class="text-muted-foreground hover:text-foreground p-1 transition-colors"
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
			<div class="font-medium">{monthNames[currentMonth.getMonth()]}</div>
			<div class="text-muted-foreground text-sm">{currentMonth.getFullYear()}</div>
		</div>

		<button
			onclick={() => navigateMonth(1)}
			class="text-muted-foreground hover:text-foreground p-1 transition-colors"
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
	<div class="mb-4 grid grid-cols-7 gap-1">
		<!-- Week day headers -->
		{#each weekDays as day, i (`header-${i}`)}
			<div class="text-muted-foreground py-2 text-center font-mono text-xs">{day}</div>
		{/each}

		<!-- Calendar days -->
		{#each calendarDays as day, i (`day-${i}`)}
			{#if day === null}
				<div class="p-2"></div>
			{:else}
				{@const isDateDisabled = isDisabled(day as Date)}
				{@const isDateSelected = isSelected(day as Date)}
				{@const isDateToday = isToday(day as Date)}
				<button
					onclick={() => handleDateSelect(day as Date)}
					disabled={isDateDisabled}
					class="relative rounded-md p-2 text-sm transition-colors
						{isDateDisabled ? 'text-muted-foreground/30 cursor-not-allowed' : 'hover:bg-surface cursor-pointer'}
						{isDateSelected ? 'bg-primary text-primary-foreground hover:bg-primary' : ''}
						{isDateToday && !isDateSelected ? 'border-border border' : ''}"
				>
					{(day as Date).getDate()}
				</button>
			{/if}
		{/each}
	</div>

	<!-- Action buttons -->
	<div class="border-border flex justify-between gap-2 border-t pt-2">
		<div class="flex gap-2">
			<button
				onclick={handleClear}
				class="border-border hover:bg-surface rounded-md border px-3 py-1.5 text-sm transition-colors"
			>
				Clear
			</button>
			<button
				onclick={handleToday}
				class="border-border hover:bg-surface rounded-md border px-3 py-1.5 text-sm transition-colors"
			>
				Today
			</button>
		</div>
		<button
			onclick={handleConfirm}
			class="rounded-pill bg-primary text-primary-foreground hover:bg-accent px-4 py-1.5 text-sm font-medium transition-colors"
		>
			Select
		</button>
	</div>
</Dialog>
