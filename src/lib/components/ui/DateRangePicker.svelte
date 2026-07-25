<script lang="ts">
	import Dialog from './Dialog.svelte';
	import DateCalendarGrid from './DateCalendarGrid.svelte';

	type Props = {
		open: boolean;
		fromValue?: string;
		toValue?: string;
		min?: string;
		max?: string;
		onClose?: () => void;
		onSelect?: (detail: { from: string; to: string }) => void;
	};

	let { open, fromValue = '', toValue = '', min, max, onClose, onSelect }: Props = $props();

	// View state: 'from' | 'to'
	let currentView = $state<'from' | 'to'>('from');

	// Calendar state for FROM view
	let fromMonth = $state(new Date());
	let selectedFromDraft = $state<string | null | undefined>(undefined);
	let selectedFromDate = $derived(
		selectedFromDraft !== undefined ? selectedFromDraft : fromValue || null
	);

	// Calendar state for TO view
	let toMonth = $state(new Date());
	let selectedToDraft = $state<string | null | undefined>(undefined);
	let selectedToDate = $derived(selectedToDraft !== undefined ? selectedToDraft : toValue || null);

	function parseDate(dateString: string): Date {
		const [year, month, day] = dateString.split('-').map(Number);
		return new Date(year, month - 1, day);
	}

	function formatDate(date: Date): string {
		const year = date.getFullYear();
		const month = String(date.getMonth() + 1).padStart(2, '0');
		const day = String(date.getDate()).padStart(2, '0');
		return `${year}-${month}-${day}`;
	}

	function handleFromSelect(date: Date) {
		selectedFromDraft = formatDate(date);
	}

	function handleToSelect(date: Date) {
		selectedToDraft = formatDate(date);
	}

	function handleFromConfirm() {
		if (selectedFromDate) {
			currentView = 'to';
			toMonth = new Date(fromMonth);
		}
	}

	function handleToConfirm() {
		if (selectedFromDate && selectedToDate) {
			onSelect?.({ from: selectedFromDate, to: selectedToDate });
		}
		onClose?.();
	}

	function handleClear() {
		selectedFromDraft = null;
		selectedToDraft = null;
		onSelect?.({ from: '', to: '' });
		onClose?.();
	}

	function handleToday(view: 'from' | 'to') {
		const today = new Date();
		if (view === 'from') {
			selectedFromDraft = formatDate(today);
		} else {
			selectedToDraft = formatDate(today);
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
</script>

<Dialog {open} title="Select Date Range" {onClose}>
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

				<DateCalendarGrid
					month={fromMonth}
					selectedDate={selectedFromDate ? parseDate(selectedFromDate) : null}
					{min}
					{max}
					view="from"
					onSelect={handleFromSelect}
					onNavigate={(d) => navigateMonth(d, 'from')}
				/>

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

				<DateCalendarGrid
					month={toMonth}
					selectedDate={selectedToDate ? parseDate(selectedToDate) : null}
					{min}
					{max}
					view="to"
					fromDate={selectedFromDate}
					onSelect={handleToSelect}
					onNavigate={(d) => navigateMonth(d, 'to')}
				/>

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
