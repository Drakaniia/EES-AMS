<script lang="ts">
	import {
		listEventsForStudent,
		getSettings,
		type Student,
		type AttendanceEvent,
		type Settings
	} from '$lib/api';
	import { filterAttendanceEventsForQuarter } from '$lib/student-analytics';
	import { ChevronLeft, ChevronRight } from 'lucide-svelte';
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

	const monthlyEvents = $derived.by(() =>
		events.filter((e) => {
			const d = new Date(e.timestamp);
			return d.getMonth() === selectedMonth && d.getFullYear() === selectedYear;
		})
	);

	const quarterlyEvents = $derived.by(() =>
		filterAttendanceEventsForQuarter(events, settings, selectedYear)
	);

	function getStatus(dayEvents: AttendanceEvent[]) {
		if (dayEvents.length === 0) return 'none';
		const hasIn = dayEvents.some((e) => e.type === 'in');
		if (hasIn) return 'present';
		// Only an explicit absent record → the day is marked absent.
		return dayEvents.some((e) => e.type === 'absent') ? 'absent' : 'none';
	}

	function isLate(event: AttendanceEvent) {
		if (event.type !== 'in' || !settings) return false;
		const eventTime = event.timestamp.split('T')[1].substring(0, 5); // HH:mm
		return eventTime > settings.lateAfter;
	}

	const stats = $derived.by(() => {
		const filtered = viewMode === 'quarter' ? quarterlyEvents : monthlyEvents;

		// Only days with an 'in' record count as presents - absent marks do not.
		const presents = new Set(
			filtered.filter((e) => e.type === 'in').map((e) => e.timestamp.split('T')[0])
		).size;
		const absences = new Set(
			filtered.filter((e) => e.type === 'absent').map((e) => e.timestamp.split('T')[0])
		).size;
		const tardies = filtered.filter((e) => isLate(e)).length;

		return { presents, absences, tardies };
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
	{onClose}
>
	<div class="space-y-5">
		{#if loading}
			<div class="flex h-64 items-center justify-center">
				<div
					class="flex items-center gap-2 text-sm font-medium text-muted-foreground"
					role="status"
				>
					<span class="size-2 rounded-full bg-primary" aria-hidden="true"></span>
					Loading attendance…
				</div>
			</div>
		{:else}
			<!-- Header Stats -->
			<dl class="grid grid-cols-2 gap-2 sm:grid-cols-4">
				{@render statCell('Present', String(stats.presents))}
				{@render statCell('Absent', String(stats.absences), 'text-destructive')}
				{@render statCell('Late', String(stats.tardies))}
				{@render statCell('Quarter', settings?.quarter || '—')}
			</dl>

			<!-- Controls -->
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-2">
					<button onclick={prevMonth} class="icon-btn" aria-label="Previous month">
						<ChevronLeft class="size-4" aria-hidden="true" />
					</button>
					<span class="min-w-32 text-center text-[12.5px] font-semibold">
						{months[selectedMonth]}
						{selectedYear}
					</span>
					<button onclick={nextMonth} class="icon-btn" aria-label="Next month">
						<ChevronRight class="size-4" aria-hidden="true" />
					</button>
				</div>

				<div class="segmented" role="group" aria-label="Attendance period">
					<button
						onclick={() => (viewMode = 'month')}
						aria-pressed={viewMode === 'month'}
						class="segmented-item"
					>
						Monthly
					</button>
					<button
						onclick={() => (viewMode = 'quarter')}
						aria-pressed={viewMode === 'quarter'}
						class="segmented-item"
					>
						Quarterly
					</button>
				</div>
			</div>

			{#if viewMode === 'month'}
				<!-- Calendar Grid -->
				<div class="grid grid-cols-7 gap-1">
					{#each ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'] as day (day)}
						<div class="pb-1.5 text-center text-[10.5px] font-semibold text-muted-foreground">
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
									: status === 'absent'
										? 'border-destructive/20 bg-destructive/10'
										: 'bg-surface'}"
							>
								<span
									class="text-[11.5px] tabular-nums {status === 'present'
										? 'font-semibold'
										: status === 'absent'
											? 'font-semibold text-destructive'
											: 'text-muted-foreground'}"
								>
									{d.day}
								</span>
								{#if status === 'present'}
									<div class="mt-1 size-1 rounded-full {late ? 'bg-warning' : 'bg-primary'}"></div>
								{:else if status === 'absent'}
									<div class="mt-1 size-1 rounded-full bg-destructive"></div>
								{/if}
							</div>
						{/if}
					{/each}
				</div>
			{:else}
				<!-- Quarterly View (List of events grouped by month in quarter) -->
				<div class="custom-scrollbar max-h-80 space-y-4 overflow-y-auto pr-2">
					{#if quarterlyEvents.length === 0}
						<div
							class="rounded-xl border border-dashed border-border bg-surface/60 px-4 py-8 text-center text-sm text-muted-foreground"
						>
							No attendance events found for {settings?.quarter ?? 'the selected quarter'}.
						</div>
					{:else}
						{#each quarterlyEvents as e (e.id)}
							<div
								class="flex items-center justify-between rounded-xl border border-border bg-surface p-3 transition-colors hover:border-primary/30"
							>
								<div class="flex items-center gap-3">
									<div
										class="flex size-9 items-center justify-center rounded-full {isLate(e)
											? 'bg-warning/10 text-warning'
											: e.type === 'absent'
												? 'bg-destructive/10 text-destructive'
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
												<span class="ml-1.5 font-semibold text-amber-600 dark:text-amber-400"
													>Late</span
												>
											{/if}
										</div>
									</div>
								</div>
								<div
									class="rounded border px-2 py-1 font-mono text-xs {e.type === 'absent'
										? 'border-destructive/30 bg-destructive/10 text-destructive'
										: 'border-border bg-background'}"
								>
									{e.type.toUpperCase()}
								</div>
							</div>
						{/each}
					{/if}
				</div>
			{/if}
			<div class="flex justify-end border-t border-border pt-3">
				<button onclick={onClose} class="btn btn-primary min-w-24">Done</button>
			</div>
		{/if}
	</div>
</Dialog>

{#snippet statCell(label: string, value: string, tone = '')}
	<div class="rounded-lg border border-border bg-card px-3 py-2">
		<dt class="text-[11px] font-medium text-muted-foreground">{label}</dt>
		<dd class="mt-0.5 text-[17px] font-semibold tabular-nums {tone}">{value}</dd>
	</div>
{/snippet}

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
