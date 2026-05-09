<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import {
		listStudents,
		listEvents,
		getSettings,
		deleteEvent,
		type Student,
		type AttendanceEvent
	} from '$lib/db';
	import { downloadCSV, eventsToCSV, fmtDate, fmtDateTime, fmtTime } from '$lib/csv';

	// ── State ────────────────────────────────────────────────────────────────
	let students = $state<Student[]>([]);
	let events = $state<AttendanceEvent[]>([]);
	let from = $state('');
	let to = $state('');
	let studentId = $state('');
	let lateAfter = $state('08:45');

	// Toast
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	// ── Derived ──────────────────────────────────────────────────────────────
	let studentMap = $derived(new Map(students.map((s) => [s.id, s])));

	let filtered = $derived(
		events.filter((e) => {
			const d = fmtDate(e.timestamp);
			if (from && d < from) return false;
			if (to && d > to) return false;
			if (studentId && e.studentId !== studentId) return false;
			return true;
		})
	);

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	async function reload() {
		const [s, e, st] = await Promise.all([listStudents(), listEvents(), getSettings()]);
		students = s;
		events = e;
		lateAfter = st.lateAfter;
	}

	function onExport() {
		const csv = eventsToCSV(filtered, students, lateAfter);
		const range = from || to ? `_${from || 'start'}_to_${to || 'end'}` : '';
		downloadCSV(`horizon-attendance${range}.csv`, csv);
		toast('CSV exported');
	}

	async function onDelete(id: string) {
		if (!confirm('Delete this event?')) return;
		await deleteEvent(id);
		toast('Deleted');
		await reload();
	}

	// ── Lifecycle ────────────────────────────────────────────────────────────
	onMount(() => {
		reload();
	});
</script>

<svelte:head>
	<title>Records — Horizon Attendance</title>
	<meta name="description" content="Filter, review, and export attendance records as CSV." />
</svelte:head>

<AppShell>
	<PageHeader
		step="Step 04 · Records"
		title="Review and export attendance"
		description="Filter by date and student, then export a daily CSV summary with check-in, check-out, hours, and lateness."
	>
		{#snippet actions()}
			<button
				onclick={onExport}
				class="rounded-pill bg-primary text-primary-foreground hover:bg-accent inline-flex items-center gap-2 px-4 py-2 text-sm font-medium transition-colors"
			>
				<!-- Download icon -->
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
		<!-- From -->
		<div class="space-y-2">
			<div class="label-mono">From</div>
			<input
				type="date"
				bind:value={from}
				class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
			/>
		</div>

		<!-- To -->
		<div class="space-y-2">
			<div class="label-mono">To</div>
			<input
				type="date"
				bind:value={to}
				class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
			/>
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
						<th class="label-mono px-4 py-3">Type</th>
						<th class="label-mono w-20 px-4 py-3 text-right"> </th>
					</tr>
				</thead>
				<tbody class="divide-border divide-y">
					{#if filtered.length === 0}
						{@render emptyState()}
					{:else}
						{#each filtered as e (e.id)}
							{@const s = studentMap.get(e.studentId)}
							<tr class="hover:bg-surface/40 transition-colors">
								<td class="px-4 py-3 align-top font-mono">{fmtDateTime(e.timestamp)}</td>
								<td class="px-4 py-3 align-top">
									<div class="font-medium">{s?.name ?? 'Unknown'}</div>
									<div class="label-mono">#{s?.studentNumber}</div>
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
										<!-- Trash2 icon -->
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
</AppShell>

<!-- ── Toast ──────────────────────────────────────────────────────────────── -->
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

<!-- ── Snippets ───────────────────────────────────────────────────────────── -->
{#snippet emptyState()}
	<tr>
		<td colspan={4} class="text-muted-foreground px-4 py-12 text-center">
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
