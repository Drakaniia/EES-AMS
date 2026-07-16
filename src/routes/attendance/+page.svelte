<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { CalendarDays, ChevronLeft, ChevronRight } from 'lucide-svelte';
	import { page } from '$app/state';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import DatePickerDialog from '$lib/components/ui/DatePickerDialog.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import AttendanceGrid from './attendance-grid.svelte';
	import AttendanceControls from './attendance-controls.svelte';
	import AttendanceLog from './attendance-log.svelte';
	import {
		getSf2WorkbookSettings,
		listStudents,
		listClasses,
		listEventsForDate,
		findStudentByCard,
		addEvent,
		addEvents,
		deleteEvent,
		deleteEvents,
		type AttendanceEvent,
		type AttendanceType,
		type CreateEventRequest,
		type Student,
		type Class
	} from '$lib/db-rust';
	import {
		sf2MonthByValue,
		defaultSf2FirstSchoolDay
	} from '$lib/features/settings/sf2-workbook';
	import { fmtDate, fmtTime } from '$lib/csv';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import {
		getTimeOfDay,
		getActiveClass,
		eventTime,
		formatAttendanceDate,
		adjustDate,
		attendanceTimestampForSelectedDate,
		getAttendanceClass,
		getSessionKey,
		checkLate,
		isScheduledDay,
		type LogLine,
		type ManualViewMode,
		type LogOptions,
		type LastResult
	} from './attendance-state.svelte';

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

	let cardInput = $state('');
	let cardInputElement: HTMLInputElement | null = $state(null);
	let isProcessing = $state(false);
	let isPresentingAll = $state(false);
	let lastScan = $state<{ serial: string; timestamp: number } | null>(null);
	let selectedDate = $state(fmtDate(Date.now()));
	let midnightTimer: ReturnType<typeof setTimeout> | null = null;
	let attendanceLog:
		| {
				showToast: (msg: string, ok?: boolean) => void;
				addLogEntry: (entry: LogLine) => void;
				addLogEntries: (entries: LogLine[]) => void;
				removeLogEntry: (id: string) => void;
				setUndo: (eventId: string, result: LastResult) => void;
				resetUndo: () => void;
				resetState: () => void;
		  }
		| undefined = $state();

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
				const active = getActiveClass(classes);
				selectedClassId = active?.id ?? classes[0]?.id ?? '';
			}

			// Sync the attendance page date with the current SF2 report month so
			// the teacher does not have to manually select a date every time.
			try {
				if (selectedClassId) {
					const sf2Settings = await getSf2WorkbookSettings(selectedClassId);
					if (sf2Settings?.reportMonth) {
						const sf2Month = sf2MonthByValue(sf2Settings.reportMonth);
						if (sf2Month) {
							const firstSchoolDay = defaultSf2FirstSchoolDay(
								sf2Settings.reportMonth,
								sf2Settings.schoolYear
							);
							const year = new Date().getFullYear();
							const adjustedDate = fmtDate(
								new Date(year, sf2Month.monthIndex, firstSchoolDay).getTime()
							);
							if (adjustedDate !== selectedDate) {
								selectedDate = adjustedDate;
								events = await listEventsForDate(selectedDate);
							}
						}
					}
				}
			} catch {
				// SF2 not configured — keep the default today date
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
	const isScheduledDayValue = $derived(isScheduledDay(selectedDate, currentClass));
	const selectedDateEvents = $derived(
		events.filter((event) => fmtDate(event.timestamp) === selectedDate)
	);
	const selectedDateLabel = $derived(formatAttendanceDate(selectedDate));
	const displayDateLabel = $derived.by(() => {
		const today = fmtDate(Date.now());
		const yesterday = adjustDate(today, -1);
		const tomorrow = adjustDate(today, 1);

		const formatted = formatAttendanceDate(selectedDate);
		if (selectedDate === today) {
			return `Today • ${formatted}`;
		} else if (selectedDate === yesterday) {
			return `Yesterday • ${formatted}`;
		} else if (selectedDate === tomorrow) {
			return `Tomorrow • ${formatted}`;
		}
		return formatted;
	});
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

	const activeClass = $derived(getActiveClass(classes));
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

	function getNextAttendanceType(student: Student): AttendanceType | null {
		const last = lastEventByStudentForSession.get(student.id);
		if (!last) return 'in';
		return null;
	}

	function getStudentStatus(student: Student) {
		const last = lastEventByStudentForSession.get(student.id);
		if (!last) return { label: 'Not recorded', tone: 'idle' };
		return { label: `Recorded ${fmtTime(last.timestamp)}`, tone: 'in' };
	}

	function getAttendanceDraft(student: Student, timestamp?: number) {
		const classObj = getAttendanceClass(
			student,
			currentClass,
			isCardReaderMode,
			activeClass,
			classById
		);
		const resolvedTimestamp =
			timestamp ?? attendanceTimestampForSelectedDate(selectedDate, selectedDateIsToday, classObj);
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
			timestamp ??
			attendanceTimestampForSelectedDate(
				selectedDate,
				selectedDateIsToday,
				getAttendanceClass(student, currentClass, isCardReaderMode, activeClass, classById)
			);
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

	async function selectAttendanceDate(date: string) {
		const nextDate = date || fmtDate(Date.now());
		datePickerOpen = false;
		if (nextDate === selectedDate) return;

		const previousDate = selectedDate;
		selectedDate = nextDate;
		attendanceLog?.resetState();
		dateLoading = true;
		try {
			events = await listEventsForDate(nextDate);
			attendanceLog?.showToast(`Loaded attendance for ${formatAttendanceDate(nextDate)}`);
		} catch (error) {
			selectedDate = previousDate;
			const message =
				error instanceof Error ? error.message : 'Attendance date could not be loaded.';
			attendanceLog?.showToast(`Date load failed: ${message}`, false);
		} finally {
			dateLoading = false;
		}
	}

	function handleDateOffset(offset: number) {
		const nextDate = adjustDate(selectedDate, offset);
		void selectAttendanceDate(nextDate);
	}

	async function handleUndo(eventId: string): Promise<boolean> {
		try {
			await deleteEvent(eventId);
			events = events.filter((e) => e.id !== eventId);
			return true;
		} catch {
			return false;
		}
	}

	async function handleCardSubmit(serial: string) {
		const trimmed = serial.trim();
		if (!trimmed) return;

		if (isProcessing || dateLoading) {
			attendanceLog?.showToast('Please wait - processing previous tap', false);
			return;
		}

		const now = Date.now();
		if (lastScan && lastScan.serial === trimmed && now - lastScan.timestamp < 2500) {
			cardInput = '';
			attendanceLog?.showToast(
				'Duplicate card tap ignored - wait a moment before scanning again',
				false
			);
			cardInputElement?.focus();
			return;
		}

		lastScan = { serial: trimmed, timestamp: now };
		cardInput = '';
		isProcessing = true;

		try {
			const student = await findStudentByCard(trimmed);
			if (!student) {
				attendanceLog?.showToast('Unknown card - not paired to any student', false);
				return;
			}
			await logForStudent(student);
		} catch (err: unknown) {
			const message = err instanceof Error ? err.message : String(err);
			attendanceLog?.showToast(`Error: ${message}`, false);
		} finally {
			isProcessing = false;
			cardInputElement?.focus();
		}
	}

	async function logForStudent(
		student: Student,
		forcedType?: AttendanceType | null,
		options: LogOptions = {}
	) {
		const last = getLastEventForSession(student);
		const type = forcedType ?? (last ? null : 'in');

		if (type === null && last) {
			try {
				await deleteEvent(last.id, 'Toggled off by user');
				events = events.filter((e) => e.id !== last.id);
				attendanceLog?.removeLogEntry(last.id);
				attendanceLog?.showToast(`${student.name} - Attendance removed`);
				attendanceLog?.resetUndo();
				return;
			} catch {
				attendanceLog?.showToast('Failed to remove attendance', false);
				return;
			}
		}

		const ts =
			options.timestamp ??
			attendanceTimestampForSelectedDate(
				selectedDate,
				selectedDateIsToday,
				getAttendanceClass(student, currentClass, isCardReaderMode, activeClass, classById)
			);
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

			events = [createdEvent, ...events];
			attendanceLog?.addLogEntry({
				id: createdEvent.id,
				studentName: student.name,
				type: finalType,
				isLate,
				message: isLate ? 'Recorded late' : 'Recorded',
				timestamp: ts
			});
			attendanceLog?.showToast(
				`${student.name} - ${isLate ? 'Late attendance' : 'Recorded'}`,
				!isLate
			);
			attendanceLog?.setUndo(createdEvent.id, {
				ok: true,
				name: student.name,
				type: finalType,
				time: ts,
				isLate,
				eventId: createdEvent.id
			});
		} catch (err: unknown) {
			const message = err instanceof Error ? err.message : String(err);
			if (message.includes('duplicate attendance') || message.includes('already recorded')) {
				attendanceLog?.showToast('Already recorded for this session', false);
			} else {
				attendanceLog?.showToast(`Error: ${message}`, false);
			}
		}
	}

	async function markStudent(student: Student, action: AttendanceType | null, closePicker = false) {
		if (isProcessing || dateLoading) {
			attendanceLog?.showToast('Please wait - processing previous request', false);
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
			attendanceLog?.showToast('Please wait - processing previous request', false);
			return;
		}

		const studentsToMark = pendingManualStudents;
		if (studentsToMark.length === 0) {
			attendanceLog?.showToast('All visible students are already recorded');
			return;
		}

		isProcessing = true;
		isPresentingAll = true;
		attendanceLog?.resetUndo();

		const eventRequests: CreateEventRequest[] = [];
		const eventMetadata = new SvelteMap<string, { student: Student; isLate: boolean }>();
		const createdEvents: AttendanceEvent[] = [];
		const createdLogLines: LogLine[] = [];
		let lateCount = 0;

		try {
			for (const student of studentsToMark) {
				const timestamp = attendanceTimestampForSelectedDate(
					selectedDate,
					selectedDateIsToday,
					getAttendanceClass(student, currentClass, isCardReaderMode, activeClass, classById)
				);
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
				attendanceLog?.addLogEntries(createdLogLines);
			}

			const recordedLabel = `${createdEvents.length} ${
				createdEvents.length === 1 ? 'student' : 'students'
			} marked present`;
			const lateLabel = lateCount > 0 ? ` (${lateCount} late)` : '';
			const skippedLabel = skippedCount > 0 ? `; ${skippedCount} already recorded` : '';

			attendanceLog?.showToast(`${recordedLabel}${lateLabel}${skippedLabel}`);
		} catch (err: unknown) {
			const message = err instanceof Error ? err.message : String(err);
			attendanceLog?.showToast(`Present all failed: ${message}`, false);
		} finally {
			isPresentingAll = false;
			isProcessing = false;
		}
	}

	async function clearAllAttendance() {
		if (isProcessing || dateLoading) {
			attendanceLog?.showToast('Please wait - processing previous request', false);
			return;
		}

		const eventIdsToRemove: string[] = [];
		for (const [, event] of lastEventByStudentForSession) {
			const student = studentById.get(event.studentId);
			if (student && matchesCurrentSession(event, student)) {
				eventIdsToRemove.push(event.id);
			}
		}

		if (eventIdsToRemove.length === 0) {
			attendanceLog?.showToast('No recorded attendance to clear');
			return;
		}

		isProcessing = true;
		attendanceLog?.resetState();

		try {
			await deleteEvents(eventIdsToRemove, 'Cleared all by user');
			events = events.filter((e) => !eventIdsToRemove.includes(e.id));
			attendanceLog?.showToast(
				`Cleared attendance for ${eventIdsToRemove.length} ${eventIdsToRemove.length === 1 ? 'student' : 'students'}`
			);
		} catch (err: unknown) {
			const message = err instanceof Error ? err.message : String(err);
			attendanceLog?.showToast(`Clear all failed: ${message}`, false);
		} finally {
			isProcessing = false;
		}
	}

	function handleCardInputChange(value: string) {
		cardInput = value;
	}
</script>

<svelte:head>
	<title>{isCardReaderMode ? 'Live Session' : 'Attendance'} - Attendance System</title>
	<meta name="description" content="Record student attendance." />
</svelte:head>

<PageHeader category={pageCategory} title={dynamicTitle} description={dynamicDescription}>
	{#snippet actions()}
		<div class="flex flex-wrap items-center gap-3">
			<div class="inline-flex items-center rounded-pill border border-border bg-background p-0.5 shadow-sm">
				<button
					type="button"
					onclick={() => handleDateOffset(-1)}
					disabled={dateLoading || isProcessing}
					class="flex size-9 items-center justify-center rounded-pill text-muted-foreground hover:bg-surface hover:text-foreground disabled:opacity-40 transition-colors cursor-pointer"
					aria-label="Previous day"
				>
					<ChevronLeft class="size-4" />
				</button>

				<button
					type="button"
					onclick={() => (datePickerOpen = true)}
					disabled={dateLoading || isProcessing}
					class="inline-flex h-9 items-center gap-2 rounded-pill px-3 text-sm font-semibold hover:bg-surface transition-colors cursor-pointer disabled:opacity-60"
					aria-haspopup="dialog"
					aria-expanded={datePickerOpen}
				>
					{#if dateLoading}
						<span class="size-2 rounded-full bg-primary animate-pulse" aria-hidden="true"></span>
					{:else}
						<CalendarDays class="size-4 text-primary" aria-hidden="true" />
					{/if}
					<span class="font-mono text-xs md:text-sm">{displayDateLabel}</span>
				</button>

				<button
					type="button"
					onclick={() => handleDateOffset(1)}
					disabled={dateLoading || isProcessing}
					class="flex size-9 items-center justify-center rounded-pill text-muted-foreground hover:bg-surface hover:text-foreground disabled:opacity-40 transition-colors cursor-pointer"
					aria-label="Next day"
				>
					<ChevronRight class="size-4" />
				</button>
			</div>

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
		class="flex min-h-0 flex-1 flex-col gap-5 px-4 py-5 md:px-8 lg:px-10 xl:grid xl:grid-cols-[minmax(0,1fr)_360px] 2xl:grid-cols-[minmax(0,1fr)_400px]"
	>
		<AttendanceControls
			{classes}
			{sessionClass}
			{isProcessing}
			{dateLoading}
			{cardInput}
			bind:cardInputElement
			{log}
			onCardInputChange={handleCardInputChange}
			onCardSubmit={handleCardSubmit}
		/>
	</section>
{:else}
	<div class="flex min-h-0 flex-1 flex-col px-4 py-5 md:px-8 lg:px-10">
		<AttendanceGrid
			{manualStudents}
			bind:manualViewMode
			{isProcessing}
			{dateLoading}
			{selectedClassId}
			{selectedDateLabel}
			{selectedDate}
			{recentActivity}
			{studentById}
			{classById}
			{recordedCount}
			{pendingCount}
			{pendingManualStudents}
			{rosterQuery}
			{isScheduledDayValue}
			{isPresentingAll}
			onMarkStudent={markStudent}
			onPresentAllStudents={presentAllStudents}
			onClearAllAttendance={clearAllAttendance}
			onRosterQueryChange={(value) => (rosterQuery = value)}
			onGetNextAttendanceType={getNextAttendanceType}
			onGetStudentStatus={getStudentStatus}
		/>
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

<AttendanceLog bind:this={attendanceLog} bind:log onUndo={handleUndo} />
