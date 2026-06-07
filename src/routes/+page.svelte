<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import {
		listEvents,
		listStudents,
		listClasses,
		type AttendanceEvent,
		type Student,
		type Class
	} from '$lib/db-rust';
	import { fmtDate, fmtTime } from '$lib/csv';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import {
		ArrowUpRight,
		CalendarClock,
		CheckCircle2,
		History,
		ScanLine,
		UsersRound
	} from 'lucide-svelte';

	let students = $state<Student[]>([]);
	let events = $state<AttendanceEvent[]>([]);
	let classes = $state<Class[]>([]);
	let loading = $state(true);
	let loadError = $state<string | null>(null);
	let sessionSummary = $state<{ summary: string; className: string } | null>(null);
	let todayKey = $state(fmtDate(Date.now()));
	let midnightTimer: ReturnType<typeof setTimeout> | null = null;

	onMount(async () => {
		await reload();
		scheduleMidnightRefresh();

		const sessionEnd = page.url.searchParams.get('sessionEnd');
		if (sessionEnd === 'true') {
			sessionSummary = {
				summary: page.url.searchParams.get('summary') || '',
				className: page.url.searchParams.get('className') || ''
			};
			goto(resolve('/'), { replaceState: true });
			setTimeout(() => (sessionSummary = null), 10000);
		}
	});

	onDestroy(() => {
		if (midnightTimer) clearTimeout(midnightTimer);
	});

	async function reload() {
		loading = true;
		loadError = null;
		try {
			const [loadedStudents, loadedEvents, loadedClasses] = await Promise.all([
				listStudents(),
				listEvents(),
				listClasses(),
				settingsStore.load()
			]);
			students = loadedStudents;
			events = loadedEvents;
			classes = loadedClasses;
		} catch (error) {
			loadError =
				error instanceof Error ? error.message : 'The local attendance backend is unavailable.';
		} finally {
			loading = false;
		}
	}

	const today = $derived(todayKey);
	const todayEvents = $derived(events.filter((event) => fmtDate(event.timestamp) === today));
	const studentMap = $derived(new SvelteMap(students.map((student) => [student.id, student])));
	const activeClass = $derived(getActiveClass());
	const assignedClass = $derived(activeClass ?? classes[0] ?? null);
	const isCardReaderMode = $derived(settingsStore.settings?.attendanceMode === 'card_reader');
	const attendanceActionLabel = $derived(
		isCardReaderMode ? 'Start Live Session' : 'Take Attendance'
	);
	const attendanceFallbackLabel = $derived(isCardReaderMode ? 'Manual Entry' : 'Open Attendance');

	const activeClassStudents = $derived(
		assignedClass ? students.filter((student) => student.classId === assignedClass.id) : students
	);
	const rosterStudents = $derived(activeClassStudents);
	const relevantTodayEvents = $derived.by(() => {
		if (!assignedClass) return todayEvents;
		return todayEvents.filter((event) => {
			const student = studentMap.get(event.studentId);
			return event.classId === assignedClass.id || student?.classId === assignedClass.id;
		});
	});
	const checkedIn = $derived.by(() => {
		const lastByStudent = new SvelteMap<string, AttendanceEvent>();
		for (const event of [...relevantTodayEvents].sort((a, b) => eventTime(a) - eventTime(b))) {
			lastByStudent.set(event.studentId, event);
		}
		return [...lastByStudent.values()].filter((event) => event.type === 'in');
	});
	const recordedStudentIds = $derived.by(() => new Set(checkedIn.map((event) => event.studentId)));
	const notRecordedStudents = $derived.by(() =>
		activeClassStudents.filter((student) => !recordedStudentIds.has(student.id))
	);
	const pendingCount = $derived(notRecordedStudents.length);
	const attendanceRate = $derived(
		activeClassStudents.length === 0
			? 0
			: Math.round((checkedIn.length / activeClassStudents.length) * 100)
	);
	const recentEvents = $derived.by(() =>
		[...events].sort((a, b) => eventTime(b) - eventTime(a)).slice(0, 8)
	);

	const dynamicTitle = $derived.by(() => {
		if (activeClass) return `Currently Teaching: ${activeClass.name}`;
		if (assignedClass) return `${assignedClass.name} Overview`;
		return 'Attendance Overview';
	});

	const dynamicDescription = $derived.by(() => {
		if (activeClass) {
			const room = activeClass.room ? `Room ${activeClass.room} / ` : '';
			return `${room}${activeClass.dayStart} - ${activeClass.dayEnd} / Session in progress`;
		}
		if (assignedClass) {
			const room = assignedClass.room ? `Room ${assignedClass.room} / ` : '';
			return `${room}${assignedClass.dayStart} - ${assignedClass.dayEnd} / Showing today's class roster.`;
		}
		return 'No class is configured yet. Add the class schedule and roster to begin tracking attendance.';
	});

	function getActiveClass(): Class | null {
		const now = new Date();
		const currentTime = now.getHours() * 60 + now.getMinutes();
		const currentDay = now.getDay();

		for (const classItem of classes) {
			if (classItem.days && !classItem.days.includes(currentDay)) continue;

			const [startHour, startMin] = classItem.dayStart.split(':').map(Number);
			const [endHour, endMin] = classItem.dayEnd.split(':').map(Number);
			const startTime = startHour * 60 + startMin;
			const endTime = endHour * 60 + endMin;

			if (currentTime >= startTime && currentTime <= endTime) return classItem;
		}
		return null;
	}

	function eventTime(event: AttendanceEvent) {
		return typeof event.timestamp === 'string'
			? new Date(event.timestamp).getTime()
			: event.timestamp;
	}

	function attendanceHref(
		classId?: string,
		manualFallback = false
	): '/attendance' | `/attendance?${string}` {
		const params: string[] = [];
		if (classId) params.push(`classId=${encodeURIComponent(classId)}`);
		if (manualFallback && isCardReaderMode) params.push('manual=true');
		const query = params.join('&');
		return query ? (`/attendance?${query}` as `/attendance?${string}`) : '/attendance';
	}

	function lastEventForStudentToday(student: Student) {
		return todayEvents
			.filter((event) => event.studentId === student.id)
			.sort((a, b) => eventTime(b) - eventTime(a))[0];
	}

	function initials(name: string) {
		return (
			name
				.split(/\s+/)
				.filter(Boolean)
				.slice(0, 2)
				.map((part) => part[0]?.toUpperCase())
				.join('') || 'ST'
		);
	}

	function scheduleMidnightRefresh() {
		if (midnightTimer) clearTimeout(midnightTimer);
		const now = new Date();
		const nextMidnight = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1, 0, 0, 2, 0);
		midnightTimer = setTimeout(
			async () => {
				todayKey = fmtDate(Date.now());
				await reload();
				scheduleMidnightRefresh();
			},
			Math.max(1000, nextMidnight.getTime() - now.getTime())
		);
	}
</script>

<svelte:head>
	<title>Dashboard - Attendance System</title>
	<meta name="description" content="Today's attendance at a glance." />
</svelte:head>

<PageHeader
	category={activeClass ? 'Live Session' : 'Dashboard'}
	title={dynamicTitle}
	description={dynamicDescription}
>
	{#snippet actions()}
		<a href={resolve('/students')} class="btn btn-secondary control-ring">
			<UsersRound class="size-4" aria-hidden="true" />
			Manage students
		</a>
		<a href={resolve(attendanceHref(assignedClass?.id))} class="btn btn-primary control-ring">
			{#if activeClass}
				<span class="relative flex h-2 w-2" aria-hidden="true">
					<span
						class="absolute inline-flex h-full w-full animate-ping rounded-full bg-white opacity-75"
					></span>
					<span class="relative inline-flex h-2 w-2 rounded-full bg-white"></span>
				</span>
			{/if}
			<ScanLine class="size-4" aria-hidden="true" />
			{attendanceActionLabel}
		</a>
	{/snippet}
</PageHeader>

<div class="page-frame flex flex-col gap-5">
	{#if sessionSummary}
		<div
			class="notice-banner flex flex-col gap-4 p-4 text-primary sm:flex-row sm:items-center sm:justify-between"
			role="status"
			aria-live="polite"
		>
			<div class="flex min-w-0 items-center gap-3">
				<div
					class="grid size-10 shrink-0 place-items-center rounded-lg bg-primary text-primary-foreground"
				>
					<CheckCircle2 class="size-5" aria-hidden="true" />
				</div>
				<div class="min-w-0">
					<h2 class="text-balance-safe font-semibold">
						Session complete: {sessionSummary.className}
					</h2>
					<p class="text-balance-safe mt-0.5 text-sm text-primary/80">{sessionSummary.summary}</p>
				</div>
			</div>
			<button
				type="button"
				onclick={() => (sessionSummary = null)}
				class="btn btn-secondary control-ring w-fit"
			>
				Dismiss
			</button>
		</div>
	{/if}

	{#if loading}
		<LoadingBlock rows={3} label="Loading attendance dashboard" />
	{:else if loadError}
		<EmptyState tone="warning" title="Attendance data could not be loaded" description={loadError}>
			{#snippet actions()}
				<button type="button" onclick={reload} class="btn btn-secondary control-ring">
					Retry
				</button>
			{/snippet}
		</EmptyState>
	{:else}
		<section class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4" aria-label="Today summary">
			{@render statCard('Roster', activeClassStudents.length, 'Students in scope')}
			{@render statCard('Present', checkedIn.length, 'Marked present today', true)}
			{@render statCard('Absent', pendingCount, 'No present record today')}
			{@render statCard('Rate', `${attendanceRate}%`, 'Current completion')}
		</section>

		<section class="surface-panel p-5" aria-label="Attendance completion">
			<div class="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
				<div class="min-w-0">
					<div class="label-mono">Today completion</div>
					<h2 class="mt-2 text-xl leading-tight font-black text-foreground">
						{checkedIn.length} present / {pendingCount} absent
					</h2>
					<p class="text-balance-safe mt-1 text-sm leading-6 text-muted-foreground">
						{activeClass
							? `Tracking ${activeClass.name} for the active session.`
							: assignedClass
								? `Tracking ${assignedClass.name} for today.`
								: 'Tracking all students with attendance activity for today.'}
					</p>
				</div>
				<div class="w-full min-w-0 md:max-w-sm">
					<div class="mb-2 flex items-center justify-between gap-3 text-sm">
						<span class="font-semibold text-foreground">{attendanceRate}% complete</span>
						<span class="text-muted-foreground">{activeClassStudents.length} total</span>
					</div>
					<div class="progress-track" aria-hidden="true">
						<div class="progress-fill" style={`width: ${attendanceRate}%`}></div>
					</div>
				</div>
			</div>
		</section>

		<section class="grid min-h-[28rem] gap-5 xl:grid-cols-[minmax(0,1.18fr)_minmax(320px,0.82fr)]">
			<div class="surface-panel flex min-h-0 flex-col">
				<div class="panel-header flex-wrap">
					<div class="min-w-0">
						<h2 class="text-lg font-black">
							{activeClass ? 'Session roster' : 'Class roster'}
						</h2>
						<p class="mt-1 text-sm text-muted-foreground">
							{activeClass
								? `${checkedIn.length} recorded / ${pendingCount} pending`
								: assignedClass
									? `${activeClassStudents.length} students in this class`
									: 'No class roster is configured.'}
						</p>
					</div>
					{#if activeClass}
						<a
							href={resolve(attendanceHref(activeClass.id, true))}
							class="btn btn-secondary control-ring min-h-9 px-3 text-xs"
						>
							{attendanceFallbackLabel}
							<ArrowUpRight class="size-3.5" aria-hidden="true" />
						</a>
					{/if}
				</div>

				<div class="panel-body min-h-0 flex-1 overflow-y-auto">
					{#if rosterStudents.length === 0}
						<EmptyState
							title={activeClass ? 'No students assigned to this class' : 'No roster to show'}
							description={activeClass
								? 'Assign students to this class from the Class List before taking attendance.'
								: 'Create a class schedule in Configuration and add students to the class.'}
						/>
					{:else}
						<ul class="grid auto-rows-fr gap-2 sm:grid-cols-2 2xl:grid-cols-3">
							{#each rosterStudents as student (student.id)}
								{@const event = lastEventForStudentToday(student)}
								<li class="list-row flex min-w-0 items-center gap-3 p-3">
									<div
										class="grid size-10 shrink-0 place-items-center rounded-lg border font-mono text-xs font-bold {event
											? 'border-primary/30 bg-primary text-primary-foreground'
											: 'border-border bg-surface text-muted-foreground'}"
										aria-hidden="true"
									>
										{initials(student.name)}
									</div>
									<div class="min-w-0 flex-1">
										<div class="text-balance-safe text-sm leading-snug font-semibold">
											{student.name}
										</div>
										<div
											class="mt-1 font-mono text-[11px] {event
												? 'text-primary'
												: 'text-muted-foreground'}"
										>
											{event ? `Recorded ${fmtTime(event.timestamp)}` : 'Not recorded'}
										</div>
									</div>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			</div>

			<div class="grid min-h-0 gap-5 xl:grid-rows-[auto_minmax(0,1fr)]">
				<aside class="surface-panel">
					<div class="panel-header">
						<div class="min-w-0">
							<h2 class="text-lg font-black">Needs attention</h2>
							<p class="mt-1 text-sm text-muted-foreground">Students not recorded for today</p>
						</div>
						<span class="chip shrink-0">{pendingCount} pending</span>
					</div>

					<div class="panel-body">
						{#if notRecordedStudents.length === 0}
							<div
								class="rounded-xl border border-dashed border-border bg-surface/45 px-4 py-6 text-center"
								role="status"
							>
								<CheckCircle2 class="mx-auto size-6 text-primary" aria-hidden="true" />
								<p class="mt-2 text-sm font-semibold">Everyone in scope is recorded.</p>
							</div>
						{:else}
							<ul class="space-y-2">
								{#each notRecordedStudents.slice(0, 6) as student (student.id)}
									<li class="list-row flex min-w-0 items-center gap-3 p-3">
										<div
											class="grid size-9 shrink-0 place-items-center rounded-lg border border-border bg-surface font-mono text-[11px] font-bold text-muted-foreground"
											aria-hidden="true"
										>
											{initials(student.name)}
										</div>
										<div class="min-w-0 flex-1">
											<div class="text-balance-safe text-sm font-semibold">{student.name}</div>
											<div class="mt-0.5 font-mono text-[11px] text-muted-foreground">
												No present record today
											</div>
										</div>
									</li>
								{/each}
							</ul>
							{#if notRecordedStudents.length > 6}
								<p class="mt-3 text-xs text-muted-foreground">
									+{notRecordedStudents.length - 6} more students not shown.
								</p>
							{/if}
						{/if}
					</div>
				</aside>

				<aside class="surface-panel flex min-h-0 flex-col">
					<div class="panel-header">
						<div>
							<h2 class="text-lg font-black">Recent activity</h2>
							<p class="mt-1 text-sm text-muted-foreground">Latest attendance events</p>
						</div>
						<span class="chip shrink-0">
							{recentEvents.length} shown
						</span>
					</div>

					<div class="panel-body min-h-0 flex-1 overflow-y-auto">
						{#if recentEvents.length === 0}
							<div
								class="rounded-xl border border-dashed border-border bg-surface/45 px-4 py-8 text-center text-sm text-muted-foreground"
								role="status"
							>
								Attendance events will appear here as soon as a card tap or manual log is saved.
							</div>
						{:else}
							<ul class="space-y-2">
								{#each recentEvents as event (event.id)}
									{@const student = studentMap.get(event.studentId)}
									<li class="list-row flex min-w-0 items-center justify-between gap-3 p-3">
										<div class="min-w-0">
											<div class="text-balance-safe text-sm font-semibold">
												{student?.name ?? 'Unknown student'}
											</div>
											<div
												class="mt-0.5 flex flex-wrap gap-x-2 gap-y-1 font-mono text-[11px] text-muted-foreground"
											>
												<span>{fmtDate(event.timestamp)}</span>
												<span aria-hidden="true">/</span>
												<span>{fmtTime(event.timestamp)}</span>
											</div>
										</div>
										<span
											class="rounded-pill bg-primary px-2 py-1 font-mono text-[10px] font-bold text-primary-foreground"
										>
											IN
										</span>
									</li>
								{/each}
							</ul>
						{/if}
					</div>

					<div class="border-t border-border p-4">
						<a
							href={resolve('/records')}
							class="btn btn-secondary control-ring min-h-9 px-3 text-xs text-primary"
						>
							View all records
							<ArrowUpRight class="size-3.5" aria-hidden="true" />
						</a>
					</div>
				</aside>
			</div>
		</section>
	{/if}
</div>

<FeedbackToast message={settingsStore.error} ok={false} />

{#snippet statCard(label: string, value: number | string, detail: string, accent = false)}
	<div class="metric-card {accent ? 'metric-card-accent' : ''}">
		<div class="flex items-start justify-between gap-3">
			<div class="min-w-0">
				<div class="label-mono {accent ? 'text-primary-foreground/80!' : ''}">{label}</div>
				<div class="mt-2 text-4xl leading-none font-semibold tracking-normal">{value}</div>
				<div class="mt-2 text-sm {accent ? 'text-primary-foreground/80' : 'text-muted-foreground'}">
					{detail}
				</div>
			</div>
			<div
				class="metric-icon {accent ? 'border-primary-foreground/20 bg-primary-foreground/10' : ''}"
				aria-hidden="true"
			>
				{#if label === 'Roster'}
					<UsersRound class="size-5" />
				{:else if label === 'Present'}
					<CheckCircle2 class="size-5" />
				{:else if label === 'Absent'}
					<CalendarClock class="size-5" />
				{:else}
					<History class="size-5" />
				{/if}
			</div>
		</div>
	</div>
{/snippet}
