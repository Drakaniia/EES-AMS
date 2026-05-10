<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import DateRangePicker from '$lib/components/ui/DateRangePicker.svelte';
	import Pagination from '$lib/components/ui/Pagination.svelte';
	import {
		listStudents,
		listEvents,
		listClasses,
		getSettings,
		deleteEvent,
		type Student,
		type AttendanceEvent,
		type Class
	} from '$lib/db-rust';
	import { downloadCSV, eventsToCSV, fmtDate, fmtDateTime, fmtTime } from '$lib/csv';

	// ── State ────────────────────────────────────────────────────────────────
	let students = $state<Student[]>([]);
	let events = $state<AttendanceEvent[]>([]);
	let classes = $state<Class[]>([]);
	let from = $state('');
	let to = $state('');
	let studentId = $state('');
	let classId = $state('');
	let lateAfter = $state('08:45');

	// Date range picker dialog state
	let dateRangePickerOpen = $state(false);

	// Toast
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	// Pagination
	let currentPage = $state(1);
	let itemsPerPage = $state(10);

	// ── Derived ──────────────────────────────────────────────────────────────
	let studentMap = $derived(new Map(students.map((s) => [s.id, s])));
	let classMap = $derived(new Map(classes.map((c) => [c.id, c])));

	let filtered = $derived(
		events.filter((e) => {
			const d = fmtDate(e.timestamp);
			if (from && d < from) return false;
			if (to && d > to) return false;
			if (studentId && e.studentId !== studentId) return false;

			if (classId) {
				const s = studentMap.get(e.studentId);
				const eventClassId = e.classId || s?.classId;
				if (eventClassId !== classId) return false;
			}

			return true;
		})
	);

	// Pagination for records
	const totalPages = $derived(Math.ceil(filtered.length / itemsPerPage));
	const paginatedFiltered = $derived(() => {
		const start = (currentPage - 1) * itemsPerPage;
		const end = start + itemsPerPage;
		return filtered.slice(start, end);
	});

	function handlePageChange(page: number) {
		currentPage = page;
	}

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	async function reload() {
		const [s, e, c, st] = await Promise.all([
			listStudents(),
			listEvents(),
			listClasses(),
			getSettings()
		]);
		students = s;
		events = e;
		classes = c;
		lateAfter = st.lateAfter;
		currentPage = 1;
	}

	function onExport() {
		const csv = eventsToCSV(filtered, students, classes, lateAfter);
		const range = from || to ? `_${from || 'start'}_to_${to || 'end'}` : '';
		const classSuffix = classId ? `_${classMap.get(classId)?.name || 'class'}` : '';
		downloadCSV(`attendance-records${classSuffix}${range}.csv`, csv);
		toast('CSV exported');
	}

	async function onDelete(id: string) {
		if (!confirm('Delete this event?')) return;
		await deleteEvent(id);
		toast('Deleted');
		await reload();
	}

	function getEventClassName(e: AttendanceEvent) {
		const id = e.classId || studentMap.get(e.studentId)?.classId;
		if (!id) return '—';
		return classMap.get(id)?.name ?? 'Unknown';
	}

	// ── Lifecycle ────────────────────────────────────────────────────────────
	onMount(() => {
		reload();
	});
</script>

<svelte:head>
	<title>Records — Attendance System</title>
	<meta name="description" content="Filter, review, and export attendance records as CSV." />
</svelte:head>

<AppShell>
	<PageHeader
		category="Archives"
		title="Attendance Logs"
		description="Review and filter historical attendance data for your classes."
	>
		{#snippet actions()}
			<button
				onclick={onExport}
				class="rounded-pill bg-primary text-primary-foreground hover:bg-accent inline-flex items-center gap-2 px-4 py-2 text-sm font-medium transition-colors"
			>
				<svg
					class="size-4"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
					<polyline points="7 10 12 15 17 10" />
					<line x1="12" y1="15" x2="12" y2="3" />
				</svg>
				Export CSV
			</button>
		{/snippet}
	</PageHeader>

	<!-- ── Filters ──────────────────────────────────────────────────────────── -->
	<section class="grid gap-4 px-6 py-8 sm:grid-cols-2 md:px-12 lg:grid-cols-4">
		<!-- Date Range -->
		<div class="space-y-2">
			<div class="label-mono">Date Range</div>
			<button
				onclick={() => (dateRangePickerOpen = true)}
				class="border-border bg-background hover:bg-surface focus:ring-primary flex h-10 w-full items-center justify-between rounded-md border px-3 text-left text-sm transition-colors focus:ring-2 focus:outline-none"
			>
				<span class={from || to ? '' : 'text-muted-foreground'}>
					{from && to
						? `${new Date(from).toLocaleDateString()} - ${new Date(to).toLocaleDateString()}`
						: from
							? `From ${new Date(from).toLocaleDateString()}`
							: to
								? `To ${new Date(to).toLocaleDateString()}`
								: 'Select date range'}
				</span>
				<svg
					class="text-muted-foreground size-4"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
					<line x1="16" y1="2" x2="16" y2="6"></line>
					<line x1="8" y1="2" x2="8" y2="6"></line>
					<line x1="3" y1="10" x2="21" y2="10"></line>
				</svg>
			</button>
		</div>

		<!-- Class -->
		<div class="space-y-2">
			<div class="label-mono">Class</div>
			<select
				bind:value={classId}
				class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
			>
				<option value="">All classes</option>
				{#each classes as c (c.id)}
					<option value={c.id}>{c.name}</option>
				{/each}
			</select>
		</div>

		<!-- Student -->
		<div class="space-y-2">
			<div class="label-mono">Student</div>
			<select
				bind:value={studentId}
				class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
			>
				<option value="">All students</option>
				{#each students as s (s.id)}
					<option value={s.id}>{s.name}</option>
				{/each}
			</select>
		</div>

		<!-- Total -->
		<div class="space-y-2">
			<div class="label-mono">Total events</div>
			<div class="flex h-10 items-center font-mono text-sm">{filtered.length}</div>
		</div>
	</section>

	<!-- ── Table ────────────────────────────────────────────────────────────── -->
	<section class="px-6 pb-16 md:px-12">
		<div class="border-border bg-card overflow-hidden rounded-2xl border">
			<table class="w-full text-sm">
				<thead class="bg-surface text-left">
					<tr>
						<th class="label-mono px-4 py-3">When</th>
						<th class="label-mono px-4 py-3">Student</th>
						<th class="label-mono px-4 py-3">Class</th>
						<th class="label-mono px-4 py-3">Type</th>
						<th class="label-mono w-20 px-4 py-3 text-right"> </th>
					</tr>
				</thead>
				<tbody class="divide-border divide-y">
					{#if filtered.length === 0}
						{@render emptyState()}
					{:else}
						{#each paginatedFiltered() as e (e.id)}
							{@const s = studentMap.get(e.studentId)}
							<tr class="hover:bg-surface/40 transition-colors">
								<td class="px-4 py-3 align-top font-mono">{fmtDateTime(e.timestamp)}</td>
								<td class="px-4 py-3 align-top">
									<div class="font-medium">{s?.name ?? 'Unknown'}</div>
									<div class="label-mono">#{s?.studentNumber}</div>
								</td>
								<td class="px-4 py-3 align-top">
									<span
										class="rounded-pill bg-surface border-border border px-2 py-0.5 text-[10px]"
									>
										{getEventClassName(e)}
									</span>
								</td>
								<td class="px-4 py-3 align-top">
									{@render typePill(e.type)}
									<span class="text-muted-foreground ml-2 font-mono text-xs"
										>{fmtTime(e.timestamp)}</span
									>
								</td>
								<td class="px-4 py-3 text-right align-top">
									<button
										onclick={() => onDelete(e.id)}
										aria-label="Delete event"
										class="border-border text-destructive hover:bg-destructive/10 inline-flex size-8 items-center justify-center rounded-md border transition-colors"
									>
										<svg
											class="size-3.5"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2"
											stroke-linecap="round"
											stroke-linejoin="round"
											aria-hidden="true"
										>
											<polyline points="3 6 5 6 21 6" />
											<path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
											<path d="M10 11v6M14 11v6" />
											<path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
										</svg>
									</button>
								</td>
							</tr>
						{/each}
					{/if}
				</tbody>
			</table>
		</div>
	</section>

	<div class="fixed right-6 bottom-6 z-30">
		<Pagination {currentPage} {totalPages} onPageChange={handlePageChange} />
	</div>
</AppShell>

{#if toastMessage}
	<div
		class="fixed right-6 bottom-6 z-50 rounded-xl border px-4 py-3 text-sm font-medium shadow-lg
			{toastOk
			? 'bg-background border-border text-foreground'
			: 'bg-destructive/10 border-destructive/40 text-destructive'}"
		role="status"
		aria-live="polite"
	>
		{toastMessage}
	</div>
{/if}

<DateRangePicker
	open={dateRangePickerOpen}
	fromValue={from}
	toValue={to}
	on:close={() => (dateRangePickerOpen = false)}
	on:select={(e: CustomEvent<{ from: string; to: string }>) => {
		from = e.detail.from;
		to = e.detail.to;
	}}
/>

{#snippet emptyState()}
	<tr>
		<td colspan={5} class="text-muted-foreground px-4 py-12 text-center">
			No records match the filters.
		</td>
	</tr>
{/snippet}

{#snippet typePill(type: 'in' | 'out')}
	<span
		class="rounded-pill px-2 py-1 font-mono text-xs
			{type === 'in'
			? 'bg-primary text-primary-foreground'
			: 'bg-surface border-border text-foreground border'}"
	>
		{type === 'in' ? 'CHECK-IN' : 'CHECK-OUT'}
	</span>
{/snippet}
