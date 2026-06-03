<script lang="ts">
	import { onMount } from 'svelte';
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

	onMount(async () => {
		await reload();

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

	const today = $derived(fmtDate(Date.now()));
	const todayEvents = $derived(events.filter((event) => fmtDate(event.timestamp) === today));
	const studentMap = $derived(new SvelteMap(students.map((student) => [student.id, student])));
	const activeClass = $derived(getActiveClass());
	const nextClass = $derived(getNextClass());
	const isCardReaderMode = $derived(settingsStore.settings?.attendanceMode === 'card_reader');
	const attendanceActionLabel = $derived(
		isCardReaderMode ? 'Start Live Session' : 'Take Attendance'
	);
	const attendanceFallbackLabel = $derived(isCardReaderMode ? 'Manual Entry' : 'Open Attendance');

	const activeClassStudents = $derived(
		activeClass ? students.filter((student) => student.classId === activeClass.id) : students
	);
	const nextClassStudents = $derived(
		nextClass ? students.filter((student) => student.classId === nextClass.cls.id) : []
	);
	const rosterStudents = $derived(activeClass ? activeClassStudents : nextClassStudents);
	const relevantTodayEvents = $derived.by(() => {
		if (!activeClass) return todayEvents;
		return todayEvents.filter((event) => {
			const student = studentMap.get(event.studentId);
			return event.classId === activeClass.id || student?.classId === activeClass.id;
		});
	});
	const checkedIn = $derived.by(() => {
		const lastByStudent = new SvelteMap<string, AttendanceEvent>();
		for (const event of [...relevantTodayEvents].sort((a, b) => eventTime(a) - eventTime(b))) {
			lastByStudent.set(event.studentId, event);
		}
		return [...lastByStudent.values()].filter((event) => event.type === 'in');
	});
	const pendingCount = $derived(Math.max(0, activeClassStudents.length - checkedIn.length));
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
		return 'Attendance Overview';
	});

	const dynamicDescription = $derived.by(() => {
		if (activeClass) {
			const room = activeClass.room ? `Room ${activeClass.room} / ` : '';
			return `${room}${activeClass.dayStart} - ${activeClass.dayEnd} / Session in progress`;
		}
		if (nextClass) {
			return `Next session: ${nextClass.cls.name} starts in ${nextClass.minutes} minutes.`;
		}
		return 'No active class is scheduled right now. Review logs, manage rosters, or prepare the next session.';
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

	function getNextClass(): { cls: Class; minutes: number } | null {
		const now = new Date();
		const currentTime = now.getHours() * 60 + now.getMinutes();
		const currentDay = now.getDay();
		let next: { cls: Class; minutes: number } | null = null;

		for (const classItem of classes) {
			if (classItem.days && !classItem.days.includes(currentDay)) continue;

			const [startHour, startMin] = classItem.dayStart.split(':').map(Number);
			const startTime = startHour * 60 + startMin;

			if (startTime > currentTime) {
				const diff = startTime - currentTime;
				if (!next || diff < next.minutes) next = { cls: classItem, minutes: diff };
			}
		}
		return next;
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
		<a
			href={resolve('/students')}
			class="control-ring inline-flex h-10 items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
		>
			<UsersRound class="size-4" aria-hidden="true" />
			Manage students
		</a>
		<a
			href={resolve(attendanceHref(activeClass?.id))}
			class="control-ring inline-flex h-10 items-center gap-2 rounded-pill border border-primary bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
		>
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

<div class="mx-auto flex w-full max-w-[1600px] flex-col gap-5 px-4 py-5 md:px-8 lg:px-10">
	{#if sessionSummary}
		<div
			class="flex flex-col gap-4 rounded-2xl border border-primary/25 bg-primary/10 p-4 text-primary sm:flex-row sm:items-center sm:justify-between"
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
				class="control-ring w-fit rounded-md border border-primary/20 px-3 py-2 text-sm font-medium hover:bg-primary/10"
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
				<button
					type="button"
					onclick={reload}
					class="control-ring rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-surface"
				>
					Retry
				</button>
			{/snippet}
		</EmptyState>
	{:else}
		<section class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4" aria-label="Today summary">
			{@render statCard('Roster', activeClassStudents.length, 'Students in scope')}
			{@render statCard('Recorded', checkedIn.length, 'Marked present today', true)}
			{@render statCard('Pending', pendingCount, 'Not yet recorded')}
			{@render statCard('Rate', `${attendanceRate}%`, 'Current completion')}
		</section>

		<section class="grid min-h-[26rem] gap-5 xl:grid-cols-[minmax(0,1.15fr)_minmax(340px,0.85fr)]">
			<div class="flex min-h-0 flex-col rounded-2xl border border-border bg-card">
				<div class="flex flex-wrap items-start justify-between gap-3 border-b border-border p-5">
					<div class="min-w-0">
						<h2 class="text-lg font-semibold">
							{activeClass ? 'Session roster' : 'Next class roster'}
						</h2>
						<p class="mt-1 text-sm text-muted-foreground">
							{activeClass
								? `${checkedIn.length} recorded / ${pendingCount} pending`
								: nextClass
									? `${nextClassStudents.length} students for ${nextClass.cls.name}`
									: 'No scheduled class is currently active.'}
						</p>
					</div>
					{#if activeClass}
						<a
							href={resolve(attendanceHref(activeClass.id, true))}
							class="control-ring inline-flex h-9 items-center gap-2 rounded-pill border border-border bg-background px-3 text-xs font-semibold hover:bg-surface"
						>
							{attendanceFallbackLabel}
							<ArrowUpRight class="size-3.5" aria-hidden="true" />
						</a>
					{/if}
				</div>

				<div class="min-h-0 flex-1 overflow-y-auto p-3">
					{#if rosterStudents.length === 0}
						<EmptyState
							title={activeClass ? 'No students assigned to this class' : 'No roster to show'}
							description={activeClass
								? 'Assign students to this class from the Class List before taking attendance.'
								: 'Create a class schedule in Configuration to see upcoming rosters here.'}
						/>
					{:else}
						<ul class="grid gap-2 sm:grid-cols-2 2xl:grid-cols-3">
							{#each rosterStudents as student (student.id)}
								{@const event = lastEventForStudentToday(student)}
								<li
									class="flex min-w-0 items-center gap-3 rounded-xl border border-border bg-background p-3"
								>
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
											{event ? `Recorded ${fmtTime(event.timestamp)}` : 'Pending'}
										</div>
									</div>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			</div>

			<aside class="flex min-h-0 flex-col rounded-2xl border border-border bg-card">
				<div class="flex items-start justify-between gap-3 border-b border-border p-5">
					<div>
						<h2 class="text-lg font-semibold">Recent activity</h2>
						<p class="mt-1 text-sm text-muted-foreground">Latest attendance events</p>
					</div>
					<span
						class="label-mono rounded-pill border border-border bg-surface px-2 py-1 text-[10px]"
					>
						{recentEvents.length} shown
					</span>
				</div>

				<div class="min-h-0 flex-1 overflow-y-auto p-3">
					{#if recentEvents.length === 0}
						<EmptyState
							title="No activity yet"
							description="Attendance events will appear here as soon as a card tap or manual log is saved."
						/>
					{:else}
						<ul class="divide-y divide-border">
							{#each recentEvents as event (event.id)}
								{@const student = studentMap.get(event.studentId)}
								<li class="flex min-w-0 items-center justify-between gap-3 py-3">
									<div class="min-w-0">
										<div class="truncate text-sm font-semibold">
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
						class="control-ring inline-flex h-9 items-center gap-2 rounded-pill border border-border bg-background px-3 text-xs font-semibold text-primary hover:bg-surface"
					>
						View all records
						<ArrowUpRight class="size-3.5" aria-hidden="true" />
					</a>
				</div>
			</aside>
		</section>
	{/if}
</div>

<FeedbackToast message={settingsStore.error} ok={false} />

{#snippet statCard(label: string, value: number | string, detail: string, accent = false)}
	<div
		class="rounded-2xl border p-5 {accent
			? 'border-primary bg-primary text-primary-foreground'
			: 'border-border bg-card'}"
	>
		<div class="flex items-start justify-between gap-3">
			<div class="min-w-0">
				<div class="label-mono {accent ? 'text-primary-foreground/80!' : ''}">{label}</div>
				<div class="mt-2 text-4xl leading-none font-semibold tracking-normal">{value}</div>
				<div class="mt-2 text-sm {accent ? 'text-primary-foreground/80' : 'text-muted-foreground'}">
					{detail}
				</div>
			</div>
			<div
				class="grid size-10 shrink-0 place-items-center rounded-lg border {accent
					? 'border-primary-foreground/20 bg-primary-foreground/10'
					: 'border-border bg-surface text-muted-foreground'}"
				aria-hidden="true"
			>
				{#if label === 'Roster'}
					<UsersRound class="size-5" />
				{:else if label === 'Recorded'}
					<CheckCircle2 class="size-5" />
				{:else if label === 'Pending'}
					<CalendarClock class="size-5" />
				{:else}
					<History class="size-5" />
				{/if}
			</div>
		</div>
	</div>
{/snippet}
