<script lang="ts">
	import { onMount } from 'svelte';
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
		studentNumber?: string;
		type: AttendanceType | 'error';
		isLate?: boolean;
		message: string;
		timestamp: number | string;
	};

	let log = $state<LogLine[]>([]);
	let students = $state<Student[]>([]);
	let classes = $state<Class[]>([]);
	let events = $state<AttendanceEvent[]>([]);
	let selectedClassId = $state<string>('');

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

	const attendanceMode = $derived(settingsStore.settings?.attendanceMode ?? 'manual');
	const isCardReaderMode = $derived(attendanceMode === 'card_reader');
	const currentClass = $derived(classes.find((c) => c.id === selectedClassId));
	const today = $derived(fmtDate(Date.now()));
	const todayEvents = $derived(events.filter((event) => fmtDate(event.timestamp) === today));

	const todayClasses = $derived.by(() => {
		const currentDay = new Date().getDay();
		return classes
			.filter((cls) => cls.days && cls.days.includes(currentDay))
			.sort((a, b) => {
				const [aH, aM] = a.dayStart.split(':').map(Number);
				const [bH, bM] = b.dayStart.split(':').map(Number);
				return aH * 60 + aM - (bH * 60 + bM);
			});
	});

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
				const matchesQuery =
					!query ||
					student.name.toLowerCase().includes(query) ||
					student.studentNumber.toLowerCase().includes(query);
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

	const checkedInCount = $derived(
		manualStudents.filter((student) => getLastEventToday(student)?.type === 'in').length
	);
	const completedCount = $derived(
		manualStudents.filter((student) => getLastEventToday(student)?.type === 'out').length
	);

	const activeClass = $derived(getActiveClass());
	const timeOfDay = $derived(getTimeOfDay());
	const pageCategory = $derived(isCardReaderMode ? 'Tap Mode' : 'Manual Mode');
	const dynamicTitle = $derived.by(() => {
		if (isCardReaderMode) {
			if (activeClass) return `${timeOfDay} ${activeClass.name} Attendance`;
			if (currentClass) return `${currentClass.name} Live Session`;
			return 'Live Session';
		}
		if (currentClass) return `${currentClass.name} Attendance`;
		return 'Manual Attendance';
	});

	const dynamicDescription = $derived.by(() => {
		if (isCardReaderMode) {
			if (activeClass) {
				return `Recording attendance for ${timeOfDay.toLowerCase()} ${activeClass.name} (${activeClass.dayStart} - ${activeClass.dayEnd})`;
			}
			if (currentClass) {
				return `Recording attendance for ${currentClass.name} (${currentClass.dayStart} - ${currentClass.dayEnd})`;
			}
			return 'Active monitoring of student check-ins.';
		}

		if (currentClass) {
			return `Name-only attendance for ${currentClass.name} (${currentClass.dayStart} - ${currentClass.dayEnd})`;
		}
		return 'Choose names from the class list and record attendance without card or ID numbers.';
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
		return last.type === 'in' ? 'out' : null;
	}

	function getStudentStatus(student: Student) {
		const last = getLastEventToday(student);
		if (!last) return { label: 'Not recorded', tone: 'idle' };
		if (last.type === 'in') return { label: `Checked in ${fmtTime(last.timestamp)}`, tone: 'in' };
		return { label: `Completed ${fmtTime(last.timestamp)}`, tone: 'out' };
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
			toast(`Undid ${lastResult.name} ${lastResult.type === 'in' ? 'check-in' : 'check-out'}`);
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
			toast(`${student.name} already completed attendance today`, false);
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
				classId: student.classId || selectedClassId || undefined,
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
					studentNumber: student.studentNumber,
					type,
					isLate,
					message: type === 'in' ? (isLate ? 'Checked in late' : 'Checked in') : 'Checked out',
					timestamp: ts
				},
				...log
			].slice(0, 30);

			toast(
				`${student.name} - ${type === 'in' ? (isLate ? 'Late check-in' : 'Checked in') : 'Checked out'}`,
				!isLate
			);
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
		if (message.includes('duplicate check-in') || message.includes('already checked in')) {
			toast(name ? `${name} already checked in today` : 'Already checked in today', false);
			return;
		}
		toast(`Error: ${message}`, false);
	}

	function endSession() {
		const classObj = classes.find((c) => c.id === selectedClassId);
		if (!classObj) {
			goto(resolve('/'));
			return;
		}

		const classStudents = students.filter((student) => student.classId === selectedClassId);
		const presentCount = classStudents.filter(
			(student) => getLastEventToday(student)?.type === 'in'
		).length;
		const summary = `${presentCount}/${classStudents.length} students present`;

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
						disabled={todayClasses.length === 0 && !selectedClassId}
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
					class="inline-flex h-10 items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
				>
					End Session
				</button>
			</div>
		{/snippet}
	</PageHeader>

	{#if settingsStore.loading && classes.length === 0}
		<div class="px-6 py-12 text-sm text-muted-foreground md:px-12">Loading attendance...</div>
	{:else if isCardReaderMode}
		<section
			class="grid min-h-[calc(100vh-28rem)] gap-8 px-6 py-10 md:px-12 lg:grid-cols-[1.2fr_1fr]"
		>
			<div
				class="relative flex min-h-[420px] items-center justify-center overflow-hidden rounded-3xl border border-border bg-surface p-10"
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
				{:else if !selectedClassId}
					<div class="relative w-full max-w-md text-center">
						<h3 class="display-lg mb-2">Select a class</h3>
						<p class="mb-8 text-muted-foreground">
							Choose a class from the header to start accepting card taps.
						</p>
					</div>
				{:else}
					<div class="relative w-full max-w-md text-center">
						<div class="label-mono mb-4 text-primary">
							<span class="animate-pulse">●</span> Ready for card taps
						</div>

						<div
							class="mx-auto grid size-40 place-items-center rounded-full border-2 border-primary shadow-[0_0_30px_-5px_var(--primary)]"
						>
							<svg
								class="size-16 text-primary"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="1.5"
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
						</div>

						<h3 class="display-lg mt-8">Tap a card</h3>
						<p class="mx-auto mt-2 max-w-md text-muted-foreground">
							Tap an ID card on the reader or type the card serial below.
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
								class="w-full rounded-md border border-border bg-background px-4 py-3 text-center font-mono text-sm focus:ring-2 focus:ring-primary focus:outline-none"
							/>
						</form>
					</div>
				{/if}
			</div>

			<div class="flex h-full flex-col rounded-2xl border border-border bg-card p-6">
				<div class="mb-4 flex shrink-0 items-baseline justify-between gap-4">
					<div class="flex flex-col">
						<h3 class="text-lg font-medium">Session log</h3>
						<span class="label-mono text-xs opacity-60">Latest activity</span>
					</div>
					<form
						onsubmit={(e) => {
							e.preventDefault();
							handleCardSubmit(cardInput);
						}}
						class="w-56"
					>
						<input
							type="text"
							bind:value={cardInput}
							placeholder="Tap card..."
							class="w-full rounded-pill border border-border bg-background px-4 py-1.5 font-mono text-xs focus:ring-2 focus:ring-primary focus:outline-none"
						/>
					</form>
				</div>

				<div class="flex-1 overflow-y-auto">
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
										<div class="label-mono">
											{#if line.studentNumber}
												#{line.studentNumber} -
											{/if}
											{fmtTime(line.timestamp)}
										</div>
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
		<section class="grid gap-8 px-6 py-10 md:px-12 lg:grid-cols-[1.3fr_0.9fr]">
			<div class="flex min-h-[560px] flex-col rounded-2xl border border-border bg-card p-6">
				<div class="mb-6 flex flex-wrap items-end justify-between gap-4">
					<div>
						<h3 class="text-xl font-semibold">Name roster</h3>
						<p class="mt-1 text-sm text-muted-foreground">
							Mark attendance from the class list without card or ID numbers.
						</p>
					</div>
					<div class="grid grid-cols-3 overflow-hidden rounded-xl border border-border bg-surface">
						{@render manualStat('Names', manualStudents.length)}
						{@render manualStat('In', checkedInCount)}
						{@render manualStat('Done', completedCount)}
					</div>
				</div>

				<div class="mb-4">
					<label for="name-search" class="label-mono">Find name</label>
					<input
						id="name-search"
						bind:value={rosterQuery}
						placeholder="Search by name..."
						class="mt-2 h-11 w-full rounded-md border border-border bg-background px-4 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>

				<div class="min-h-0 flex-1 overflow-y-auto rounded-xl border border-border">
					{#if manualStudents.length === 0}
						<div
							class="flex h-full min-h-72 items-center justify-center p-8 text-center text-sm text-muted-foreground"
						>
							No names match this class or search.
						</div>
					{:else}
						<ul class="divide-y divide-border">
							{#each manualStudents as student (student.id)}
								{@const action = getNextAttendanceType(student)}
								{@const status = getStudentStatus(student)}
								<li class="flex items-center justify-between gap-4 px-4 py-3 hover:bg-surface/50">
									<div class="min-w-0">
										<div class="truncate text-base font-semibold">{student.name}</div>
										<div
											class="mt-1 text-xs {status.tone === 'in'
												? 'text-primary'
												: status.tone === 'out'
													? 'text-muted-foreground'
													: 'text-muted-foreground'}"
										>
											{status.label}
										</div>
									</div>
									<button
										disabled={!action || isProcessing}
										onclick={async () => {
											if (!action) return;
											if (isProcessing) {
												toast('Please wait - processing previous request', false);
												return;
											}
											isProcessing = true;
											try {
												await logForStudent(student, action);
											} finally {
												isProcessing = false;
											}
										}}
										class="min-w-28 rounded-pill px-4 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 {action ===
										'in'
											? 'bg-primary text-primary-foreground hover:bg-accent'
											: action === 'out'
												? 'border border-border bg-background hover:bg-surface'
												: 'border border-border bg-surface text-muted-foreground'}"
									>
										{action === 'in' ? 'Check in' : action === 'out' ? 'Check out' : 'Completed'}
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			</div>

			<div class="flex min-h-[560px] flex-col rounded-2xl border border-border bg-card p-6">
				<div class="mb-4 flex items-baseline justify-between">
					<div>
						<h3 class="text-lg font-medium">Recent activity</h3>
						<span class="label-mono text-xs opacity-60">Today</span>
					</div>
					<span class="label-mono">{recentActivity.length} events</span>
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
				{lastResult.type === 'in'
					? lastResult.isLate
						? 'bg-destructive/20 text-destructive'
						: 'bg-primary/20 text-primary'
					: 'bg-surface text-muted-foreground'}"
			>
				{#if lastResult.type === 'in'}
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
				{:else}
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
						<path d="M18 6 6 18" />
						<path d="m6 6 12 12" />
					</svg>
				{/if}
			</div>
			<div>
				<div class="text-xl font-bold">{lastResult.name}</div>
				<div class="label-mono flex gap-2">
					<span class={lastResult.isLate ? 'font-bold text-destructive' : ''}>
						{lastResult.type === 'in' ? (lastResult.isLate ? 'LATE' : 'CHECK-IN') : 'CHECK-OUT'}
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
						onclick={async () => {
							if (!action) return;
							if (isProcessing) {
								toast('Please wait - processing previous request', false);
								return;
							}
							isProcessing = true;
							try {
								await logForStudent(student, action);
								pickerOpen = false;
							} finally {
								isProcessing = false;
							}
						}}
						class="flex w-full items-center justify-between px-4 py-3 text-left transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
					>
						<span>
							<span class="block font-medium">{student.name}</span>
							<span class="mt-0.5 block text-xs text-muted-foreground">
								{getStudentStatus(student).label}
							</span>
						</span>
						<span class="label-mono text-xs font-bold text-primary">
							{action === 'out' ? 'CHECK OUT' : action === 'in' ? 'CHECK IN' : 'DONE'}
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
			: type === 'out'
				? 'border border-border bg-surface text-foreground'
				: 'bg-destructive text-destructive-foreground'}"
	>
		{type === 'in' ? 'IN' : type === 'out' ? 'OUT' : 'ERROR'}
	</span>
{/snippet}

{#snippet manualStat(label: string, value: number)}
	<div class="min-w-20 px-4 py-3 text-center">
		<div class="label-mono text-[10px]">{label}</div>
		<div class="mt-1 text-2xl font-semibold">{value}</div>
	</div>
{/snippet}
