<script lang="ts">
	import {
		listEventsForStudent,
		getSettings,
		type Student,
		type AttendanceEvent,
		type Settings
	} from '$lib/db-rust';
	import Dialog from '../ui/Dialog.svelte';

	interface Props {
		student: Student | null;
		open: boolean;
		onClose: () => void;
	}

	let { student, open, onClose }: Props = $props();

	let events = $state<AttendanceEvent[]>([]);
	let settings = $state<Settings | null>(null);
	let loading = $state(false);

	let viewMode = $state<'month' | 'quarter'>('month');
	let now = new Date();
	let selectedMonth = $state(now.getMonth());
	let selectedYear = $state(now.getFullYear());

	const months = [
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

	$effect(() => {
		if (open && student) {
			loadData();
		}
	});

	async function loadData() {
		if (!student) return;
		loading = true;
		try {
			const [evs, sets] = await Promise.all([listEventsForStudent(student.id), getSettings()]);
			events = evs;
			settings = sets;
		} catch (error) {
			console.error('Failed to load attendance data:', error);
		} finally {
			loading = false;
		}
	}

	// Helper to get days in month
	function getDaysInMonth(month: number, year: number) {
		return new Date(year, month + 1, 0).getDate();
	}

	// Helper to get day of week for the 1st of the month
	function getFirstDayOfMonth(month: number, year: number) {
		return new Date(year, month, 1).getDay();
	}

	const calendarDays = $derived.by(() => {
		const days = getDaysInMonth(selectedMonth, selectedYear);
		const firstDay = getFirstDayOfMonth(selectedMonth, selectedYear);
		const result = [];

		// Padding for previous month
		for (let i = 0; i < firstDay; i++) {
			result.push(null);
		}

		// Days of current month
		for (let i = 1; i <= days; i++) {
			const dateStr = `${selectedYear}-${String(selectedMonth + 1).padStart(2, '0')}-${String(i).padStart(2, '0')}`;
			const dayEvents = events.filter((e) => e.timestamp.startsWith(dateStr));
			result.push({
				day: i,
				date: dateStr,
				events: dayEvents
			});
		}

		return result;
	});

	function getStatus(dayEvents: AttendanceEvent[]) {
		if (dayEvents.length === 0) return 'none';
		const hasIn = dayEvents.some((e) => e.type === 'in');
		// Simple logic: if they clocked in, they are present.
		// We could add "Late" logic here if we compare with settings.lateAfter
		return hasIn ? 'present' : 'none';
	}

	function isLate(event: AttendanceEvent) {
		if (event.type !== 'in' || !settings) return false;
		const eventTime = event.timestamp.split('T')[1].substring(0, 5); // HH:mm
		return eventTime > settings.lateAfter;
	}

	const stats = $derived.by(() => {
		const filtered = events.filter((e) => {
			const d = new Date(e.timestamp);
			return d.getMonth() === selectedMonth && d.getFullYear() === selectedYear;
		});

		const presents = new Set(filtered.map((e) => e.timestamp.split('T')[0])).size;
		const tardies = filtered.filter((e) => isLate(e)).length;

		return { presents, tardies };
	});

	function prevMonth() {
		if (selectedMonth === 0) {
			selectedMonth = 11;
			selectedYear--;
		} else {
			selectedMonth--;
		}
	}

	function nextMonth() {
		if (selectedMonth === 11) {
			selectedMonth = 0;
			selectedYear++;
		} else {
			selectedMonth++;
		}
	}
</script>

<Dialog
	{open}
	title="Student Attendance Record"
	description={student?.name ?? ''}
	maxWidth="lg"
	on:close={onClose}
>
	<div class="space-y-6">
		{#if loading}
			<div class="flex h-64 items-center justify-center">
				<div
					class="size-8 animate-spin rounded-full border-2 border-primary border-t-transparent"
				></div>
			</div>
		{:else}
			<!-- Header Stats -->
			<div class="grid grid-cols-3 gap-4">
				<div class="rounded-xl border border-border bg-surface p-4">
					<div class="label-mono text-xs">Total Presents</div>
					<div class="mt-1 text-2xl font-bold text-primary">{stats.presents}</div>
				</div>
				<div class="rounded-xl border border-border bg-surface p-4">
					<div class="label-mono text-xs">Late Arrivals</div>
					<div class="text-warning mt-1 text-2xl font-bold">{stats.tardies}</div>
				</div>
				<div class="rounded-xl border border-border bg-surface p-4">
					<div class="label-mono text-xs">Current Quarter</div>
					<div class="mt-1 text-2xl font-bold">{settings?.quarter || '—'}</div>
				</div>
			</div>

			<!-- Controls -->
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-2">
					<button
						onclick={prevMonth}
						class="rounded-md border border-border p-1.5 transition-colors hover:bg-surface"
						aria-label="Previous month"
					>
						<svg
							class="size-4"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<path d="m15 18-6-6 6-6" />
						</svg>
					</button>
					<span class="min-w-32 text-center font-medium">
						{months[selectedMonth]}
						{selectedYear}
					</span>
					<button
						onclick={nextMonth}
						class="rounded-md border border-border p-1.5 transition-colors hover:bg-surface"
						aria-label="Next month"
					>
						<svg
							class="size-4"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<path d="m9 18 6-6-6-6" />
						</svg>
					</button>
				</div>

				<div class="flex rounded-lg border border-border bg-surface p-1">
					<button
						onclick={() => (viewMode = 'month')}
						class="rounded-md px-3 py-1 text-xs font-medium transition-all {viewMode === 'month'
							? 'bg-background text-primary shadow-sm'
							: 'text-muted-foreground'}"
					>
						Monthly
					</button>
					<button
						onclick={() => (viewMode = 'quarter')}
						class="rounded-md px-3 py-1 text-xs font-medium transition-all {viewMode === 'quarter'
							? 'bg-background text-primary shadow-sm'
							: 'text-muted-foreground'}"
					>
						Quarterly
					</button>
				</div>
			</div>

			{#if viewMode === 'month'}
				<!-- Calendar Grid -->
				<div class="grid grid-cols-7 gap-1">
					{#each ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'] as day (day)}
						<div
							class="py-2 text-center text-[10px] font-bold tracking-wider text-muted-foreground uppercase"
						>
							{day}
						</div>
					{/each}

					{#each calendarDays as d, i (i)}
						{#if d === null}
							<div class="aspect-square"></div>
						{:else}
							{@const status = getStatus(d.events)}
							{@const late = d.events.some((e) => isLate(e))}
							<div
								class="relative flex aspect-square flex-col items-center justify-center rounded-lg border border-border transition-colors
								{status === 'present'
									? late
										? 'bg-warning/10 border-warning/20'
										: 'border-primary/20 bg-primary/10'
									: 'bg-surface'}"
							>
								<span
									class="font-mono text-xs {status === 'present'
										? 'font-bold'
										: 'text-muted-foreground'}"
								>
									{d.day}
								</span>
								{#if status === 'present'}
									<div class="mt-1 size-1 rounded-full {late ? 'bg-warning' : 'bg-primary'}"></div>
								{/if}
							</div>
						{/if}
					{/each}
				</div>
			{:else}
				<!-- Quarterly View (List of events grouped by month in quarter) -->
				<div class="custom-scrollbar max-h-80 space-y-4 overflow-y-auto pr-2">
					{#each events.filter((e) => {
						const d = new Date(e.timestamp);
						// Rough quarter logic if settings doesn't have explicit dates
						const q = Math.floor(d.getMonth() / 3) + 1;
						return q.toString() === settings?.quarter;
					}) as e (e.id)}
						<div
							class="flex items-center justify-between rounded-xl border border-border bg-surface p-3 transition-colors hover:border-primary/30"
						>
							<div class="flex items-center gap-3">
								<div
									class="flex size-9 items-center justify-center rounded-full {isLate(e)
										? 'bg-warning/10 text-warning'
										: 'bg-primary/10 text-primary'}"
								>
									<svg
										class="size-4"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
									>
										<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10" />
									</svg>
								</div>
								<div>
									<div class="text-sm font-medium">
										{new Date(e.timestamp).toLocaleDateString(undefined, {
											month: 'short',
											day: 'numeric',
											year: 'numeric'
										})}
									</div>
									<div class="font-mono text-xs text-muted-foreground">
										{new Date(e.timestamp).toLocaleTimeString(undefined, {
											hour: '2-digit',
											minute: '2-digit'
										})}
										{#if isLate(e)}
											<span class="text-warning ml-2 font-bold">LATE</span>
										{/if}
									</div>
								</div>
							</div>
							<div class="rounded border border-border bg-background px-2 py-1 font-mono text-xs">
								{e.type.toUpperCase()}
							</div>
						</div>
					{/each}
				</div>
			{/if}

			<div class="flex justify-end pt-2">
				<button
					onclick={onClose}
					class="rounded-pill bg-primary px-6 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
				>
					Done
				</button>
			</div>
		{/if}
	</div>
</Dialog>

<style>
	.custom-scrollbar::-webkit-scrollbar {
		width: 4px;
	}
	.custom-scrollbar::-webkit-scrollbar-track {
		background: transparent;
	}
	.custom-scrollbar::-webkit-scrollbar-thumb {
		background: var(--color-border);
		border-radius: 10px;
	}
	.custom-scrollbar::-webkit-scrollbar-thumb:hover {
		background: var(--color-muted-foreground);
	}
</style>
