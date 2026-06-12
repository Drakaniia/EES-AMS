<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import {
		CalendarDays,
		CheckCheck,
		Grid2X2,
		List,
		Search,
		ScanLine,
		ShieldAlert
	} from 'lucide-svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import DatePickerDialog from '$lib/components/ui/DatePickerDialog.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import {
		listStudents,
		listClasses,
		listEventsForDate,
		findStudentByCard,
		addEvent,
		addEvents,
		deleteEvent,
		type AttendanceEvent,
		type AttendanceType,
		type CreateEventRequest,
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
	type LogOptions = {
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
	let datePickerOpen = $state(false);
	let dateLoading = $state(false);

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
	let isPresentingAll = $state(false);
	let lastEventId = $state<string | null>(null);
	let undoTimer: ReturnType<typeof setTimeout> | null = null;
	let lastScan = $state<{ serial: string; timestamp: number } | null>(null);
	let selectedDate = $state(fmtDate(Date.now()));
	let midnightTimer: ReturnType<typeof setTimeout> | null = null;

	onMount(async () => {
		await loadInitial();
		scheduleMidnightRefresh();
	});

	onDestroy(() => {
		if (midnightTimer) clearTimeout(midnightTimer);
	});

	$effect(() => {
		if (
			isCardReaderMode &&
			!pickerOpen &&
			!datePickerOpen &&
			cardInputElement &&
			!loading &&
			!loadError
		) {
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
				selectedClassId = active?.id ?? classes[0]?.id ?? '';
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
		const [s, c, e] = await Promise.all([
			listStudents(),
			listClasses(),
			listEventsForDate(selectedDate)
		]);
		students = s;
		classes = c;
		events = e;
	}

	const settingsPending = $derived(settingsStore.loading && !settingsStore.settings);
	const attendanceMode = $derived(settingsStore.settings?.attendanceMode ?? 'manual');
	const isCardReaderMode = $derived(attendanceMode === 'card_reader');
	const currentClass = $derived(classes.find((c) => c.id === selectedClassId));
	const selectedDateEvents = $derived(
		events.filter((event) => fmtDate(event.timestamp) === selectedDate)
	);
	const selectedDateLabel = $derived(formatAttendanceDate(selectedDate));
	const selectedDateIsToday = $derived(selectedDate === fmtDate(Date.now()));
	const studentById = $derived(new SvelteMap(students.map((student) => [student.id, student])));
	const classById = $derived(new SvelteMap(classes.map((classItem) => [classItem.id, classItem])));

	const manualStudents = $derived.by(() => {
		const query = rosterQuery.trim().toLowerCase();
		return students
			.filter((student) => {
				const matchesClass =
					!selectedClassId ||
					student.classId === selectedClassId ||
					(classes.length <= 1 && !student.classId);
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
				const matchesClass =
					!selectedClassId ||
					student.classId === selectedClassId ||
					(classes.length <= 1 && !student.classId);
				return matchesQuery && matchesClass;
			})
			.sort((a, b) => a.name.localeCompare(b.name));
	});

	const recentActivity = $derived.by(() =>
		selectedDateEvents
			.filter((event) => {
				const student = studentById.get(event.studentId);
				return (
					!selectedClassId ||
					event.classId === selectedClassId ||
					student?.classId === selectedClassId ||
					(classes.length <= 1 && !event.classId && !student?.classId)
				);
			})
			.sort((a, b) => eventTime(b) - eventTime(a))
			.slice(0, 14)
	);

	const lastEventByStudentForSession = $derived.by(() => {
		const byStudent = new SvelteMap<string, AttendanceEvent>();

		for (const event of selectedDateEvents) {
			const student = studentById.get(event.studentId);
			if (!student || !matchesCurrentSession(event, student)) continue;

			const previous = byStudent.get(event.studentId);
			if (!previous || eventTime(event) > eventTime(previous)) {
				byStudent.set(event.studentId, event);
			}
		}

		return byStudent;
	});

	const pendingManualStudents = $derived.by(() =>
		manualStudents.filter((student) => getNextAttendanceType(student) === 'in')
	);
	const recordedCount = $derived(manualStudents.length - pendingManualStudents.length);
	const pendingCount = $derived(pendingManualStudents.length);

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
				return `Recording attendance for ${sessionClass.name} on ${selectedDateLabel} (${sessionClass.dayStart} - ${sessionClass.dayEnd})`;
			}
			return `Active monitoring of student attendance for ${selectedDateLabel}.`;
		}

		if (currentClass) {
			return `Name-only attendance for ${currentClass.name} on ${selectedDateLabel} (${currentClass.dayStart} - ${currentClass.dayEnd})`;
		}
		return `Choose names from the class list and record attendance for ${selectedDateLabel}.`;
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

	function parseDateKey(dateKey: string) {
		const [year, month, day] = dateKey.split('-').map(Number);
		if (
			typeof year !== 'number' ||
			typeof month !== 'number' ||
			typeof day !== 'number' ||
			!Number.isFinite(year) ||
			!Number.isFinite(month) ||
			!Number.isFinite(day)
		) {
			return null;
		}

		return { year, monthIndex: month - 1, day };
	}

	function formatAttendanceDate(dateKey: string) {
		const parts = parseDateKey(dateKey);
		if (!parts) return dateKey;

		return new Date(parts.year, parts.monthIndex, parts.day).toLocaleDateString(undefined, {
			weekday: 'short',
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function firstClassTime(classObj: Class | undefined) {
		return classObj?.sessions?.[0]?.startTime ?? classObj?.dayStart ?? '08:00';
	}

	function attendanceTimestampForSelectedDate(classObj: Class | undefined) {
		if (selectedDateIsToday) return Date.now();

		const parts = parseDateKey(selectedDate);
		if (!parts) return Date.now();

		const [hourValue, minuteValue] = firstClassTime(classObj).split(':').map(Number);
		const hour = typeof hourValue === 'number' && Number.isFinite(hourValue) ? hourValue : 8;
		const minute =
			typeof minuteValue === 'number' && Number.isFinite(minuteValue) ? minuteValue : 0;

		return new Date(parts.year, parts.monthIndex, parts.day, hour, minute, 0, 0).getTime();
	}

	function studentName(studentId: string) {
		return studentById.get(studentId)?.name ?? 'Unknown student';
	}

	function getStudentClass(student: Student) {
		return student.classId ? classById.get(student.classId) : undefined;
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

	function getAttendanceDraft(student: Student, timestamp?: number) {
		const classObj = getAttendanceClass(student);
		const resolvedTimestamp = timestamp ?? attendanceTimestampForSelectedDate(classObj);
		const classId = classObj?.id || selectedClassId || student.classId || undefined;
		const sessionKey = getSessionKey(classObj, resolvedTimestamp);

		return {
			classObj,
			classId,
			sessionKey,
			isLate: checkLate(classObj, resolvedTimestamp),
			className: classObj?.name ?? 'Unassigned class'
		};
	}

	function matchesCurrentSession(event: AttendanceEvent, student: Student, timestamp?: number) {
		const resolvedTimestamp =
			timestamp ?? attendanceTimestampForSelectedDate(getAttendanceClass(student));
		const draft = getAttendanceDraft(student, resolvedTimestamp);
		if (event.sessionKey) return event.sessionKey === draft.sessionKey;
		const eventClassId = event.classId || student.classId || 'unassigned';
		return (
			fmtDate(event.timestamp) === fmtDate(resolvedTimestamp) &&
			eventClassId === (draft.classId || 'unassigned')
		);
	}

	function getLastEventForSession(student: Student) {
		return lastEventByStudentForSession.get(student.id);
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

	function scheduleMidnightRefresh() {
		if (midnightTimer) clearTimeout(midnightTimer);
		const now = new Date();
		const dateAtScheduleTime = fmtDate(now.getTime());
		const nextMidnight = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1, 0, 0, 2, 0);
		midnightTimer = setTimeout(
			async () => {
				if (selectedDate === dateAtScheduleTime) {
					selectedDate = fmtDate(Date.now());
					await reload();
				}
				scheduleMidnightRefresh();
			},
			Math.max(1000, nextMidnight.getTime() - now.getTime())
		);
	}

	function resetRecentActionState() {
		lastResult = null;
		lastEventId = null;
		log = [];
		if (undoTimer) {
			clearTimeout(undoTimer);
			undoTimer = null;
		}
	}

	async function selectAttendanceDate(date: string) {
		const nextDate = date || fmtDate(Date.now());
		datePickerOpen = false;
		if (nextDate === selectedDate) return;

		const previousDate = selectedDate;
		selectedDate = nextDate;
		resetRecentActionState();
		dateLoading = true;
		try {
			events = await listEventsForDate(nextDate);
			toast(`Loaded attendance for ${formatAttendanceDate(nextDate)}`);
		} catch (error) {
			selectedDate = previousDate;
			const message =
				error instanceof Error ? error.message : 'Attendance date could not be loaded.';
			toast(`Date load failed: ${message}`, false);
		} finally {
			dateLoading = false;
		}
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

		if (isProcessing || dateLoading) {
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

	async function logForStudent(
		student: Student,
		forcedType?: AttendanceType,
		options: LogOptions = {}
	) {
		const last = getLastEventForSession(student);
		const type = forcedType ?? (last ? null : 'in');

		if (type === null && last) {
			try {
				await deleteEvent(last.id, 'Toggled off by user');
				events = events.filter((e) => e.id !== last.id);
				log = log.filter((l) => l.id !== last.id);
				toast(`${student.name} - Attendance removed`, true);

				if (undoTimer) clearTimeout(undoTimer);
				lastResult = null;
				lastEventId = null;
				return;
			} catch (err: unknown) {
				handleAttendanceError(err, student);
				return;
			}
		}

		const ts = options.timestamp ?? attendanceTimestampForSelectedDate(getAttendanceClass(student));
		const draft = getAttendanceDraft(student, ts);

		const finalType = type ?? 'in';
		const isLate = finalType === 'in' && draft.isLate;

		try {
			const createdEvent = await addEvent({
				studentId: student.id,
				classId: draft.classId,
				type: finalType,
				note: isLate ? 'Late' : undefined,
				sessionKey: draft.sessionKey,
				timestamp: new Date(ts).toISOString()
			});

			lastEventId = createdEvent.id;
			events = [createdEvent, ...events];
			lastResult = {
				ok: true,
				name: student.name,
				type: finalType,
				time: ts,
				isLate,
				eventId: createdEvent.id
			};
			log = [
				{
					id: createdEvent.id,
					studentName: student.name,
					type: finalType,
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
			toast('Already recorded for this session', false);
			return;
		}
		toast(`Error: ${message}`, false);
	}

	async function markStudent(student: Student, action: AttendanceType | null, closePicker = false) {
		if (isProcessing || dateLoading) {
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

	async function presentAllStudents() {
		if (isProcessing || dateLoading) {
			toast('Please wait - processing previous request', false);
			return;
		}

		const studentsToMark = pendingManualStudents;
		if (studentsToMark.length === 0) {
			toast('All visible students are already recorded');
			return;
		}

		isProcessing = true;
		isPresentingAll = true;
		lastResult = null;
		lastEventId = null;
		if (undoTimer) clearTimeout(undoTimer);

		const eventRequests: CreateEventRequest[] = [];
		const eventMetadata = new SvelteMap<string, { student: Student; isLate: boolean }>();
		const createdEvents: AttendanceEvent[] = [];
		const createdLogLines: LogLine[] = [];
		let lateCount = 0;

		try {
			for (const student of studentsToMark) {
				const timestamp = attendanceTimestampForSelectedDate(getAttendanceClass(student));
				const draft = getAttendanceDraft(student, timestamp);
				const isLate = draft.isLate;

				eventRequests.push({
					studentId: student.id,
					classId: draft.classId,
					type: 'in',
					note: isLate ? 'Late' : undefined,
					sessionKey: draft.sessionKey,
					timestamp: new Date(timestamp).toISOString()
				});
				eventMetadata.set(student.id, { student, isLate });
			}

			const batchEvents = await addEvents(eventRequests);
			createdEvents.push(...batchEvents);
			const skippedCount = Math.max(0, eventRequests.length - batchEvents.length);

			for (const createdEvent of batchEvents) {
				const metadata = eventMetadata.get(createdEvent.studentId);
				if (!metadata) continue;
				createdLogLines.push({
					id: createdEvent.id,
					studentName: metadata.student.name,
					type: 'in',
					isLate: metadata.isLate,
					message: metadata.isLate ? 'Recorded late' : 'Recorded by Present all',
					timestamp: eventTime(createdEvent)
				});

				if (metadata.isLate) lateCount += 1;
			}

			if (createdEvents.length > 0) {
				events = [...createdEvents, ...events];
				log = [...createdLogLines, ...log].slice(0, 30);
			}

			const recordedLabel = `${createdEvents.length} ${
				createdEvents.length === 1 ? 'student' : 'students'
			} marked present`;
			const lateLabel = lateCount > 0 ? ` (${lateCount} late)` : '';
			const skippedLabel = skippedCount > 0 ? `; ${skippedCount} already recorded` : '';

			toast(`${recordedLabel}${lateLabel}${skippedLabel}`);
		} catch (err: unknown) {
			const message = err instanceof Error ? err.message : String(err);
			toast(`Present all failed: ${message}`, false);
		} finally {
			isPresentingAll = false;
			isProcessing = false;
		}
	}
</script>

<svelte:head>
	<title>{isCardReaderMode ? 'Live Session' : 'Attendance'} - Attendance System</title>
	<meta name="description" content="Record student attendance." />
</svelte:head>

<PageHeader category={pageCategory} title={dynamicTitle} description={dynamicDescription}>
	{#snippet actions()}
		<div class="flex flex-wrap items-center gap-3">
			<button
				type="button"
				onclick={() => (datePickerOpen = true)}
				disabled={dateLoading || isProcessing}
				aria-haspopup="dialog"
				aria-expanded={datePickerOpen}
				class="control-ring inline-flex h-10 items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if dateLoading}
					<span class="size-2 rounded-full bg-primary" aria-hidden="true"></span>
				{:else}
					<CalendarDays class="size-4 text-primary" aria-hidden="true" />
				{/if}
				<span class="font-mono">{selectedDate}</span>
			</button>

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
		</div>
	{/snippet}
</PageHeader>

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
						Check the assigned class schedule in Settings before starting a live session.
					</p>
				</div>
			{:else}
				<div class="relative w-full max-w-md text-center" role="status" aria-live="polite">
					<div class="label-mono mb-4 text-primary">
						<span class="inline-block size-2 rounded-full bg-primary align-middle"></span> Ready for card
						taps
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
							disabled={isProcessing || dateLoading}
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
				<span class="label-mono rounded-pill border border-border bg-surface px-2 py-1 text-[10px]">
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
							One click per learner. Boxes show whether attendance has been recorded for
							{selectedDateLabel}.
						</p>
					</div>
					<div class="grid grid-cols-3 overflow-hidden rounded-xl border border-border bg-surface">
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

				<button
					type="button"
					disabled={isProcessing ||
						dateLoading ||
						pendingManualStudents.length === 0 ||
						manualStudents.length === 0}
					onclick={presentAllStudents}
					title={rosterQuery.trim()
						? 'Marks every pending student currently shown by the search as present'
						: 'Marks every pending student in this roster as present'}
					class="inline-flex h-10 shrink-0 items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
				>
					{#if isPresentingAll}
						<span class="size-2 rounded-full bg-primary-foreground" aria-hidden="true"></span>
					{:else}
						<CheckCheck class="size-4" aria-hidden="true" />
					{/if}
					{isPresentingAll
						? 'Recording...'
						: pendingManualStudents.length > 0
							? `Present all (${pendingManualStudents.length})`
							: 'Present all'}
				</button>

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
						class="grid h-full auto-rows-[116px] grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-3 overflow-y-auto pr-1"
					>
						{#each manualStudents as student (student.id)}
							{@const action = getNextAttendanceType(student)}
							{@const status = getStudentStatus(student)}
							<button
								type="button"
								title={`${student.name} - ${status.label}`}
								disabled={isProcessing || dateLoading}
								onclick={() => markStudent(student, action)}
								class="group flex h-[116px] min-w-0 flex-col justify-between overflow-hidden rounded-xl border p-3 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-65 {action ===
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
											class="student-card-name text-sm leading-snug font-semibold break-words whitespace-normal"
										>
											{student.name}
										</span>
									</span>
								</span>
								<span class="flex items-center justify-between gap-2">
									<span class="min-w-0 truncate text-[10px] leading-snug text-muted-foreground">
										{selectedClassId ? status.label : getStudentClassName(student)}
									</span>
									<span
										class="label-mono shrink-0 text-[10px] font-bold {action === 'in'
											? 'text-primary'
											: 'text-muted-foreground'}"
									>
										{action === 'in' ? 'IN' : 'RECORDED'}
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
										disabled={isProcessing || dateLoading}
										onclick={() => markStudent(student, action)}
										class="w-fit min-w-28 rounded-pill px-4 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 {action ===
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

		<div class="flex min-h-[34rem] flex-col rounded-2xl border border-border bg-card p-5">
			<div class="mb-4 flex shrink-0 items-start justify-between gap-3">
				<div>
					<h3 class="text-lg font-medium">Recent activity</h3>
					<span class="label-mono text-xs opacity-60">{selectedDate}</span>
				</div>
				<span class="label-mono rounded-pill border border-border bg-surface px-2 py-1 text-[10px]">
					{recentActivity.length} events
				</span>
			</div>

			<div class="min-h-0 flex-1 overflow-y-auto">
				{#if recentActivity.length === 0}
					<div
						class="flex h-full w-full flex-col items-center justify-center rounded-xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground"
					>
						No attendance has been recorded for {selectedDateLabel}.
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

<DatePickerDialog
	open={datePickerOpen}
	value={selectedDate}
	onClose={() => (datePickerOpen = false)}
	onSelect={({ date }) => {
		void selectAttendanceDate(date);
	}}
/>

<Dialog
	open={pickerOpen}
	title="Manual log"
	description={`Search by name to manually record attendance for ${selectedDateLabel}.`}
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
						disabled={isProcessing || dateLoading}
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

<style>
	.student-card-name {
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 3;
		line-clamp: 3;
	}
</style>
