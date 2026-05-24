<script lang="ts">
	import { onMount } from 'svelte';
	import { Grid2X2, List, Search, ScanLine } from 'lucide-svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
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

	let log = $state<LogLine[]>([]);
	let students = $state<Student[]>([]);
	let classes = $state<Class[]>([]);
	let events = $state<AttendanceEvent[]>([]);
	let selectedClassId = $state<string>('');
	let manualViewMode = $state<ManualViewMode>('boxes');

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
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;
	let isProcessing = $state(false);
	let isClosingDay = $state(false);
	let lastEventId = $state<string | null>(null);
	let undoTimer: ReturnType<typeof setTimeout> | null = null;

	onMount(() => {
		(async () => {
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
		})();
	});

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
		manualStudents.filter((student) => getLastEventToday(student)?.type === 'in').length
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

	function getLastEventToday(student: Student) {
		return todayEvents
			.filter((event) => event.studentId === student.id)
			.sort((a, b) => eventTime(b) - eventTime(a))[0];
	}

	function getNextAttendanceType(student: Student): AttendanceType | null {
		const last = getLastEventToday(student);
		if (!last) return 'in';
		return null;
	}

	function getStudentStatus(student: Student) {
		const last = getLastEventToday(student);
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
		return classes.find((classItem) => classItem.id === student.classId)?.name ?? 'No class';
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
		}
	}

	async function logForStudent(student: Student, forcedType?: AttendanceType) {
		const type = forcedType ?? getNextAttendanceType(student);
		if (!type) {
			toast(`${student.name} already recorded today`, false);
			return;
		}

		const studentClass = classes.find((c) => c.id === student.classId) || currentClass;
		const ts = Date.now();

		if (!isWithinClassHours(studentClass, ts)) {
			toast('Not within class hours - attendance not allowed', false);
			return;
		}

		const isLate = type === 'in' && checkLate(studentClass, ts);

		try {
			const createdEvent = await addEvent({
				studentId: student.id,
				classId: student.classId || selectedClassId || sessionClass?.id || undefined,
				type,
				note: isLate ? 'Late' : undefined
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
			handleAttendanceError(err, student.name);
		}
	}

	function handleAttendanceError(err: unknown, name?: string) {
		const message = err instanceof Error ? err.message : String(err);
		if (message.includes('duplicate attendance') || message.includes('already recorded')) {
			toast(name ? `${name} already recorded today` : 'Already recorded today', false);
			return;
		}
		toast(`Error: ${message}`, false);
	}

	async function markStudent(student: Student, action: AttendanceType | null, closePicker = false) {
		if (!action) {
			toast(`${student.name} already recorded today`, false);
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
			(student) => getLastEventToday(student)?.type === 'in'
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
					class="inline-flex h-10 items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
				>
					{isClosingDay ? 'Closing...' : 'End Session'}
				</button>
			</div>
		{/snippet}
	</PageHeader>

	{#if settingsPending}
		<div class="px-6 py-12 text-sm text-muted-foreground md:px-12">Loading attendance...</div>
	{:else if isCardReaderMode}
		<section
			class="grid h-[calc(100vh-16rem)] min-h-[560px] gap-5 px-6 py-6 md:px-12 xl:grid-cols-[minmax(0,1fr)_360px] 2xl:grid-cols-[minmax(0,1fr)_400px]"
		>
			<div
				class="relative flex min-h-0 items-center justify-center overflow-hidden rounded-2xl border border-border bg-surface p-8"
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
					<div class="relative w-full max-w-md text-center">
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

						<form
							onsubmit={(e) => {
								e.preventDefault();
								handleCardSubmit(cardInput);
							}}
							class="mx-auto mt-6 max-w-sm"
						>
							<input
								type="text"
								bind:value={cardInput}
								placeholder="Tap card or enter serial..."
								class="h-12 w-full rounded-md border border-border bg-background px-4 text-center font-mono text-sm focus:ring-2 focus:ring-primary focus:outline-none"
							/>
						</form>
					</div>
				{/if}
			</div>

			<div class="flex min-h-0 flex-col rounded-2xl border border-border bg-card p-5">
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
									<div class="min-w-0">
										<div class="truncate font-medium">{line.studentName}</div>
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
		<section
			class="grid h-[calc(100vh-16rem)] min-h-[560px] gap-5 px-6 py-6 md:px-12 xl:grid-cols-[minmax(0,1fr)_340px]"
		>
			<div class="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-border bg-card">
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
							class="grid h-full auto-rows-[84px] grid-cols-[repeat(auto-fill,minmax(112px,1fr))] gap-2 overflow-y-auto pr-1"
						>
							{#each manualStudents as student (student.id)}
								{@const action = getNextAttendanceType(student)}
								{@const status = getStudentStatus(student)}
								<button
									type="button"
									title={`${student.name} - ${status.label}`}
									disabled={!action || isProcessing}
									onclick={() => markStudent(student, action)}
									class="group flex h-full min-w-0 flex-col justify-between rounded-xl border p-2 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-65 {action ===
									'in'
										? 'border-border bg-background hover:border-primary hover:bg-primary/10'
										: 'border-border bg-surface/80 text-muted-foreground'}"
								>
									<span class="flex min-w-0 items-start gap-2">
										<span
											class="grid size-8 shrink-0 place-items-center rounded-lg border text-[11px] font-bold {status.tone ===
											'in'
												? 'border-primary/30 bg-primary text-primary-foreground'
												: 'border-border bg-surface text-foreground'}"
										>
											{getStudentInitials(student.name)}
										</span>
										<span class="min-w-0">
											<span class="line-clamp-2 text-xs leading-tight font-semibold break-words">
												{student.name}
											</span>
										</span>
									</span>
									<span class="flex items-center justify-between gap-2">
										<span class="truncate text-[10px] text-muted-foreground">
											{selectedClassId ? status.label : getStudentClassName(student)}
										</span>
										<span
											class="label-mono shrink-0 text-[10px] font-bold {action === 'in'
												? 'text-primary'
												: 'text-muted-foreground'}"
										>
											{action === 'in' ? 'IN' : 'REC'}
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
									<li class="flex items-center justify-between gap-4 px-4 py-3 hover:bg-surface/50">
										<div class="flex min-w-0 items-center gap-3">
											<div
												class="grid size-10 shrink-0 place-items-center rounded-lg border border-border bg-surface text-xs font-bold"
											>
												{getStudentInitials(student.name)}
											</div>
											<div class="min-w-0">
												<div class="truncate text-base font-semibold">{student.name}</div>
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
											disabled={!action || isProcessing}
											onclick={() => markStudent(student, action)}
											class="min-w-28 rounded-pill px-4 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 {action ===
											'in'
												? 'bg-primary text-primary-foreground hover:bg-accent'
												: 'border border-border bg-surface text-muted-foreground'}"
										>
											{action === 'in' ? 'Record' : 'Recorded'}
										</button>
									</li>
								{/each}
							</ul>
						</div>
					{/if}
				</div>
			</div>

			<div class="flex min-h-0 flex-col rounded-2xl border border-border bg-card p-5">
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
									<div class="min-w-0">
										<div class="truncate font-medium">{studentName(event.studentId)}</div>
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
			class="pointer-events-auto flex items-center gap-4 rounded-3xl border px-8 py-5 shadow-2xl
				{lastResult.ok
				? 'border-border bg-background text-foreground'
				: 'border-destructive bg-destructive text-destructive-foreground'}"
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
			<div>
				<div class="text-xl font-bold">{lastResult.name}</div>
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
	on:close={() => (pickerOpen = false)}
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
						disabled={!action || isProcessing}
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
							{action === 'in' ? 'RECORD' : 'RECORDED'}
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

{#if toastMessage}
	<div
		class="fixed top-12 right-6 z-60 flex items-center gap-3 rounded-xl border px-4 py-3 text-sm font-medium shadow-lg
			{toastOk
			? 'border-border bg-background text-foreground'
			: 'border-destructive/40 bg-destructive/10 text-destructive'}"
		role="status"
		aria-live="polite"
	>
		<span>{toastMessage}</span>
		{#if lastEventId && toastOk}
			<button
				onclick={undoLast}
				class="rounded-md bg-primary/10 px-2 py-1 text-xs font-semibold text-primary transition-colors hover:bg-primary/20"
			>
				UNDO
			</button>
		{/if}
	</div>
{/if}

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
