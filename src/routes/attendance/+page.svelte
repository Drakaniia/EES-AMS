<script lang="ts">
	import { onMount } from 'svelte';
	import { Grid2X2, List, Search, ScanLine, ShieldAlert } from 'lucide-svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import TaskProgress from '$lib/components/ui/TaskProgress.svelte';
	import {
		listStudents,
		listClasses,
		listEvents,
		findStudentByCard,
		addEvent,
		deleteEvent,
		closeSf2AttendanceDay,
		type AttendanceEvent,
		type AttendanceType,
		type Student,
		type Class
	} from '$lib/db-rust';
	import { fmtDate, fmtTime } from '$lib/csv';
	import { settingsStore } from '$lib/stores/settings.svelte';

	type LogLine = {
		id: string;
		studentName: string;
		type: AttendanceType | 'error';
		isLate?: boolean;
		message: string;
		timestamp: number | string;
	};

	type ManualViewMode = 'boxes' | 'list';
	type OverrideTarget = {
		student: Student;
		type: AttendanceType;
		timestamp: number;
		classId?: string;
		className: string;
		sessionKey: string;
		message: string;
		isLate: boolean;
	};
	type LogOptions = {
		overrideReason?: string;
		timestamp?: number;
	};

	let log = $state<LogLine[]>([]);
	let students = $state<Student[]>([]);
	let classes = $state<Class[]>([]);
	let events = $state<AttendanceEvent[]>([]);
	let selectedClassId = $state<string>('');
	let manualViewMode = $state<ManualViewMode>('boxes');
	let loading = $state(true);
	let loadError = $state<string | null>(null);

	let pickerOpen = $state(false);
	let pickerQuery = $state('');
	let rosterQuery = $state('');
	let lastResult = $state<{
		ok: boolean;
		name: string;
		type: AttendanceType;
		time: number;
		isLate?: boolean;
		eventId?: string;
	} | null>(null);

	let cardInput = $state('');
	let cardInputElement = $state<HTMLInputElement | null>(null);
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;
	let isProcessing = $state(false);
	let isClosingDay = $state(false);
	let lastEventId = $state<string | null>(null);
	let undoTimer: ReturnType<typeof setTimeout> | null = null;
	let lastScan = $state<{ serial: string; timestamp: number } | null>(null);
	let overrideTarget = $state<OverrideTarget | null>(null);
	let overrideReason = $state('');
	let isOverrideSaving = $state(false);

	onMount(async () => {
		await loadInitial();
	});

	$effect(() => {
		if (isCardReaderMode && !pickerOpen && cardInputElement && !loading && !loadError) {
			cardInputElement.focus();
		}
	});

	async function loadInitial() {
		loading = true;
		loadError = null;
		try {
			await Promise.all([reload(), settingsStore.load()]);

			const requestedClassId = page.url.searchParams.get('classId');
			if (requestedClassId && classes.some((c) => c.id === requestedClassId)) {
				selectedClassId = requestedClassId;
			} else {
				const active = getActiveClass();
				if (active) selectedClassId = active.id;
			}

			if (page.url.searchParams.get('manual') === 'true') {
				pickerQuery = '';
				pickerOpen = true;
			}
		} catch (error) {
			loadError = error instanceof Error ? error.message : 'Attendance data could not be loaded.';
		} finally {
			loading = false;
		}
	}

	async function reload() {
		const [s, c, e] = await Promise.all([listStudents(), listClasses(), listEvents()]);
		students = s;
		classes = c;
		events = e;
	}

	const settingsPending = $derived(settingsStore.loading && !settingsStore.settings);
	const attendanceMode = $derived(settingsStore.settings?.attendanceMode ?? 'manual');
	const isCardReaderMode = $derived(attendanceMode === 'card_reader');
	const currentClass = $derived(classes.find((c) => c.id === selectedClassId));
	const today = $derived(fmtDate(Date.now()));
	const todayEvents = $derived(events.filter((event) => fmtDate(event.timestamp) === today));

	const manualStudents = $derived.by(() => {
		const query = rosterQuery.trim().toLowerCase();
		return students
			.filter((student) => {
				const matchesClass = !selectedClassId || student.classId === selectedClassId;
				const matchesQuery = !query || student.name.toLowerCase().includes(query);
				return matchesClass && matchesQuery;
			})
			.sort((a, b) => a.name.localeCompare(b.name));
	});

	const pickerStudents = $derived.by(() => {
		const query = pickerQuery.trim().toLowerCase();
		return students
			.filter((student) => {
				const matchesQuery = !query || student.name.toLowerCase().includes(query);
				const matchesClass = !selectedClassId || student.classId === selectedClassId;
				return matchesQuery && matchesClass;
			})
			.sort((a, b) => a.name.localeCompare(b.name));
	});

	const recentActivity = $derived.by(() =>
		todayEvents
			.filter((event) => {
				const student = students.find((s) => s.id === event.studentId);
				return (
					!selectedClassId ||
					event.classId === selectedClassId ||
					student?.classId === selectedClassId
				);
			})
			.sort((a, b) => eventTime(b) - eventTime(a))
			.slice(0, 14)
	);

	const recordedCount = $derived(
		manualStudents.filter((student) => getLastEventForSession(student)?.type === 'in').length
	);
	const pendingCount = $derived(manualStudents.length - recordedCount);

	const activeClass = $derived(getActiveClass());
	const sessionClass = $derived.by(() => {
		if (currentClass) return currentClass;
		if (isCardReaderMode) return activeClass ?? undefined;
		return undefined;
	});
	const timeOfDay = $derived(getTimeOfDay());
	const pageCategory = $derived(
		settingsPending ? 'Attendance' : isCardReaderMode ? 'Tap Mode' : 'Manual Mode'
	);
	const dynamicTitle = $derived.by(() => {
		if (settingsPending) return 'Attendance';
		if (isCardReaderMode) {
			if (sessionClass) return `${timeOfDay} ${sessionClass.name} Attendance`;
			return 'Live Session';
		}
		if (currentClass) return `${currentClass.name} Attendance`;
		return 'Manual Attendance';
	});

	const dynamicDescription = $derived.by(() => {
		if (settingsPending) return 'Loading attendance mode and class roster.';
		if (isCardReaderMode) {
			if (sessionClass) {
				return `Recording attendance for ${sessionClass.name} (${sessionClass.dayStart} - ${sessionClass.dayEnd})`;
			}
			return 'Active monitoring of student attendance.';
		}

		if (currentClass) {
			return `Name-only attendance for ${currentClass.name} (${currentClass.dayStart} - ${currentClass.dayEnd})`;
		}
		return 'Choose names from the class list and record attendance without card serials.';
	});

	function getTimeOfDay(): 'Morning' | 'Afternoon' {
		return new Date().getHours() < 12 ? 'Morning' : 'Afternoon';
	}

	function getActiveClass(): Class | null {
		const now = new Date();
		const currentTime = now.getHours() * 60 + now.getMinutes();
		const currentDay = now.getDay();

		for (const cls of classes) {
			if (cls.days && !cls.days.includes(currentDay)) continue;

			const [startHour, startMin] = cls.dayStart.split(':').map(Number);
			const [endHour, endMin] = cls.dayEnd.split(':').map(Number);
			const startTime = startHour * 60 + startMin;
			const endTime = endHour * 60 + endMin;

			if (currentTime >= startTime && currentTime <= endTime) return cls;
		}
		return null;
	}

	function eventTime(event: AttendanceEvent) {
		return typeof event.timestamp === 'string'
			? new Date(event.timestamp).getTime()
			: event.timestamp;
	}

	function studentName(studentId: string) {
		return students.find((student) => student.id === studentId)?.name ?? 'Unknown student';
	}

	function getStudentClass(student: Student) {
		return classes.find((classItem) => classItem.id === student.classId);
	}

	function getAttendanceClass(student: Student) {
		return currentClass ?? (isCardReaderMode ? activeClass : undefined) ?? getStudentClass(student);
	}

	function getSessionSegment(classObj: Class | undefined, timestamp: number) {
		if (!classObj?.sessions || classObj.sessions.length <= 1) return 'day';

		const now = new Date(timestamp);
		const timeStr = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
		const session = classObj.sessions.find(
			(item) => timeStr >= item.startTime && timeStr <= item.endTime
		);

		return (session?.name || 'off-schedule')
			.trim()
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-|-$/g, '');
	}

	function getSessionKey(classObj: Class | undefined, timestamp: number) {
		const classKey = classObj?.id || 'unassigned';
		const segment = getSessionSegment(classObj, timestamp) || 'day';
		return `${fmtDate(timestamp)}|${classKey}|${segment}`;
	}

	function getAttendanceDraft(student: Student, timestamp = Date.now()) {
		const classObj = getAttendanceClass(student);
		const classId = classObj?.id || selectedClassId || student.classId || undefined;
		const sessionKey = getSessionKey(classObj, timestamp);

		return {
			classObj,
			classId,
			sessionKey,
			isLate: checkLate(classObj, timestamp),
			className: classObj?.name ?? 'Unassigned class'
		};
	}

	function matchesCurrentSession(event: AttendanceEvent, student: Student, timestamp = Date.now()) {
		const draft = getAttendanceDraft(student, timestamp);
		if (event.sessionKey) return event.sessionKey === draft.sessionKey;
		const eventClassId = event.classId || student.classId || 'unassigned';
		return (
			fmtDate(event.timestamp) === fmtDate(timestamp) &&
			eventClassId === (draft.classId || 'unassigned')
		);
	}

	function getLastEventForSession(student: Student) {
		return todayEvents
			.filter((event) => event.studentId === student.id && matchesCurrentSession(event, student))
			.sort((a, b) => eventTime(b) - eventTime(a))[0];
	}

	function getNextAttendanceType(student: Student): AttendanceType | null {
		const last = getLastEventForSession(student);
		if (!last) return 'in';
		return null;
	}

	function getStudentStatus(student: Student) {
		const last = getLastEventForSession(student);
		if (!last) return { label: 'Not recorded', tone: 'idle' };
		return { label: `Recorded ${fmtTime(last.timestamp)}`, tone: 'in' };
	}

	function getStudentInitials(name: string) {
		const initials = name
			.split(/\s+/)
			.filter(Boolean)
			.slice(0, 2)
			.map((part) => part[0]?.toUpperCase())
			.join('');

		return initials || 'ST';
	}

	function getStudentClassName(student: Student) {
		return getStudentClass(student)?.name ?? 'No class';
	}

	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	async function undoLast() {
		if (!lastEventId || !lastResult) return;

		try {
			await deleteEvent(lastEventId);
			const eventIdToRemove = lastEventId;
			log = log.filter((line) => line.id !== eventIdToRemove);
			events = events.filter((event) => event.id !== eventIdToRemove);
			toast(`Undid ${lastResult.name} attendance`);
		} catch {
			toast('Failed to undo last action', false);
		} finally {
			lastEventId = null;
			lastResult = null;
			if (undoTimer) clearTimeout(undoTimer);
		}
	}

	function checkLate(classObj: Class | undefined, timestamp: number): boolean {
		if (!classObj) return false;

		const now = new Date(timestamp);
		const timeStr = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
		let lateAfter = classObj.lateAfter;

		if (classObj.sessions && classObj.sessions.length > 0) {
			for (const session of classObj.sessions) {
				if (timeStr >= session.startTime && timeStr <= session.endTime) {
					lateAfter = session.lateAfter;
					break;
				}
			}
		}

		if (!lateAfter) return false;
		const [h, m] = lateAfter.split(':').map(Number);
		const lateTime = new Date(now.getFullYear(), now.getMonth(), now.getDate(), h, m, 0, 0);
		return now > lateTime;
	}

	function isWithinClassHours(classObj: Class | undefined, timestamp: number): boolean {
		if (!classObj) return false;

		const now = new Date(timestamp);
		const timeStr = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

		if (classObj.sessions && classObj.sessions.length > 0) {
			for (const session of classObj.sessions) {
				if (timeStr >= session.startTime && timeStr <= session.endTime) return true;
			}
			return false;
		}

		return timeStr >= classObj.dayStart && timeStr <= classObj.dayEnd;
	}

	async function handleCardSubmit(serial: string) {
		const trimmed = serial.trim();
		if (!trimmed) return;

		if (isProcessing) {
			toast('Please wait - processing previous tap', false);
			return;
		}

		const now = Date.now();
		if (lastScan && lastScan.serial === trimmed && now - lastScan.timestamp < 2500) {
			cardInput = '';
			toast('Duplicate card tap ignored - wait a moment before scanning again', false);
			cardInputElement?.focus();
			return;
		}

		lastScan = { serial: trimmed, timestamp: now };
		cardInput = '';
		isProcessing = true;

		try {
			const student = await findStudentByCard(trimmed);
			if (!student) {
				toast('Unknown card - not paired to any student', false);
				return;
			}
			await logForStudent(student);
		} catch (err: unknown) {
			handleAttendanceError(err);
		} finally {
			isProcessing = false;
			cardInputElement?.focus();
		}
	}

	function openOverride(student: Student, type: AttendanceType, message: string) {
		const timestamp = Date.now();
		const draft = getAttendanceDraft(student, timestamp);
		overrideTarget = {
			student,
			type,
			timestamp,
			classId: draft.classId,
			className: draft.className,
			sessionKey: draft.sessionKey,
			message,
			isLate: draft.isLate
		};
		overrideReason = '';
		pickerOpen = false;
	}

	async function submitOverride() {
		if (!overrideTarget || isOverrideSaving) return;

		const reason = overrideReason.trim();
		if (reason.length < 3) {
			toast('Override reason is required', false);
			return;
		}

		isOverrideSaving = true;
		isProcessing = true;
		try {
			await logForStudent(overrideTarget.student, overrideTarget.type, {
				overrideReason: reason,
				timestamp: overrideTarget.timestamp
			});
			overrideTarget = null;
			overrideReason = '';
		} finally {
			isOverrideSaving = false;
			isProcessing = false;
		}
	}

	async function logForStudent(
		student: Student,
		forcedType?: AttendanceType,
		options: LogOptions = {}
	) {
		const type = forcedType ?? getNextAttendanceType(student);
		if (!type) {
			openOverride(student, 'in', `${student.name} is already recorded for this session.`);
			return;
		}

		const ts = options.timestamp ?? Date.now();
		const draft = getAttendanceDraft(student, ts);

		if (!options.overrideReason && !isWithinClassHours(draft.classObj, ts)) {
			openOverride(student, type, 'This attendance is outside the selected class session.');
			return;
		}

		const isLate = type === 'in' && draft.isLate;

		try {
			const createdEvent = await addEvent({
				studentId: student.id,
				classId: draft.classId,
				type,
				note: isLate ? 'Late' : undefined,
				sessionKey: draft.sessionKey,
				overrideReason: options.overrideReason,
				timestamp: new Date(ts).toISOString()
			});

			lastEventId = createdEvent.id;
			events = [createdEvent, ...events];
			lastResult = {
				ok: true,
				name: student.name,
				type,
				time: ts,
				isLate,
				eventId: createdEvent.id
			};
			log = [
				{
					id: createdEvent.id,
					studentName: student.name,
					type,
					isLate,
					message: isLate ? 'Recorded late' : 'Recorded',
					timestamp: ts
				},
				...log
			].slice(0, 30);

			toast(`${student.name} - ${isLate ? 'Late attendance' : 'Recorded'}`, !isLate);
			if (undoTimer) clearTimeout(undoTimer);
			undoTimer = setTimeout(() => {
				lastResult = null;
				lastEventId = null;
			}, 5000);
		} catch (err: unknown) {
			handleAttendanceError(err, student);
		}
	}

	function handleAttendanceError(err: unknown, student?: Student) {
		const message = err instanceof Error ? err.message : String(err);
		if (message.includes('duplicate attendance') || message.includes('already recorded')) {
			if (student) {
				openOverride(student, 'in', `${student.name} is already recorded for this session.`);
				return;
			}
			toast('Already recorded for this session', false);
			return;
		}
		toast(`Error: ${message}`, false);
	}

	async function markStudent(student: Student, action: AttendanceType | null, closePicker = false) {
		if (!action) {
			openOverride(student, 'in', `${student.name} is already recorded for this session.`);
			return;
		}

		if (isProcessing) {
			toast('Please wait - processing previous request', false);
			return;
		}

		isProcessing = true;
		try {
			await logForStudent(student, action);
			if (closePicker) pickerOpen = false;
		} finally {
			isProcessing = false;
		}
	}

	async function endSession() {
		const classObj = sessionClass;
		if (!classObj) {
			goto(resolve('/'));
			return;
		}

		if (
			!confirm(
				`Close attendance for ${classObj.name} today? Missing learners will be treated as absences in the SF2 export.`
			)
		) {
			return;
		}

		isClosingDay = true;
		let closeSummary: string;
		try {
			const summary = await closeSf2AttendanceDay(classObj.id, today);
			closeSummary = `; SF2 day closed with ${summary.absentCount} absent`;
		} catch (error) {
			const msg = error instanceof Error ? error.message : 'Failed to close SF2 day';
			toast(`Failed to close SF2 day: ${msg}`, false);
			isClosingDay = false;
			return;
		}

		const classStudents = students.filter((student) => student.classId === classObj.id);
		const presentCount = classStudents.filter(
			(student) => getLastEventForSession(student)?.type === 'in'
		).length;
		const summary = `${presentCount}/${classStudents.length} students present${closeSummary}`;

		goto(
			resolve(
				`/?sessionEnd=true&summary=${encodeURIComponent(summary)}&className=${encodeURIComponent(classObj.name)}`
			)
		);
	}
</script>

<svelte:head>
	<title>{isCardReaderMode ? 'Live Session' : 'Attendance'} - Attendance System</title>
	<meta name="description" content="Record student attendance." />
</svelte:head>

<AppShell>
	<PageHeader category={pageCategory} title={dynamicTitle} description={dynamicDescription}>
		{#snippet actions()}
			<div class="flex flex-wrap items-center gap-3">
				<select
					aria-label="Class"
					bind:value={selectedClassId}
					class="h-10 min-w-56 rounded-pill border border-border bg-background px-4 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				>
					<option value="">{isCardReaderMode ? 'Auto class' : 'All classes'}</option>
					{#each classes as c (c.id)}
						<option value={c.id}>{c.name}</option>
					{/each}
				</select>

				{#if isCardReaderMode}
					<button
						disabled={classes.length === 0}
						onclick={() => {
							pickerQuery = '';
							pickerOpen = true;
						}}
						class="inline-flex h-10 items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
					>
						Manual log
					</button>
				{/if}

				<button
					onclick={endSession}
					disabled={isClosingDay || !sessionClass}
					class="inline-flex h-10 items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
				>
					{#if isClosingDay}
						<span
							class="size-4 animate-spin rounded-full border-2 border-primary/20 border-t-primary"
							aria-hidden="true"
						></span>
					{/if}
					{isClosingDay ? 'Closing...' : 'End Session'}
				</button>
			</div>
		{/snippet}
	</PageHeader>

	{#if isClosingDay}
		<div class="px-6 pt-4 md:px-12">
			<TaskProgress
				active={isClosingDay}
				title="Closing attendance session"
				description="Writing absences to the SF2 workbook and preparing the session summary."
			/>
		</div>
	{/if}

	{#if loading || settingsPending}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<LoadingBlock rows={3} label="Loading attendance workspace" />
		</div>
	{:else if loadError}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<EmptyState tone="warning" title="Attendance is unavailable" description={loadError}>
				{#snippet actions()}
					<button
						type="button"
						onclick={loadInitial}
						class="control-ring rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-surface"
					>
						Retry
					</button>
				{/snippet}
			</EmptyState>
		</div>
	{:else if isCardReaderMode}
		<section
			class="grid gap-5 px-4 py-5 md:px-8 lg:px-10 xl:grid-cols-[minmax(0,1fr)_360px] 2xl:grid-cols-[minmax(0,1fr)_400px]"
		>
			<div
				class="relative flex min-h-[30rem] items-center justify-center overflow-hidden rounded-2xl border border-border bg-surface p-6 md:p-8"
			>
				<div
					aria-hidden="true"
					class="pointer-events-none absolute inset-0 opacity-50"
					style="background: radial-gradient(60% 60% at 50% 40%, color-mix(in oklab, var(--primary) 22%, transparent), transparent 70%)"
				></div>

				{#if classes.length === 0}
					<div class="relative w-full max-w-md text-center">
						<h3 class="display-lg mb-2">No Classes</h3>
						<p class="mb-8 text-muted-foreground">
							Add a class in Settings before starting a live session.
						</p>
					</div>
				{:else if !sessionClass}
					<div class="relative w-full max-w-md text-center">
						<h3 class="display-lg mb-2">No active class</h3>
						<p class="mb-8 text-muted-foreground">
							Choose a class from the header, or let the active schedule select one.
						</p>
					</div>
				{:else}
					<div class="relative w-full max-w-md text-center" role="status" aria-live="polite">
						<div class="label-mono mb-4 text-primary">
							<span class="inline-block size-2 animate-pulse rounded-full bg-primary align-middle"
							></span> Ready for card taps
						</div>

						<div
							class="mx-auto grid size-36 place-items-center rounded-full border-2 border-primary bg-background shadow-[0_0_30px_-8px_var(--primary)]"
						>
							<ScanLine class="size-16 text-primary" strokeWidth={1.5} />
						</div>

						<h3 class="mt-8 text-4xl font-semibold tracking-normal">Tap a card</h3>
						<p class="mx-auto mt-2 max-w-sm text-sm text-muted-foreground">
							The reader field stays focused for card serials and typed fallback entries.
						</p>
						{#if isProcessing}
							<p class="mt-3 text-sm font-medium text-primary" aria-live="assertive">
								Processing card tap...
							</p>
						{/if}

						<form
							onsubmit={(e) => {
								e.preventDefault();
								handleCardSubmit(cardInput);
							}}
							class="mx-auto mt-6 max-w-sm"
						>
							<label for="card-reader-serial" class="sr-only">Card serial</label>
							<input
								id="card-reader-serial"
								bind:this={cardInputElement}
								type="text"
								bind:value={cardInput}
								placeholder="Tap card or enter serial..."
								autocomplete="off"
								spellcheck="false"
								aria-describedby="card-reader-help"
								disabled={isProcessing}
								class="control-ring h-12 w-full rounded-md border border-border bg-background px-4 text-center font-mono text-sm disabled:cursor-wait disabled:opacity-70"
							/>
							<p id="card-reader-help" class="mt-2 text-xs text-muted-foreground">
								Press Enter after typing a serial manually.
							</p>
						</form>
					</div>
				{/if}
			</div>

			<div class="flex min-h-[30rem] flex-col rounded-2xl border border-border bg-card p-5">
				<div class="mb-4 flex shrink-0 items-start justify-between gap-3">
					<div class="flex flex-col">
						<h3 class="text-lg font-medium">Session log</h3>
						<span class="label-mono text-xs opacity-60">Latest card or manual actions</span>
					</div>
					<span
						class="label-mono rounded-pill border border-border bg-surface px-2 py-1 text-[10px]"
					>
						{log.length} entries
					</span>
				</div>

				<div class="min-h-0 flex-1 overflow-y-auto">
					{#if log.length === 0}
						<div
							class="flex h-full w-full flex-col items-center justify-center rounded-xl border border-dashed border-border p-4 text-center text-sm text-muted-foreground"
						>
							No activity recorded in this session.
						</div>
					{:else}
						<ul class="divide-y divide-border">
							{#each log as line (line.id)}
								<li class="flex items-center justify-between gap-3 py-3">
									<div class="min-w-0 flex-1">
										<div class="leading-snug font-medium break-words">{line.studentName}</div>
										<div class="label-mono">{fmtTime(line.timestamp)}</div>
									</div>
									<div class="flex items-center gap-2">
										{#if line.isLate}
											<span
												class="rounded-pill border border-destructive/20 bg-destructive/10 px-2 py-0.5 font-mono text-[10px] font-bold text-destructive"
											>
												LATE
											</span>
										{/if}
										{@render pill(line.type)}
									</div>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			</div>
		</section>
	{:else}
		<section class="grid gap-5 px-4 py-5 md:px-8 lg:px-10 xl:grid-cols-[minmax(0,1fr)_340px]">
			<div
				class="flex min-h-[34rem] flex-col overflow-hidden rounded-2xl border border-border bg-card"
			>
				<div class="shrink-0 border-b border-border p-5">
					<div class="flex flex-wrap items-start justify-between gap-4">
						<div>
							<h3 class="text-xl font-semibold">Student boxes</h3>
							<p class="mt-1 max-w-xl text-sm text-muted-foreground">
								One click per learner. Boxes show whether attendance has been recorded.
							</p>
						</div>
						<div
							class="grid grid-cols-3 overflow-hidden rounded-xl border border-border bg-surface"
						>
							{@render manualStat('Names', manualStudents.length)}
							{@render manualStat('Recorded', recordedCount)}
							{@render manualStat('Pending', pendingCount)}
						</div>
					</div>
				</div>

				<div class="flex shrink-0 flex-wrap items-center gap-3 border-b border-border p-4">
					<div class="relative min-w-64 flex-1">
						<Search
							class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
						/>
						<label for="name-search" class="sr-only">Find name</label>
						<input
							id="name-search"
							bind:value={rosterQuery}
							placeholder="Search by name..."
							class="h-10 w-full rounded-md border border-border bg-background pr-4 pl-10 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
						/>
					</div>

					<div
						class="flex shrink-0 overflow-hidden rounded-pill border border-border bg-surface p-1"
						role="group"
						aria-label="Attendance roster view"
					>
						<button
							type="button"
							aria-pressed={manualViewMode === 'boxes'}
							onclick={() => (manualViewMode = 'boxes')}
							class="inline-flex h-9 items-center gap-2 rounded-pill px-3 text-sm font-medium transition-colors {manualViewMode ===
							'boxes'
								? 'bg-background text-foreground shadow-sm'
								: 'text-muted-foreground hover:text-foreground'}"
						>
							<Grid2X2 class="size-4" />
							Boxes
						</button>
						<button
							type="button"
							aria-pressed={manualViewMode === 'list'}
							onclick={() => (manualViewMode = 'list')}
							class="inline-flex h-9 items-center gap-2 rounded-pill px-3 text-sm font-medium transition-colors {manualViewMode ===
							'list'
								? 'bg-background text-foreground shadow-sm'
								: 'text-muted-foreground hover:text-foreground'}"
						>
							<List class="size-4" />
							List
						</button>
					</div>
				</div>

				<div class="min-h-0 flex-1 p-4">
					{#if manualStudents.length === 0}
						<div
							class="flex h-full min-h-72 items-center justify-center rounded-xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground"
						>
							No names match this class or search.
						</div>
					{:else if manualViewMode === 'boxes'}
						<div
							class="grid h-full auto-rows-[minmax(116px,auto)] grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-3 overflow-y-auto pr-1"
						>
							{#each manualStudents as student (student.id)}
								{@const action = getNextAttendanceType(student)}
								{@const status = getStudentStatus(student)}
								<button
									type="button"
									title={`${student.name} - ${status.label}`}
									disabled={isProcessing}
									onclick={() => markStudent(student, action)}
									class="group flex min-h-[116px] min-w-0 flex-col justify-between rounded-xl border p-3 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-65 {action ===
									'in'
										? 'border-border bg-background hover:border-primary hover:bg-primary/10'
										: 'border-border bg-surface/80 text-muted-foreground'}"
								>
									<span class="flex min-w-0 items-start gap-2">
										<span
											class="grid size-9 shrink-0 place-items-center rounded-lg border text-[11px] font-bold {status.tone ===
											'in'
												? 'border-primary/30 bg-primary text-primary-foreground'
												: 'border-border bg-surface text-foreground'}"
										>
											{getStudentInitials(student.name)}
										</span>
										<span class="min-w-0 flex-1">
											<span
												class="text-sm leading-snug font-semibold break-words whitespace-normal"
											>
												{student.name}
											</span>
										</span>
									</span>
									<span class="flex items-center justify-between gap-2">
										<span class="min-w-0 text-[10px] leading-snug text-muted-foreground">
											{selectedClassId ? status.label : getStudentClassName(student)}
										</span>
										<span
											class="label-mono shrink-0 text-[10px] font-bold {action === 'in'
												? 'text-primary'
												: 'text-muted-foreground'}"
										>
											{action === 'in' ? 'IN' : 'OVR'}
										</span>
									</span>
								</button>
							{/each}
						</div>
					{:else}
						<div class="h-full overflow-y-auto rounded-xl border border-border">
							<ul class="divide-y divide-border">
								{#each manualStudents as student (student.id)}
									{@const action = getNextAttendanceType(student)}
									{@const status = getStudentStatus(student)}
									<li
										class="flex flex-col gap-3 px-4 py-3 hover:bg-surface/50 sm:flex-row sm:items-center sm:justify-between"
									>
										<div class="flex min-w-0 items-center gap-3">
											<div
												class="grid size-10 shrink-0 place-items-center rounded-lg border border-border bg-surface text-xs font-bold"
											>
												{getStudentInitials(student.name)}
											</div>
											<div class="min-w-0 flex-1">
												<div class="text-base leading-snug font-semibold break-words">
													{student.name}
												</div>
												<div
													class="mt-1 text-xs {status.tone === 'in'
														? 'text-primary'
														: 'text-muted-foreground'}"
												>
													{status.label}
												</div>
											</div>
										</div>
										<button
											disabled={isProcessing}
											onclick={() => markStudent(student, action)}
											class="w-fit min-w-28 rounded-pill px-4 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 {action ===
											'in'
												? 'bg-primary text-primary-foreground hover:bg-accent'
												: 'border border-border bg-surface text-muted-foreground'}"
										>
											{action === 'in' ? 'Record' : 'Override'}
										</button>
									</li>
								{/each}
							</ul>
						</div>
					{/if}
				</div>
			</div>

			<div class="flex min-h-[34rem] flex-col rounded-2xl border border-border bg-card p-5">
				<div class="mb-4 flex shrink-0 items-start justify-between gap-3">
					<div>
						<h3 class="text-lg font-medium">Recent activity</h3>
						<span class="label-mono text-xs opacity-60">Today</span>
					</div>
					<span
						class="label-mono rounded-pill border border-border bg-surface px-2 py-1 text-[10px]"
					>
						{recentActivity.length} events
					</span>
				</div>

				<div class="min-h-0 flex-1 overflow-y-auto">
					{#if recentActivity.length === 0}
						<div
							class="flex h-full w-full flex-col items-center justify-center rounded-xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground"
						>
							No attendance has been recorded today.
						</div>
					{:else}
						<ul class="divide-y divide-border">
							{#each recentActivity as event (event.id)}
								<li class="flex items-center justify-between gap-3 py-3">
									<div class="min-w-0 flex-1">
										<div class="leading-snug font-medium break-words">
											{studentName(event.studentId)}
										</div>
										<div class="label-mono">{fmtTime(event.timestamp)}</div>
									</div>
									{@render pill(event.type)}
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			</div>
		</section>
	{/if}
</AppShell>

{#if lastResult}
	<div class="pointer-events-none fixed inset-x-0 bottom-10 z-50 flex justify-center px-4">
		<div
			class="pointer-events-auto flex max-w-[min(34rem,calc(100vw-2rem))] items-center gap-4 rounded-2xl border px-5 py-4 shadow-2xl md:px-8 md:py-5
				{lastResult.ok
				? 'border-border bg-background text-foreground'
				: 'border-destructive bg-destructive text-destructive-foreground'}"
			role="status"
			aria-live="assertive"
		>
			<div
				class="grid size-12 place-items-center rounded-full
				{lastResult.isLate ? 'bg-destructive/20 text-destructive' : 'bg-primary/20 text-primary'}"
			>
				<svg
					class="size-6"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2.5"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<polyline points="20 6 9 17 4 12" />
				</svg>
			</div>
			<div class="min-w-0">
				<div class="text-balance-safe text-lg leading-tight font-bold md:text-xl">
					{lastResult.name}
				</div>
				<div class="label-mono flex gap-2">
					<span class={lastResult.isLate ? 'font-bold text-destructive' : ''}>
						{lastResult.isLate ? 'LATE' : 'IN'}
					</span>
					<span class="text-muted-foreground">-</span>
					<span class="text-muted-foreground">{fmtTime(lastResult.time)}</span>
				</div>
			</div>
		</div>
	</div>
{/if}

<Dialog
	open={pickerOpen}
	title="Manual log"
	description="Search by name to manually record attendance."
	onClose={() => (pickerOpen = false)}
>
	<input
		placeholder="Search name..."
		bind:value={pickerQuery}
		class="w-full rounded-md border border-border bg-background px-4 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
	/>

	<ul class="max-h-[300px] divide-y divide-border overflow-y-auto rounded-xl border border-border">
		{#if pickerStudents.length === 0}
			<li class="py-10 text-center text-sm text-muted-foreground">
				No names found {selectedClassId ? 'in this class' : ''}.
			</li>
		{:else}
			{#each pickerStudents as student (student.id)}
				{@const action = getNextAttendanceType(student)}
				<li>
					<button
						disabled={isProcessing}
						onclick={() => markStudent(student, action, true)}
						class="flex w-full items-center justify-between px-4 py-3 text-left transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
					>
						<span>
							<span class="block font-medium">{student.name}</span>
							<span class="mt-0.5 block text-xs text-muted-foreground">
								{getStudentStatus(student).label}
							</span>
						</span>
						<span class="label-mono text-xs font-bold text-primary">
							{action === 'in' ? 'RECORD' : 'OVERRIDE'}
						</span>
					</button>
				</li>
			{/each}
		{/if}
	</ul>

	<div class="flex justify-end pt-2">
		<button
			onclick={() => (pickerOpen = false)}
			class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
		>
			Close
		</button>
	</div>
</Dialog>

<Dialog
	open={!!overrideTarget}
	title="Admin override"
	description="Save an exception with a reason for audit history."
	onClose={() => {
		if (!isOverrideSaving) {
			overrideTarget = null;
			overrideReason = '';
		}
	}}
>
	{#if overrideTarget}
		<div class="rounded-xl border border-primary/20 bg-primary/10 p-4 text-sm">
			<div class="flex items-start gap-3">
				<div
					class="grid size-10 shrink-0 place-items-center rounded-lg bg-primary text-primary-foreground"
				>
					<ShieldAlert class="size-5" aria-hidden="true" />
				</div>
				<div class="min-w-0 flex-1">
					<div class="font-semibold">{overrideTarget.student.name}</div>
					<p class="mt-1 text-muted-foreground">{overrideTarget.message}</p>
					<div class="mt-3 flex flex-wrap gap-2 font-mono text-[11px]">
						<span class="rounded-pill border border-border bg-background px-2 py-1">
							{overrideTarget.className}
						</span>
						<span class="rounded-pill border border-border bg-background px-2 py-1">
							{fmtTime(overrideTarget.timestamp)}
						</span>
						{#if overrideTarget.isLate}
							<span
								class="rounded-pill border border-destructive/20 bg-destructive/10 px-2 py-1 text-destructive"
							>
								LATE
							</span>
						{/if}
					</div>
				</div>
			</div>
		</div>

		<div class="space-y-2">
			<label for="override-reason" class="label-mono">Reason</label>
			<textarea
				id="override-reason"
				bind:value={overrideReason}
				rows="4"
				placeholder="Example: substitute class, late arrival, mistaken earlier tap..."
				class="min-h-28 w-full resize-none rounded-md border border-border bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
			></textarea>
		</div>

		<div class="flex justify-end gap-2 pt-2">
			<button
				type="button"
				disabled={isOverrideSaving}
				onclick={() => {
					overrideTarget = null;
					overrideReason = '';
				}}
				class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				Cancel
			</button>
			<button
				type="button"
				disabled={isOverrideSaving || overrideReason.trim().length < 3}
				onclick={submitOverride}
				class="rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
			>
				{isOverrideSaving ? 'Saving...' : 'Save override'}
			</button>
		</div>
	{/if}
</Dialog>

<FeedbackToast
	message={toastMessage}
	ok={toastOk}
	actionLabel={lastEventId && toastOk ? 'Undo' : undefined}
	onAction={undoLast}
	onClose={() => (toastMessage = null)}
/>

{#snippet pill(type: AttendanceType | 'error')}
	<span
		class="shrink-0 rounded-pill px-2 py-1 font-mono text-[10px] font-bold
			{type === 'in'
			? 'bg-primary text-primary-foreground'
			: 'bg-destructive text-destructive-foreground'}"
	>
		{type === 'in' ? 'IN' : 'ERROR'}
	</span>
{/snippet}

{#snippet manualStat(label: string, value: number)}
	<div class="min-w-20 px-4 py-3 text-center">
		<div class="label-mono text-[10px]">{label}</div>
		<div class="mt-1 text-2xl font-semibold">{value}</div>
	</div>
{/snippet}
