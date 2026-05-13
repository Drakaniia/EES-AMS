<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import {
		listEvents,
		listStudents,
		listClasses,
		type AttendanceEvent,
		type Student,
		type Class
	} from '$lib/db-rust';
	import { fmtDate, fmtTime } from '$lib/csv';

	let students = $state<Student[]>([]);
	let events = $state<AttendanceEvent[]>([]);
	let classes = $state<Class[]>([]);
	let manualActiveClassId = $state<string | null>(null);

	let sessionSummary = $state<{ summary: string; className: string } | null>(null);

	onMount(async () => {
		[students, events, classes] = await Promise.all([listStudents(), listEvents(), listClasses()]);
		const active = getActiveClass();
		if (active) {
			manualActiveClassId = active.id;
		}

		const sessionEnd = page.url.searchParams.get('sessionEnd');
		if (sessionEnd === 'true') {
			sessionSummary = {
				summary: page.url.searchParams.get('summary') || '',
				className: page.url.searchParams.get('className') || ''
			};
			// Clear URL params
			goto(resolve('/'), { replaceState: true });
			// Auto-hide after 10 seconds
			setTimeout(() => (sessionSummary = null), 10000);
		}
	});

	const today = fmtDate(Date.now());

	const todayEvents = $derived(events.filter((e) => fmtDate(e.timestamp) === today));

	const studentMap = $derived(new SvelteMap(students.map((s) => [s.id, s])));

	// Last event per student today — determine who's currently checked in
	const checkedIn = $derived.by(() => {
		const lastByStudent = new SvelteMap<string, AttendanceEvent>();
		// Filter events to only include those from today and optionally for the active class
		const relevantEvents = activeClass
			? todayEvents.filter((e) => e.classId === activeClass.id)
			: todayEvents;

		for (const e of [...relevantEvents].sort((a, b) => {
			const aTime = typeof a.timestamp === 'string' ? new Date(a.timestamp).getTime() : a.timestamp;
			const bTime = typeof b.timestamp === 'string' ? new Date(b.timestamp).getTime() : b.timestamp;
			return aTime - bTime;
		})) {
			lastByStudent.set(e.studentId, e);
		}
		return [...lastByStudent.values()].filter((e) => e.type === 'in');
	});

	// ── Utility Functions ────────────────────────────────────────────────────────

	function getActiveClass(): Class | null {
		const now = new Date();
		const currentTime = now.getHours() * 60 + now.getMinutes();

		for (const cls of classes) {
			const [startHour, startMin] = cls.dayStart.split(':').map(Number);
			const [endHour, endMin] = cls.dayEnd.split(':').map(Number);
			const startTime = startHour * 60 + startMin;
			const endTime = endHour * 60 + endMin;

			if (currentTime >= startTime && currentTime <= endTime) {
				return cls;
			}
		}
		return null;
	}

	function getNextClass(): { cls: Class; minutes: number } | null {
		const now = new Date();
		const currentTime = now.getHours() * 60 + now.getMinutes();

		let next: { cls: Class; minutes: number } | null = null;

		for (const cls of classes) {
			const [startHour, startMin] = cls.dayStart.split(':').map(Number);
			const startTime = startHour * 60 + startMin;

			if (startTime > currentTime) {
				const diff = startTime - currentTime;
				if (!next || diff < next.minutes) {
					next = { cls, minutes: diff };
				}
			}
		}
		return next;
	}

	// ── Dynamic Logic ──────────────────────────────────────────────────────────

	const activeClass = $derived.by(() => {
		if (manualActiveClassId) {
			return classes.find((c) => c.id === manualActiveClassId) || null;
		}
		return getActiveClass();
	});
	const nextClass = $derived(getNextClass());

	const activeClassStudents = $derived(
		activeClass ? students.filter((s) => s.classId === activeClass.id) : students
	);

	const nextClassStudents = $derived(
		nextClass ? students.filter((s) => s.classId === nextClass.cls.id) : []
	);

	const dynamicTitle = $derived(() => {
		if (activeClass) {
			return manualActiveClassId
				? `Manual Session: ${activeClass.name}`
				: `Currently Teaching: ${activeClass.name}`;
		}
		return 'Dashboard';
	});

	const dynamicDescription = $derived(() => {
		if (activeClass) {
			return `${activeClass.room ? `Room ${activeClass.room} • ` : ''}${activeClass.dayStart} – ${activeClass.dayEnd} • Session ${manualActiveClassId ? 'primed' : 'in progress'}`;
		}
		if (nextClass) {
			return `Welcome back. Your next session, ${nextClass.cls.name}, begins in ${nextClass.minutes} minutes.`;
		}
		return 'No active sessions at the moment. Use this time to prepare or view your schedule.';
	});
</script>

<svelte:head>
	<title>Dashboard — Attendance System</title>
	<meta name="description" content="Today's attendance at a glance." />
</svelte:head>

<AppShell>
	<PageHeader
		category={activeClass ? 'Live' : 'Dashboard'}
		title={dynamicTitle()}
		description={dynamicDescription()}
	>
		{#snippet actions()}
			{#if classes.length > 0}
				<div class="relative inline-flex items-center">
					<select
						bind:value={manualActiveClassId}
						class="rounded-pill border-border bg-background hover:bg-surface focus:ring-primary/20 h-10 appearance-none border px-4 py-2 pr-10 text-sm font-medium transition-colors focus:ring-2 focus:outline-none"
						aria-label="Manual Session Start"
					>
						<option value={null}>Auto-detect Session</option>
						{#each classes as cls (cls.id)}
							<option value={cls.id}>{cls.name} ({cls.dayStart})</option>
						{/each}
					</select>
					<div class="pointer-events-none absolute right-3 flex items-center">
						<svg
							class="text-muted-foreground size-4"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="m6 9 6 6 6-6" />
						</svg>
					</div>
				</div>
			{/if}

			<a
				href={resolve('/students')}
				onclick={(e) => {
					e.preventDefault();
					goto(resolve('/students'));
				}}
				class="rounded-pill border-border bg-background hover:bg-surface inline-flex h-10 items-center gap-2 border px-4 py-2 text-sm font-medium transition-colors"
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
				href={activeClass
					? resolve(`/attendance?classId=${activeClass.id}`)
					: resolve('/attendance')}
				onclick={(e) => {
					e.preventDefault();
					goto(
						activeClass ? resolve(`/attendance?classId=${activeClass.id}`) : resolve('/attendance')
					);
				}}
				class="rounded-pill bg-primary text-primary-foreground hover:bg-accent inline-flex h-10 items-center gap-2 px-4 py-2 text-sm font-medium transition-colors"
			>
				{#if activeClass}
					<span class="relative flex h-2 w-2">
						<span
							class="absolute inline-flex h-full w-full animate-ping rounded-full bg-white opacity-75"
						></span>
						<span class="relative inline-flex h-2 w-2 rounded-full bg-white"></span>
					</span>
				{/if}
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
				Start Attendance
			</a>
		{/snippet}
	</PageHeader>

	{#if sessionSummary}
		<div class="px-6 pt-10 md:px-12">
			<div
				class="bg-primary/10 border-primary/20 text-primary flex items-center justify-between rounded-2xl border p-6"
			>
				<div class="flex items-center gap-4">
					<div
						class="bg-primary text-primary-foreground grid size-12 place-items-center rounded-full"
					>
						<svg
							class="size-6"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2.5"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<polyline points="20 6 9 17 4 12" />
						</svg>
					</div>
					<div>
						<h4 class="text-lg font-bold">Session Complete: {sessionSummary.className}</h4>
						<p class="text-sm font-medium opacity-80">{sessionSummary.summary}</p>
					</div>
				</div>
				<button
					onclick={() => (sessionSummary = null)}
					class="hover:bg-primary/10 rounded-full p-2 transition-colors"
					aria-label="Close session summary"
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
						<path d="M18 6 6 18" /><path d="m6 6 12 12" />
					</svg>
				</button>
			</div>
		</div>
	{/if}

	<!-- Stats row -->
	<section class="grid gap-4 px-6 py-10 sm:grid-cols-2 md:px-12 lg:grid-cols-3">
		{@render statCard('Class Size', activeClassStudents.length)}
		{@render statCard('Total Attendance', todayEvents.length, true)}
		{@render statCard('Currently checked in', checkedIn.length)}
	</section>

	<!-- Panels -->
	<section class="grid min-h-[calc(100vh-32rem)] gap-8 px-6 pb-16 md:px-12 lg:grid-cols-2">
		<!-- Currently in the room -->
		<div class="border-border bg-card flex h-full flex-col rounded-2xl border p-6">
			<div class="mb-4 flex flex-shrink-0 items-baseline justify-between">
				<div class="flex flex-col">
					<h3 class="text-lg font-medium">
						{activeClass ? 'Currently in the room' : 'Next Session Class List'}
					</h3>
					<span class="label-mono text-xs opacity-60">
						{activeClass
							? 'Last tap registered as check-in'
							: `${nextClassStudents.length} Students`}
					</span>
				</div>
				{#if activeClass}
					<a
						href={resolve(`/attendance?classId=${activeClass.id}&manual=true`)}
						onclick={(e) => {
							e.preventDefault();
							goto(resolve(`/attendance?classId=${activeClass.id}&manual=true`));
						}}
						class="rounded-pill border-border bg-background hover:bg-surface inline-flex h-8 items-center gap-1.5 border px-3 text-xs font-medium transition-colors"
					>
						Manual Check-in
					</a>
				{/if}
			</div>

			{#if activeClass && checkedIn.length === 0}
				<div class="flex flex-1 items-center justify-center">
					{@render emptyState('No one is checked in yet. Start attendance to begin.')}
				</div>
			{:else if !activeClass && nextClass}
				<div class="flex flex-1 flex-col">
					<div class="text-muted-foreground mb-4 text-sm">
						Preparing for <strong>{nextClass.cls.name}</strong>{nextClass.cls.room ? ` (Room ${nextClass.cls.room})` : ''} at {nextClass
							.cls.dayStart}
					</div>
					{#if nextClassStudents.length === 0}
						<div class="flex flex-1 items-center justify-center">
							{@render emptyState('No students enrolled in the next class.', true)}
						</div>
					{:else}
						<ul class="divide-border flex-1 divide-y overflow-y-auto">
							{#each nextClassStudents as s (s.id)}
								<li class="flex items-center justify-between py-3">
									<div>
										<div class="font-medium">{s.name}</div>
										<div class="label-mono">#{s.studentNumber}</div>
									</div>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			{:else if checkedIn.length === 0}
				<div class="flex flex-1 items-center justify-center">
					{@render emptyState('No active sessions at the moment.', true)}
				</div>
			{:else}
				<ul class="divide-border flex-1 divide-y overflow-y-auto">
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
		<div class="border-border bg-card flex h-full flex-col rounded-2xl border p-6">
			<div class="mb-4 flex flex-shrink-0 items-baseline justify-between">
				<h3 class="text-lg font-medium">Recent activity</h3>
				<span class="label-mono">Last 8 events</span>
			</div>

			{#if events.length === 0}
				<div class="flex flex-1 items-center justify-center">
					{@render emptyState('No events yet.')}
				</div>
			{:else}
				<ul class="divide-border flex-1 divide-y overflow-y-auto">
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

			<div class="mt-4 flex-shrink-0">
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
		<div class="label-mono {accent ? 'text-primary-foreground/80!' : ''}">{label}</div>
		<div class="mt-2 text-5xl font-medium tracking-tight">{value}</div>
	</div>
{/snippet}

{#snippet emptyState(text: string, showScheduleAction = false)}
	<div
		class="text-muted-foreground border-border flex h-full w-full flex-col items-center justify-center rounded-xl border border-dashed p-8 text-center text-sm"
	>
		<div class="bg-surface mb-4 rounded-full p-3">
			<svg
				class="size-6 opacity-60"
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
		</div>
		<p class="mb-6 max-w-[200px]">{text}</p>
		{#if showScheduleAction}
			<a
				href={resolve('/settings')}
				onclick={(e) => {
					e.preventDefault();
					goto(resolve('/settings'));
				}}
				class="rounded-pill border-border bg-background hover:bg-surface inline-flex items-center gap-2 border px-4 py-2 text-xs font-medium transition-colors"
			>
				View full schedule
			</a>
		{/if}
	</div>
{/snippet}
