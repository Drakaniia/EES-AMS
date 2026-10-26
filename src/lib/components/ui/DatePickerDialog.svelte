<script lang="ts">
	import { ChevronLeft, ChevronRight } from 'lucide-svelte';
	import Dialog from './Dialog.svelte';

	type Props = {
		open: boolean;
		value?: string;
		min?: string;
		max?: string;
		onClose?: () => void;
		onSelect?: (detail: { date: string }) => void;
	};

	let { open, value = '', min, max, onClose, onSelect }: Props = $props();

	// Calendar state
	let currentMonth = $state(new Date());
	let selectedDraft = $state<string | null | undefined>(undefined);
	let selectedDate = $derived(selectedDraft !== undefined ? selectedDraft : value || null);

	$effect(() => {
		if (open) {
			selectedDraft = undefined;
			currentMonth = value ? parseDate(value) : new Date();
		}
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
		selectedDraft = formatDate(date);
	}

	function handleConfirm() {
		if (selectedDate) {
			onSelect?.({ date: selectedDate });
		}
		onClose?.();
	}

	function handleClear() {
		selectedDraft = null;
		onSelect?.({ date: '' });
		onClose?.();
	}

	function handleToday() {
		const today = new Date();
		if (!isDisabled(today)) {
			selectedDraft = formatDate(today);
			currentMonth = new Date(today.getFullYear(), today.getMonth(), 1);
		}
	}

	function navigateMonth(direction: number) {
		currentMonth = new Date(currentMonth.getFullYear(), currentMonth.getMonth() + direction, 1);
	}

	// Generate calendar days, week starting Monday.
	const calendarDays = $derived.by(() => {
		const daysInMonth = getDaysInMonth(currentMonth);
		const firstDay = getFirstDayOfMonth(currentMonth);
		const days: (Date | null)[] = [];
		const leading = (firstDay + 6) % 7;

		for (let i = 0; i < leading; i++) {
			days.push(null);
		}

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

	const weekDays = ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'];
</script>

<Dialog {open} title="Select date" {onClose}>
	<div class="mb-3 flex items-center justify-between">
		<button
			type="button"
			onclick={() => navigateMonth(-1)}
			class="icon-btn"
			aria-label="Previous month"
		>
			<ChevronLeft class="size-4" aria-hidden="true" />
		</button>

		<div class="text-center">
			<div class="text-[14px] font-semibold tracking-[-0.01em]">
				{monthNames[currentMonth.getMonth()]}
			</div>
			<div class="text-[11px] font-medium text-muted-foreground">{currentMonth.getFullYear()}</div>
		</div>

		<button type="button" onclick={() => navigateMonth(1)} class="icon-btn" aria-label="Next month">
			<ChevronRight class="size-4" aria-hidden="true" />
		</button>
	</div>

	<div class="mb-4 grid grid-cols-7 gap-y-1">
		{#each weekDays as day, i (`header-${i}`)}
			<div class="pb-1 text-center text-[10.5px] font-semibold text-muted-foreground">{day}</div>
		{/each}

		{#each calendarDays as day, i (`day-${i}`)}
			{#if day === null}
				<div></div>
			{:else}
				{@const isDateDisabled = isDisabled(day as Date)}
				{@const isDateSelected = isSelected(day as Date)}
				{@const isDateToday = isToday(day as Date)}
				<button
					type="button"
					onclick={() => handleDateSelect(day as Date)}
					disabled={isDateDisabled}
					class="relative mx-auto grid size-8 place-items-center rounded-md text-[12.5px] font-medium transition-colors
						{isDateDisabled ? 'cursor-not-allowed text-muted-foreground/35' : ''}
						{!isDateDisabled && !isDateSelected ? 'cursor-pointer hover:bg-surface' : ''}
						{isDateSelected ? 'cursor-default bg-primary font-semibold text-primary-foreground' : ''}
						{isDateToday && !isDateSelected ? 'ring-1 ring-border' : ''}"
					aria-pressed={isDateSelected}
				>
					{(day as Date).getDate()}
				</button>
			{/if}
		{/each}
	</div>

	<div class="flex items-center justify-between gap-2 border-t border-border pt-3">
		<div class="flex gap-1.5">
			<button type="button" onclick={handleClear} class="btn btn-secondary btn-sm">Clear</button>
			<button type="button" onclick={handleToday} class="btn btn-secondary btn-sm">Today</button>
		</div>
		<button type="button" onclick={handleConfirm} class="btn btn-primary btn-sm">Select</button>
	</div>
</Dialog>
