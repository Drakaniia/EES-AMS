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
	} from '$lib/api';
	import { fmtDate, fmtTime } from '$lib/csv';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import {
		ArrowUpRight,
		CalendarClock,
		CalendarDays,
		CheckCircle2,
		History,
		ScanLine,
		UsersRound
	} from 'lucide-svelte';
	import {
		getActiveClass,
		eventTime,
		initials,
		attendanceHref
	} from '../../dashboard-state.svelte';
	import { formatAttendanceDate } from '../attendance-state.svelte';

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
			goto(resolve('/attendance/overview'), { replaceState: true });
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
	const todayLabel = $derived.by(() => {
		const formatted = formatAttendanceDate(todayKey);
		return `Today \u2022 ${formatted}`;
	});
	const todayEvents = $derived(events.filter((event) => fmtDate(event.timestamp) === today));
	const studentMap = $derived(new SvelteMap(students.map((student) => [student.id, student])));
	const activeClass = $derived(getActiveClass(classes));
	const assignedClass = $derived(classes[0] ?? null);

	const classStudents = $derived.by(() => {
		if (!assignedClass) return students;
		return students.filter((student) => !student.classId || student.classId === assignedClass.id);
	});
	const relevantTodayEvents = $derived.by(() => {
		if (!assignedClass) return todayEvents;
		const classStudentIds = new Set(classStudents.map((student) => student.id));
		return todayEvents.filter((event) => {
			const student = studentMap.get(event.studentId);
			return (
				event.classId === assignedClass.id ||
				student?.classId === assignedClass.id ||
				classStudentIds.has(event.studentId)
			);
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
	const absentStudentIds = $derived.by(
		() =>
			new Set(
				relevantTodayEvents
					.filter((event) => event.type === 'absent')
					.map((event) => event.studentId)
			)
	);
	// Absence is an explicit mark; students with no record at all are merely
	// pending (present by default) and are never counted as absent.
	const absentStudents = $derived.by(() =>
		classStudents.filter((student) => absentStudentIds.has(student.id))
	);
	const pendingStudents = $derived.by(() =>
		classStudents.filter(
			(student) => !recordedStudentIds.has(student.id) && !absentStudentIds.has(student.id)
		)
	);
	const absentCount = $derived(absentStudents.length);
	const pendingCount = $derived(pendingStudents.length);
	const attendanceRate = $derived(
		classStudents.length === 0 ? 0 : Math.round((checkedIn.length / classStudents.length) * 100)
	);
	const recentEvents = $derived.by(() =>
		[...events].sort((a, b) => eventTime(b) - eventTime(a)).slice(0, 8)
	);

	const dynamicTitle = $derived.by(() => {
		if (assignedClass) return `${assignedClass.name} Overview`;
		return 'Attendance Overview';
	});

	const dynamicDescription = $derived.by(() => {
		if (assignedClass) {
			const room = assignedClass.room ? `Room ${assignedClass.room} / ` : '';
			return `${room}${assignedClass.dayStart} - ${assignedClass.dayEnd} / Tracking today's attendance.`;
		}
		return 'No class is configured yet. Add the class schedule and student list to begin tracking attendance.';
	});

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
	<title>Daily Overview - Attendance System</title>
	<meta name="description" content="Today's attendance at a glance." />
</svelte:head>

<PageHeader category="Attendance" title={dynamicTitle} description={dynamicDescription}>
	{#snippet actions()}
		<span
			class="inline-flex h-9 items-center gap-2 px-1 text-[12.5px] font-medium text-muted-foreground"
			role="status"
			title={todayKey}
		>
			<CalendarDays class="size-3.5" aria-hidden="true" strokeWidth={1.9} />
			<span>{todayLabel}</span>
		</span>
		<a href={resolve('/students')} class="btn btn-secondary">
			<UsersRound class="size-4" aria-hidden="true" strokeWidth={1.9} />
			Manage students
		</a>
		<a href={resolve(attendanceHref(assignedClass?.id))} class="btn btn-primary">
			<ScanLine class="size-4" aria-hidden="true" strokeWidth={1.9} />
			Take attendance
		</a>
	{/snippet}
</PageHeader>

<div class="page-frame flex min-h-0 flex-1 flex-col gap-5">
	{#if sessionSummary}
		<div
			class="notice-banner flex flex-col gap-3 p-3.5 sm:flex-row sm:items-center sm:justify-between"
			role="status"
			aria-live="polite"
		>
			<div class="flex min-w-0 items-center gap-2.5">
				<CheckCircle2
					class="size-[18px] shrink-0 text-primary"
					aria-hidden="true"
					strokeWidth={2}
				/>
				<div class="min-w-0">
					<h2 class="text-balance-safe text-[13px] font-semibold">
						Session complete: {sessionSummary.className}
					</h2>
					<p class="text-balance-safe mt-0.5 text-[12px] text-muted-foreground">
						{sessionSummary.summary}
					</p>
				</div>
			</div>
			<button
				type="button"
				onclick={() => (sessionSummary = null)}
				class="btn btn-secondary btn-sm w-fit"
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
			{@render statCard('Students', classStudents.length, 'Assigned class')}
			{@render statCard('Present', checkedIn.length, 'Marked present today', true)}
			{@render statCard('Absent', absentCount, 'Marked absent today')}
			{@render statCard('Pending', pendingCount, 'No record yet')}
		</section>

		<section class="surface-panel p-5" aria-label="Attendance completion">
			<div class="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
				<div class="min-w-0">
					<p class="label-mono">Today’s completion</p>
					<h2
						class="mt-1.5 text-[17px] leading-tight font-semibold tracking-[-0.015em] text-foreground"
					>
						{checkedIn.length} present · {absentCount} absent · {pendingCount} pending
					</h2>
					<p class="text-balance-safe mt-1 text-[12.5px] leading-5 text-muted-foreground">
						{assignedClass
							? `Tracking ${assignedClass.name} for today.`
							: 'Tracking all students with attendance activity for today.'}
					</p>
				</div>
				<div class="w-full min-w-0 md:max-w-sm">
					<div class="mb-2 flex items-center justify-between gap-3 text-sm">
						<span class="font-semibold text-foreground">{attendanceRate}% complete</span>
						<span class="text-muted-foreground">{classStudents.length} total</span>
					</div>
					<div class="progress-track" aria-hidden="true">
						<div class="progress-fill" style={`width: ${attendanceRate}%`}></div>
					</div>
				</div>
			</div>
		</section>

		<section
			class="flex min-h-0 flex-1 gap-5 xl:grid xl:grid-cols-[minmax(320px,0.82fr)_minmax(0,1.18fr)]"
		>
			<aside class="surface-panel flex min-h-0 flex-col">
				<div class="panel-header">
					<div class="min-w-0">
						<h2 class="text-[15px] font-semibold tracking-[-0.012em]">Absent today</h2>
						<p class="mt-0.5 text-[12px] text-muted-foreground">Explicitly marked absent</p>
					</div>
					<span class="chip shrink-0">{absentCount} absent</span>
				</div>

				<div class="panel-body min-h-0 flex-1 overflow-y-auto">
					{#if absentStudents.length === 0}
						<div
							class="rounded-xl border border-dashed border-border bg-surface/45 px-4 py-6 text-center"
							role="status"
						>
							<CheckCircle2 class="mx-auto size-6 text-primary" aria-hidden="true" />
							<p class="mt-2 text-sm font-semibold">No students marked absent today.</p>
						</div>
					{:else}
						<ul class="space-y-2">
							{#each absentStudents.slice(0, 6) as student (student.id)}
								<li class="list-row flex min-w-0 items-center gap-3 p-3">
									<div
										class="grid size-9 shrink-0 place-items-center rounded-lg border border-red-500/30 bg-red-50 font-mono text-[11px] font-bold text-red-700"
										aria-hidden="true"
									>
										{initials(student.name)}
									</div>
									<div class="min-w-0 flex-1">
										<div class="text-balance-safe text-sm font-semibold">{student.name}</div>
										<div class="mt-0.5 font-mono text-[11px] text-red-700/80">
											Marked absent today
										</div>
									</div>
								</li>
							{/each}
						</ul>
						{#if absentStudents.length > 6}
							<p class="mt-3 text-xs text-muted-foreground">
								+{absentStudents.length - 6} more students not shown.
							</p>
						{/if}
					{/if}
				</div>
			</aside>

			<aside class="surface-panel flex min-h-0 flex-col">
				<div class="panel-header">
					<div>
						<h2 class="text-[15px] font-semibold tracking-[-0.012em]">Recent activity</h2>
						<p class="mt-0.5 text-[12px] text-muted-foreground">Latest attendance events</p>
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
							Attendance events will appear here as soon as attendance is recorded.
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
										class="rounded-md px-1.5 py-0.5 text-[10.5px] font-semibold {event.type ===
										'absent'
											? 'bg-destructive/10 text-destructive'
											: 'bg-emerald-500/10 text-emerald-800 dark:text-emerald-300'}"
									>
										{event.type === 'absent' ? 'Absent' : 'Present'}
									</span>
								</li>
							{/each}
						</ul>
					{/if}
				</div>

				<div class="border-t border-border p-4">
					<a
						href={resolve('/attendance/logs')}
						class="btn btn-secondary control-ring min-h-9 px-3 text-xs text-primary"
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
	<div class="metric-card {accent ? 'metric-card-accent' : ''}">
		<div class="flex items-start justify-between gap-3">
			<div class="min-w-0">
				<p class="text-[11.5px] font-medium text-muted-foreground">{label}</p>
				<div class="metric-value mt-2">{value}</div>
				<p class="mt-1.5 text-[11.5px] text-muted-foreground">{detail}</p>
			</div>
			<div class="metric-icon" aria-hidden="true">
				{#if label === 'Students'}
					<UsersRound class="size-4" strokeWidth={1.9} />
				{:else if label === 'Present'}
					<CheckCircle2 class="size-4 text-primary" strokeWidth={1.9} />
				{:else if label === 'Absent'}
					<CalendarClock class="size-4" strokeWidth={1.9} />
				{:else}
					<History class="size-4" strokeWidth={1.9} />
				{/if}
			</div>
		</div>
	</div>
{/snippet}
