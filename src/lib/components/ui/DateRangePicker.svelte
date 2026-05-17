<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import Dialog from './Dialog.svelte';

	type Props = {
		open: boolean;
		fromValue?: string;
		toValue?: string;
		min?: string;
		max?: string;
	};

	let { open, fromValue = '', toValue = '', min, max }: Props = $props();

	const dispatch = createEventDispatcher();

	// View state: 'from' | 'to'
	let currentView = $state<'from' | 'to'>('from');

	// Calendar state for FROM view
	let fromMonth = $state(new Date());
	let selectedFromDate = $state<string | null>(null);

	// Calendar state for TO view
	let toMonth = $state(new Date());
	let selectedToDate = $state<string | null>(null);

	// Sync selected dates with props
	$effect(() => {
		selectedFromDate = fromValue || null;
		selectedToDate = toValue || null;
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

	function isSelected(date: Date, view: 'from' | 'to'): boolean {
		const selected = view === 'from' ? selectedFromDate : selectedToDate;
		if (!selected) return false;
		const selectedDate = parseDate(selected);
		return (
			date.getDate() === selectedDate.getDate() &&
			date.getMonth() === selectedDate.getMonth() &&
			date.getFullYear() === selectedDate.getFullYear()
		);
	}

	function isDisabled(date: Date, view: 'from' | 'to'): boolean {
		const dateStr = formatDate(date);
		if (min && dateStr < min) return true;
		if (max && dateStr > max) return true;

		// For TO view, dates before FROM date should be disabled
		if (view === 'to' && selectedFromDate && dateStr < selectedFromDate) {
			return true;
		}

		return false;
	}

	function handleDateSelect(date: Date, view: 'from' | 'to') {
		if (isDisabled(date, view)) return;

		if (view === 'from') {
			selectedFromDate = formatDate(date);
		} else {
			selectedToDate = formatDate(date);
		}
	}

	function handleFromConfirm() {
		if (selectedFromDate) {
			// Transition to TO view
			currentView = 'to';
			// Set TO month to FROM month for better UX
			toMonth = new Date(fromMonth);
		}
	}

	function handleToConfirm() {
		if (selectedToDate) {
			dispatch('select', { from: selectedFromDate, to: selectedToDate });
		}
		dispatch('close');
	}

	function handleClear() {
		selectedFromDate = null;
		selectedToDate = null;
		dispatch('select', { from: '', to: '' });
		dispatch('close');
	}

	function handleToday(view: 'from' | 'to') {
		const today = new Date();
		if (!isDisabled(today, view)) {
			if (view === 'from') {
				selectedFromDate = formatDate(today);
			} else {
				selectedToDate = formatDate(today);
			}
		}
	}

	function handlePrevious() {
		currentView = 'from';
	}

	function navigateMonth(direction: number, view: 'from' | 'to') {
		if (view === 'from') {
			fromMonth = new Date(fromMonth.getFullYear(), fromMonth.getMonth() + direction, 1);
		} else {
			toMonth = new Date(toMonth.getFullYear(), toMonth.getMonth() + direction, 1);
		}
	}

	// Generate calendar days
	function getCalendarDays(month: Date) {
		const daysInMonth = getDaysInMonth(month);
		const firstDay = getFirstDayOfMonth(month);
		const days: (Date | null)[] = [];

		// Empty cells for days before month starts
		for (let i = 0; i < firstDay; i++) {
			days.push(null);
		}

		// Days of the month
		for (let i = 1; i <= daysInMonth; i++) {
			days.push(new Date(month.getFullYear(), month.getMonth(), i));
		}

		return days;
	}

	const fromCalendarDays = $derived.by(() => getCalendarDays(fromMonth));
	const toCalendarDays = $derived.by(() => getCalendarDays(toMonth));

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

<Dialog {open} title="Select Date Range" on:close={() => dispatch('close')}>
	<div class="relative overflow-hidden">
		<!-- FROM View -->
		<div
			class="transition-all duration-300 ease-in-out"
			style="transform: translateX({currentView === 'from'
				? '0'
				: '-100'}%); opacity: {currentView === 'from' ? '1' : '0'}; position: {currentView ===
			'from'
				? 'relative'
				: 'absolute'}; inset: 0;"
		>
			<div class="space-y-4">
				<div class="text-center">
					<div class="text-sm font-medium text-muted-foreground">Select FROM date</div>
				</div>

				<!-- Month navigation for FROM -->
				<div class="flex items-center justify-between">
					<button
						onclick={() => navigateMonth(-1, 'from')}
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
						<div class="font-medium">{monthNames[fromMonth.getMonth()]}</div>
						<div class="text-sm text-muted-foreground">{fromMonth.getFullYear()}</div>
					</div>

					<button
						onclick={() => navigateMonth(1, 'from')}
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

				<!-- Calendar grid for FROM -->
				<div class="grid grid-cols-7 gap-1">
					<!-- Week day headers -->
					{#each weekDays as day, i (`from-header-${i}`)}
						<div class="py-2 text-center font-mono text-xs text-muted-foreground">{day}</div>
					{/each}

					<!-- Calendar days -->
					{#each fromCalendarDays as day, i (`from-day-${i}`)}
						{#if day === null}
							<div class="p-2"></div>
						{:else}
							{@const isDateDisabled = isDisabled(day as Date, 'from')}
							{@const isDateSelected = isSelected(day as Date, 'from')}
							{@const isDateToday = isToday(day as Date)}
							<button
								onclick={() => handleDateSelect(day as Date, 'from')}
								disabled={isDateDisabled}
								class="relative rounded-md p-2 text-sm transition-colors
									{isDateDisabled ? 'cursor-not-allowed text-muted-foreground/30' : ''}
									{!isDateDisabled && !isDateSelected ? 'cursor-pointer hover:bg-surface' : ''}
									{isDateSelected ? 'cursor-default bg-primary text-primary-foreground' : ''}
									{isDateToday && !isDateSelected ? 'border border-border' : ''}"
							>
								{(day as Date).getDate()}
							</button>
						{/if}
					{/each}
				</div>

				<!-- Action buttons for FROM -->
				<div class="flex justify-between gap-2 border-t border-border pt-2">
					<div class="flex gap-2">
						<button
							onclick={handleClear}
							class="rounded-md border border-border px-3 py-1.5 text-sm transition-colors hover:bg-surface"
						>
							Clear
						</button>
						<button
							onclick={() => handleToday('from')}
							class="rounded-md border border-border px-3 py-1.5 text-sm transition-colors hover:bg-surface"
						>
							Today
						</button>
					</div>
					<button
						onclick={handleFromConfirm}
						disabled={!selectedFromDate}
						class="rounded-pill bg-primary px-4 py-1.5 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
					>
						Select
					</button>
				</div>
			</div>
		</div>

		<!-- TO View -->
		<div
			class="transition-all duration-300 ease-in-out"
			style="transform: translateX({currentView === 'to' ? '0' : '100'}%); opacity: {currentView ===
			'to'
				? '1'
				: '0'}; position: {currentView === 'to' ? 'relative' : 'absolute'}; inset: 0;"
		>
			<div class="space-y-4">
				<div class="text-center">
					<div class="text-sm font-medium text-muted-foreground">Select TO date</div>
					{#if selectedFromDate}
						<div class="text-xs text-foreground">
							From: {new Date(selectedFromDate).toLocaleDateString()}
						</div>
					{/if}
				</div>

				<!-- Month navigation for TO -->
				<div class="flex items-center justify-between">
					<button
						onclick={() => navigateMonth(-1, 'to')}
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
						<div class="font-medium">{monthNames[toMonth.getMonth()]}</div>
						<div class="text-sm text-muted-foreground">{toMonth.getFullYear()}</div>
					</div>

					<button
						onclick={() => navigateMonth(1, 'to')}
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

				<!-- Calendar grid for TO -->
				<div class="grid grid-cols-7 gap-1">
					<!-- Week day headers -->
					{#each weekDays as day, i (`to-header-${i}`)}
						<div class="py-2 text-center font-mono text-xs text-muted-foreground">{day}</div>
					{/each}

					<!-- Calendar days -->
					{#each toCalendarDays as day, i (`to-day-${i}`)}
						{#if day === null}
							<div class="p-2"></div>
						{:else}
							{@const isDateDisabled = isDisabled(day as Date, 'to')}
							{@const isDateSelected = isSelected(day as Date, 'to')}
							{@const isDateToday = isToday(day as Date)}
							<button
								onclick={() => handleDateSelect(day as Date, 'to')}
								disabled={isDateDisabled}
								class="relative rounded-md p-2 text-sm transition-colors
									{isDateDisabled ? 'cursor-not-allowed text-muted-foreground/30' : ''}
									{!isDateDisabled && !isDateSelected ? 'cursor-pointer hover:bg-surface' : ''}
									{isDateSelected ? 'cursor-default bg-primary text-primary-foreground' : ''}
									{isDateToday && !isDateSelected ? 'border border-border' : ''}"
							>
								{(day as Date).getDate()}
							</button>
						{/if}
					{/each}
				</div>

				<!-- Action buttons for TO -->
				<div class="flex justify-between gap-2 border-t border-border pt-2">
					<div class="flex gap-2">
						<button
							onclick={handleClear}
							class="rounded-md border border-border px-3 py-1.5 text-sm transition-colors hover:bg-surface"
						>
							Clear
						</button>
						<button
							onclick={() => handleToday('to')}
							class="rounded-md border border-border px-3 py-1.5 text-sm transition-colors hover:bg-surface"
						>
							Today
						</button>
					</div>
					<div class="flex gap-2">
						<button
							onclick={handlePrevious}
							class="rounded-md border border-border px-3 py-1.5 text-sm transition-colors hover:bg-surface"
						>
							Previous
						</button>
						<button
							onclick={handleToConfirm}
							disabled={!selectedToDate}
							class="rounded-pill bg-primary px-4 py-1.5 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
						>
							Select
						</button>
					</div>
				</div>
			</div>
		</div>
	</div>
</Dialog>
