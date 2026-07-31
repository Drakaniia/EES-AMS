import { SvelteMap } from 'svelte/reactivity';
import { page } from '$app/state';
import {
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
import { fmtDate, fmtTime } from '$lib/csv';
import { settingsStore } from '$lib/stores/settings.svelte';
import {
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
	type LogOptions,
	type LastResult
} from './attendance-state.svelte';

export type AttendanceLogHandle = {
	showToast: (msg: string, ok?: boolean) => void;
	addLogEntry: (entry: LogLine) => void;
	addLogEntries: (entries: LogLine[]) => void;
	removeLogEntry: (id: string) => void;
	setUndo: (eventId: string, result: LastResult) => void;
	resetUndo: () => void;
	resetState: () => void;
};

type ManualViewMode = 'boxes' | 'list';

class AttendancePageState {
	// ── State ──────────────────────────────────────────────────────────────────
	log = $state<LogLine[]>([]);
	students = $state<Student[]>([]);
	classes = $state<Class[]>([]);
	events = $state<AttendanceEvent[]>([]);
	selectedClassId = $state('');
	manualViewMode = $state<ManualViewMode>('boxes');
	loading = $state(true);
	loadError = $state<string | null>(null);
	datePickerOpen = $state(false);
	dateLoading = $state(false);

	pickerOpen = $state(false);
	pickerQuery = $state('');
	rosterQuery = $state('');

	cardInput = $state('');
	cardInputElement: HTMLInputElement | null = $state(null);
	isProcessing = $state(false);
	isPresentingAll = $state(false);
	lastScan = $state<{ serial: string; timestamp: number } | null>(null);
	selectedDate = $state(fmtDate(Date.now()));
	midnightTimer: ReturnType<typeof setTimeout> | null = null;
	attendanceLog: AttendanceLogHandle | undefined = $state();

	// ── Derived ────────────────────────────────────────────────────────────────
	settingsPending = $derived(settingsStore.loading && !settingsStore.settings);
	attendanceMode = $derived(settingsStore.settings?.attendanceMode ?? 'manual');
	isCardReaderMode = $derived(this.attendanceMode === 'card_reader');
	currentClass = $derived(this.classes.find((c) => c.id === this.selectedClassId));
	isScheduledDayValue = $derived(isScheduledDay(this.selectedDate, this.currentClass));
	selectedDateEvents = $derived(
		this.events.filter((event) => fmtDate(event.timestamp) === this.selectedDate)
	);
	selectedDateLabel = $derived(formatAttendanceDate(this.selectedDate));
	displayDateLabel = $derived.by(() => {
		const today = fmtDate(Date.now());
		const yesterday = adjustDate(today, -1);
		const tomorrow = adjustDate(today, 1);

		const formatted = formatAttendanceDate(this.selectedDate);
		if (this.selectedDate === today) {
			return `Today \u2022 ${formatted}`;
		} else if (this.selectedDate === yesterday) {
			return `Yesterday \u2022 ${formatted}`;
		} else if (this.selectedDate === tomorrow) {
			return `Tomorrow \u2022 ${formatted}`;
		}
		return formatted;
	});
	selectedDateIsToday = $derived(this.selectedDate === fmtDate(Date.now()));
	studentById = $derived(new SvelteMap(this.students.map((student) => [student.id, student])));
	classById = $derived(new SvelteMap(this.classes.map((classItem) => [classItem.id, classItem])));

	manualStudents = $derived.by(() => {
		const query = this.rosterQuery.trim().toLowerCase();
		return this.students
			.filter((student) => {
				const matchesClass =
					!this.selectedClassId ||
					student.classId === this.selectedClassId ||
					(this.classes.length <= 1 && !student.classId);
				const matchesQuery = !query || student.name.toLowerCase().includes(query);
				return matchesClass && matchesQuery;
			})
			.sort((a, b) => a.name.localeCompare(b.name));
	});

	pickerStudents = $derived.by(() => {
		const query = this.pickerQuery.trim().toLowerCase();
		return this.students
			.filter((student) => {
				const matchesQuery = !query || student.name.toLowerCase().includes(query);
				const matchesClass =
					!this.selectedClassId ||
					student.classId === this.selectedClassId ||
					(this.classes.length <= 1 && !student.classId);
				return matchesQuery && matchesClass;
			})
			.sort((a, b) => a.name.localeCompare(b.name));
	});

	lastEventByStudentForSession = $derived.by(() => {
		const byStudent = new SvelteMap<string, AttendanceEvent>();

		for (const event of this.selectedDateEvents) {
			const student = this.studentById.get(event.studentId);
			if (!student || !this.matchesCurrentSession(event, student)) continue;

			const previous = byStudent.get(event.studentId);
			if (!previous || eventTime(event) > eventTime(previous)) {
				byStudent.set(event.studentId, event);
			}
		}

		return byStudent;
	});

	pendingManualStudents = $derived.by(() =>
		this.manualStudents.filter((student) => this.getNextAttendanceType(student) === 'in')
	);
	recordedCount = $derived(this.manualStudents.length - this.pendingManualStudents.length);
	pendingCount = $derived(this.pendingManualStudents.length);

	activeClass = $derived(getActiveClass(this.classes));
	sessionClass = $derived.by(() => {
		if (this.currentClass) return this.currentClass;
		if (this.isCardReaderMode) return this.activeClass ?? undefined;
		return undefined;
	});

	// ── Lifecycle ──────────────────────────────────────────────────────────────
	constructor() {
		$effect.root(() => {
			$effect(() => {
				if (
					this.isCardReaderMode &&
					!this.pickerOpen &&
					!this.datePickerOpen &&
					this.cardInputElement &&
					!this.loading &&
					!this.loadError
				) {
					this.cardInputElement.focus();
				}
			});
		});
	}

	async init() {
		await this.loadInitial();
		this.scheduleMidnightRefresh();
	}

	destroy() {
		if (this.midnightTimer) clearTimeout(this.midnightTimer);
	}

	async loadInitial() {
		this.loading = true;
		this.loadError = null;
		try {
			await Promise.all([this.reload(), settingsStore.load()]);

			const requestedClassId = page.url.searchParams.get('classId');
			if (requestedClassId && this.classes.some((c) => c.id === requestedClassId)) {
				this.selectedClassId = requestedClassId;
			} else {
				const active = getActiveClass(this.classes);
				this.selectedClassId = active?.id ?? this.classes[0]?.id ?? '';
			}

			if (page.url.searchParams.get('manual') === 'true') {
				this.pickerQuery = '';
				this.pickerOpen = true;
			}
		} catch (error) {
			this.loadError =
				error instanceof Error ? error.message : 'Attendance data could not be loaded.';
		} finally {
			this.loading = false;
		}
	}

	async reload() {
		const [s, c, e] = await Promise.all([
			listStudents(),
			listClasses(),
			listEventsForDate(this.selectedDate)
		]);
		this.students = s;
		this.classes = c;
		this.events = e;
	}

	// ── Session helpers ────────────────────────────────────────────────────────
	getNextAttendanceType(student: Student): AttendanceType | null {
		const last = this.lastEventByStudentForSession.get(student.id);
		if (!last) return 'in';
		return null;
	}

	getStudentStatus(student: Student) {
		const last = this.lastEventByStudentForSession.get(student.id);
		if (!last) return { label: 'Not recorded', tone: 'idle' as const };
		return { label: `Recorded ${fmtTime(last.timestamp)}`, tone: 'in' as const };
	}

	getAttendanceDraft(student: Student, timestamp?: number) {
		const classObj = getAttendanceClass(
			student,
			this.currentClass,
			this.isCardReaderMode,
			this.activeClass,
			this.classById
		);
		const resolvedTimestamp =
			timestamp ??
			attendanceTimestampForSelectedDate(this.selectedDate, this.selectedDateIsToday, classObj);
		const classId = classObj?.id || this.selectedClassId || student.classId || undefined;
		const sessionKey = getSessionKey(classObj, resolvedTimestamp);

		return {
			classObj,
			classId,
			sessionKey,
			isLate: checkLate(classObj, resolvedTimestamp),
			className: classObj?.name ?? 'Unassigned class'
		};
	}

	matchesCurrentSession(event: AttendanceEvent, student: Student, timestamp?: number) {
		const resolvedTimestamp =
			timestamp ??
			attendanceTimestampForSelectedDate(
				this.selectedDate,
				this.selectedDateIsToday,
				getAttendanceClass(student, this.currentClass, this.isCardReaderMode, this.activeClass, this.classById)
			);
		const draft = this.getAttendanceDraft(student, resolvedTimestamp);
		if (event.sessionKey) return event.sessionKey === draft.sessionKey;
		const eventClassId = event.classId || student.classId || 'unassigned';
		return (
			fmtDate(event.timestamp) === fmtDate(resolvedTimestamp) &&
			eventClassId === (draft.classId || 'unassigned')
		);
	}

	getLastEventForSession(student: Student) {
		return this.lastEventByStudentForSession.get(student.id);
	}

	// ── Date navigation ────────────────────────────────────────────────────────
	scheduleMidnightRefresh() {
		if (this.midnightTimer) clearTimeout(this.midnightTimer);
		const now = new Date();
		const dateAtScheduleTime = fmtDate(now.getTime());
		const nextMidnight = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1, 0, 0, 2, 0);
		this.midnightTimer = setTimeout(
			async () => {
				if (this.selectedDate === dateAtScheduleTime) {
					this.selectedDate = fmtDate(Date.now());
					await this.reload();
				}
				this.scheduleMidnightRefresh();
			},
			Math.max(1000, nextMidnight.getTime() - now.getTime())
		);
	}

	async selectAttendanceDate(date: string) {
		const nextDate = date || fmtDate(Date.now());
		this.datePickerOpen = false;
		if (nextDate === this.selectedDate) return;

		const previousDate = this.selectedDate;
		this.selectedDate = nextDate;
		this.attendanceLog?.resetState();
		this.dateLoading = true;
		try {
			this.events = await listEventsForDate(nextDate);
			this.attendanceLog?.showToast(`Loaded attendance for ${formatAttendanceDate(nextDate)}`);
		} catch (error) {
			this.selectedDate = previousDate;
			const message =
				error instanceof Error ? error.message : 'Attendance date could not be loaded.';
			this.attendanceLog?.showToast(`Date load failed: ${message}`, false);
		} finally {
			this.dateLoading = false;
		}
	}

	handleDateOffset(offset: number) {
		const nextDate = adjustDate(this.selectedDate, offset);
		void this.selectAttendanceDate(nextDate);
	}

	// ── Card reader operations ─────────────────────────────────────────────────
	async handleCardSubmit(serial: string) {
		const trimmed = serial.trim();
		if (!trimmed) return;

		if (this.isProcessing || this.dateLoading) {
			this.attendanceLog?.showToast('Please wait - processing previous tap', false);
			return;
		}

		const now = Date.now();
		if (this.lastScan && this.lastScan.serial === trimmed && now - this.lastScan.timestamp < 2500) {
			this.cardInput = '';
			this.attendanceLog?.showToast(
				'Duplicate card tap ignored - wait a moment before scanning again',
				false
			);
			this.cardInputElement?.focus();
			return;
		}

		this.lastScan = { serial: trimmed, timestamp: now };
		this.cardInput = '';
		this.isProcessing = true;

		try {
			const student = await findStudentByCard(trimmed);
			if (!student) {
				this.attendanceLog?.showToast('Unknown card - not paired to any student', false);
				return;
			}
			await this.logForStudent(student);
		} catch (err: unknown) {
			const message = err instanceof Error ? err.message : String(err);
			this.attendanceLog?.showToast(`Error: ${message}`, false);
		} finally {
			this.isProcessing = false;
			this.cardInputElement?.focus();
		}
	}

	handleCardInputChange(value: string) {
		this.cardInput = value;
	}

	// ── Log operations ─────────────────────────────────────────────────────────
	async logForStudent(
		student: Student,
		forcedType?: AttendanceType | null,
		options: LogOptions = {}
	) {
		const last = this.getLastEventForSession(student);
		const type = forcedType ?? (last ? null : 'in');

		if (type === null && last) {
			try {
				await deleteEvent(last.id, 'Toggled off by user');
				this.events = this.events.filter((e) => e.id !== last.id);
				this.attendanceLog?.removeLogEntry(last.id);
				this.attendanceLog?.showToast(`${student.name} - Attendance removed`);
				this.attendanceLog?.resetUndo();
				return;
			} catch {
				this.attendanceLog?.showToast('Failed to remove attendance', false);
				return;
			}
		}

		const ts =
			options.timestamp ??
			attendanceTimestampForSelectedDate(
				this.selectedDate,
				this.selectedDateIsToday,
				getAttendanceClass(student, this.currentClass, this.isCardReaderMode, this.activeClass, this.classById)
			);
		const draft = this.getAttendanceDraft(student, ts);

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

			this.events = [createdEvent, ...this.events];
			this.attendanceLog?.addLogEntry({
				id: createdEvent.id,
				studentName: student.name,
				type: finalType,
				isLate,
				message: isLate ? 'Recorded late' : 'Recorded',
				timestamp: ts
			});
			this.attendanceLog?.showToast(
				`${student.name} - ${isLate ? 'Late attendance' : 'Recorded'}`,
				!isLate
			);
			this.attendanceLog?.setUndo(createdEvent.id, {
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
				this.attendanceLog?.showToast('Already recorded for this session', false);
			} else {
				this.attendanceLog?.showToast(`Error: ${message}`, false);
			}
		}
	}

	async markStudent(student: Student, action: AttendanceType | null, closePicker = false) {
		if (this.isProcessing || this.dateLoading) {
			this.attendanceLog?.showToast('Please wait - processing previous request', false);
			return;
		}

		this.isProcessing = true;
		try {
			await this.logForStudent(student, action);
			if (closePicker) this.pickerOpen = false;
		} finally {
			this.isProcessing = false;
		}
	}

	async handleUndo(eventId: string): Promise<boolean> {
		try {
			await deleteEvent(eventId);
			this.events = this.events.filter((e) => e.id !== eventId);
			return true;
		} catch {
			return false;
		}
	}

	// ── Bulk operations ────────────────────────────────────────────────────────
	async presentAllStudents() {
		if (this.isProcessing || this.dateLoading) {
			this.attendanceLog?.showToast('Please wait - processing previous request', false);
			return;
		}

		const studentsToMark = this.pendingManualStudents;
		if (studentsToMark.length === 0) {
			this.attendanceLog?.showToast('All visible students are already recorded');
			return;
		}

		this.isProcessing = true;
		this.isPresentingAll = true;
		this.attendanceLog?.resetUndo();

		const eventRequests: CreateEventRequest[] = [];
		const eventMetadata = new SvelteMap<string, { student: Student; isLate: boolean }>();
		const createdEvents: AttendanceEvent[] = [];
		const createdLogLines: LogLine[] = [];
		let lateCount = 0;

		try {
			for (const student of studentsToMark) {
				const timestamp = attendanceTimestampForSelectedDate(
					this.selectedDate,
					this.selectedDateIsToday,
					getAttendanceClass(student, this.currentClass, this.isCardReaderMode, this.activeClass, this.classById)
				);
				const draft = this.getAttendanceDraft(student, timestamp);
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
				this.events = [...createdEvents, ...this.events];
				this.attendanceLog?.addLogEntries(createdLogLines);
			}

			const recordedLabel = `${createdEvents.length} ${
				createdEvents.length === 1 ? 'student' : 'students'
			} marked present`;
			const lateLabel = lateCount > 0 ? ` (${lateCount} late)` : '';
			const skippedLabel = skippedCount > 0 ? `; ${skippedCount} already recorded` : '';

			this.attendanceLog?.showToast(`${recordedLabel}${lateLabel}${skippedLabel}`);
		} catch (err: unknown) {
			const message = err instanceof Error ? err.message : String(err);
			this.attendanceLog?.showToast(`Present all failed: ${message}`, false);
		} finally {
			this.isPresentingAll = false;
			this.isProcessing = false;
		}
	}

	async clearAllAttendance() {
		if (this.isProcessing || this.dateLoading) {
			this.attendanceLog?.showToast('Please wait - processing previous request', false);
			return;
		}

		const eventIdsToRemove: string[] = [];
		for (const [, event] of this.lastEventByStudentForSession) {
			const student = this.studentById.get(event.studentId);
			if (student && this.matchesCurrentSession(event, student)) {
				eventIdsToRemove.push(event.id);
			}
		}

		if (eventIdsToRemove.length === 0) {
			this.attendanceLog?.showToast('No recorded attendance to clear');
			return;
		}

		this.isProcessing = true;
		this.attendanceLog?.resetState();

		try {
			await deleteEvents(eventIdsToRemove, 'Cleared all by user');
			this.events = this.events.filter((e) => !eventIdsToRemove.includes(e.id));
			this.attendanceLog?.showToast(
				`Cleared attendance for ${eventIdsToRemove.length} ${eventIdsToRemove.length === 1 ? 'student' : 'students'}`
			);
		} catch (err: unknown) {
			const message = err instanceof Error ? err.message : String(err);
			this.attendanceLog?.showToast(`Clear all failed: ${message}`, false);
		} finally {
			this.isProcessing = false;
		}
	}
}

export const attendanceState = new AttendancePageState();
