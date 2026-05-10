<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import { listEvents, listStudents, type AttendanceEvent, type Student } from '$lib/db';
	import { fmtDate, fmtTime } from '$lib/csv';

	let students = $state<Student[]>([]);
	let events = $state<AttendanceEvent[]>([]);

	onMount(async () => {
		[students, events] = await Promise.all([listStudents(), listEvents()]);
	});

	const today = fmtDate(Date.now());

	const todayEvents = $derived(events.filter((e) => fmtDate(e.timestamp) === today));

	const studentMap = $derived(new SvelteMap(students.map((s) => [s.id, s])));

	// Last event per student today — determine who's currently checked in
	const checkedIn = $derived.by(() => {
		const lastByStudent = new SvelteMap<string, AttendanceEvent>();
		for (const e of [...todayEvents].sort((a, b) => a.timestamp - b.timestamp)) {
			lastByStudent.set(e.studentId, e);
		}
		return [...lastByStudent.values()].filter((e) => e.type === 'in');
	});

	const weekday = new Date().toLocaleDateString(undefined, { weekday: 'long' });
</script>

<svelte:head>
	<title>Dashboard — Attendance System</title>
	<meta name="description" content="Today's attendance at a glance." />
</svelte:head>

<AppShell>
	<PageHeader
		category="Overview"
		title="Attendance Overview"
		description="A simple overview of today's attendance. Monitor real-time logs and manage student check-ins."
	>
		{#snippet actions()}
			<a
				href={resolve('/students')}
				onclick={(e) => {
					e.preventDefault();
					goto(resolve('/students'));
				}}
				class="rounded-pill border-border bg-background hover:bg-surface inline-flex items-center gap-2 border px-4 py-2 text-sm font-medium transition-colors"
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
					<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
					<circle cx="9" cy="7" r="4" />
					<path d="M23 21v-2a4 4 0 0 0-3-3.87" />
					<path d="M16 3.13a4 4 0 0 1 0 7.75" />
				</svg>
				Manage students
			</a>
			<a
				href={resolve('/attendance')}
				onclick={(e) => {
					e.preventDefault();
					goto(resolve('/attendance'));
				}}
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
					<path d="M3 7V5a2 2 0 0 1 2-2h2" />
					<path d="M17 3h2a2 2 0 0 1 2 2v2" />
					<path d="M21 17v2a2 2 0 0 1-2 2h-2" />
					<path d="M7 21H5a2 2 0 0 1-2-2v-2" />
					<line x1="7" y1="12" x2="17" y2="12" />
				</svg>
				Open Tap Mode
			</a>
		{/snippet}
	</PageHeader>

	<!-- Stats row -->
	<section class="grid gap-4 px-6 py-10 sm:grid-cols-2 md:px-12 lg:grid-cols-3">
		{@render statCard('Students enrolled', students.length)}
		{@render statCard('Logged today', todayEvents.length, true)}
		{@render statCard('Currently checked in', checkedIn.length)}
	</section>

	<!-- Panels -->
	<section class="grid gap-8 px-6 pb-16 md:px-12 lg:grid-cols-2">
		<!-- Currently in the room -->
		<div class="border-border bg-card rounded-2xl border p-6">
			<div class="mb-4 flex items-baseline justify-between">
				<h3 class="text-lg font-medium">Currently in the room</h3>
				<span class="label-mono">Last tap registered as check-in</span>
			</div>

			{#if checkedIn.length === 0}
				{@render emptyState('No one is checked in yet. Open Tap Mode to begin.')}
			{:else}
				<ul class="divide-border divide-y">
					{#each checkedIn as e (e.id)}
						{@const s = studentMap.get(e.studentId)}
						<li class="flex items-center justify-between py-3">
							<div>
								<div class="font-medium">{s?.name ?? 'Unknown'}</div>
								<div class="label-mono">#{s?.studentNumber}</div>
							</div>
							<div class="text-muted-foreground font-mono text-sm">in · {fmtTime(e.timestamp)}</div>
						</li>
					{/each}
				</ul>
			{/if}
		</div>

		<!-- Recent activity -->
		<div class="border-border bg-card rounded-2xl border p-6">
			<div class="mb-4 flex items-baseline justify-between">
				<h3 class="text-lg font-medium">Recent activity</h3>
				<span class="label-mono">Last 8 events</span>
			</div>

			{#if events.length === 0}
				{@render emptyState('No events yet.')}
			{:else}
				<ul class="divide-border divide-y">
					{#each events.slice(0, 8) as e (e.id)}
						{@const s = studentMap.get(e.studentId)}
						<li class="flex items-center justify-between py-3">
							<div class="min-w-0">
								<div class="truncate font-medium">{s?.name ?? 'Unknown'}</div>
								<div class="label-mono">{fmtDate(e.timestamp)} · {fmtTime(e.timestamp)}</div>
							</div>
							<span
								class="rounded-pill px-2 py-1 font-mono text-xs {e.type === 'in'
									? 'bg-primary text-primary-foreground'
									: 'bg-surface text-foreground border-border border'}"
							>
								{e.type === 'in' ? 'CHECK-IN' : 'CHECK-OUT'}
							</span>
						</li>
					{/each}
				</ul>
			{/if}

			<div class="mt-4">
				<a
					href={resolve('/records')}
					onclick={(e) => {
						e.preventDefault();
						goto(resolve('/records'));
					}}
					class="text-primary hover:text-accent inline-flex items-center gap-1 font-mono text-sm transition-colors"
				>
					View all records
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
						<line x1="7" y1="17" x2="17" y2="7" />
						<polyline points="7 7 17 7 17 17" />
					</svg>
				</a>
			</div>
		</div>
	</section>
</AppShell>

{#snippet statCard(label: string, value: number, accent = false)}
	<div
		class="border-border rounded-2xl border p-6 {accent
			? 'bg-primary text-primary-foreground'
			: 'bg-surface'}"
	>
		<div class="label-mono {accent ? '!text-primary-foreground/80' : ''}">{label}</div>
		<div class="mt-2 text-5xl font-medium tracking-tight">{value}</div>
	</div>
{/snippet}

{#snippet emptyState(text: string)}
	<div
		class="text-muted-foreground border-border rounded-xl border border-dashed py-10 text-center text-sm"
	>
		<svg
			class="mx-auto mb-2 size-5 opacity-60"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
			aria-hidden="true"
		>
			<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
			<polyline points="14 2 14 8 20 8" />
			<line x1="16" y1="13" x2="8" y2="13" />
			<line x1="16" y1="17" x2="8" y2="17" />
			<polyline points="10 9 9 9 8 9" />
		</svg>
		{text}
	</div>
{/snippet}
