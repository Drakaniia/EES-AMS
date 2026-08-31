import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import { page } from '$app/state';
import {
	listStudents,
	listClasses,
	listEventsForDate,
	addEvent,
	deleteEvents,
	type AttendanceEvent,
	type AttendanceType,
	type Student,
	type Class
} from '$lib/db-rust';
import { fmtDate } from '$lib/csv';
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
import { handleCardSubmit as submitCard } from './attendance-card-reader';
import {
	markStudent as opMarkStudent,
	markAbsent as opMarkAbsent,
	rebuildAbsentFromEvents as opRebuildAbsent,
	handleUndo as opHandleUndo,
	presentAllStudents as opPresentAll,
	clearAllAttendance as opClearAll
} from './attendance-operations';

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

	// Absent highlight driven by explicit 'absent' event records. The set mirrors the
	// persisted events (rebuilt on load) so absent marks survive navigation and the
	// SF2 page shows an X for the marked student only.
	absentStudentIds = $state(new SvelteSet<string>());

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

	selectedClassRosterCount = $derived(
		this.students.filter((student) => this.matchesSelectedClass(student)).length
	);

	manualStudents = $derived.by(() => {
		const query = this.rosterQuery.trim().toLowerCase();
		return this.students
			.filter(
				(student) =>
					this.matchesSelectedClass(student) &&
					(!query || student.name.toLowerCase().includes(query))
			)
			.sort((a, b) => a.name.localeCompare(b.name));
	});

	pickerStudents = $derived.by(() => {
		const query = this.pickerQuery.trim().toLowerCase();
		return this.students
			.filter(
				(student) =>
					(!query || student.name.toLowerCase().includes(query)) &&
					this.matchesSelectedClass(student)
			)
			.sort((a, b) => a.name.localeCompare(b.name));
	});

	lastEventByStudentForSession = $derived.by(() => {
		const byStudent = new SvelteMap<string, AttendanceEvent>();

		for (const event of this.selectedDateEvents) {
			if (event.type !== 'in') continue;
			const student = this.studentById.get(event.studentId);
			if (!student || !this.matchesCurrentSession(event, student)) continue;

			const previous = byStudent.get(event.studentId);
			if (!previous || eventTime(event) > eventTime(previous)) {
				byStudent.set(event.studentId, event);
			}
		}

		return byStudent;
	});

	// Students with an explicit absent record this session. Absence is a real
	// stored record (like 'in'), so it survives navigation and SF2 renders an X.
	lastAbsentEventByStudentForSession = $derived.by(() => {
		const byStudent = new SvelteMap<string, AttendanceEvent>();

		for (const event of this.selectedDateEvents) {
			if (event.type !== 'absent') continue;
			const student = this.studentById.get(event.studentId);
			if (!student || !this.matchesCurrentSession(event, student)) continue;

			const previous = byStudent.get(event.studentId);
			if (!previous || eventTime(event) > eventTime(previous)) {
				byStudent.set(event.studentId, event);
			}
		}

		return byStudent;
	});

	// Students with no 'in' record this session (pending + absent). Absent students
	// have an 'absent' record but no 'in' record - they are intentionally included
	// here so presentCount stays the true recorded count.
	unrecordedStudents = $derived.by(() =>
		this.manualStudents.filter((student) => !this.lastEventByStudentForSession.has(student.id))
	);
	presentCount = $derived(this.manualStudents.length - this.unrecordedStudents.length);
	absentCount = $derived(
		this.manualStudents.filter((student) => this.absentStudentIds.has(student.id)).length
	);
	pendingCount = $derived(this.manualStudents.length - this.presentCount - this.absentCount);

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

			// Rebuild the absent highlight from persisted records so absent marks made
			// before leaving the page survive navigation back.
			this.rebuildAbsentFromEvents();

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
	matchesSelectedClass(student: Student) {
		return (
			!this.selectedClassId ||
			student.classId === this.selectedClassId ||
			(this.classes.length <= 1 && !student.classId)
		);
	}

	getNextAttendanceType(student: Student): AttendanceType {
		if (this.lastEventByStudentForSession.has(student.id)) return 'absent';
		return 'in';
	}

	getStudentStatus(student: Student) {
		const last = this.lastEventByStudentForSession.get(student.id);
		if (last) return { label: 'Present', tone: 'present' as const };
		if (this.absentStudentIds.has(student.id)) return { label: 'Absent', tone: 'absent' as const };
		return { label: 'Pending · Present by default', tone: 'pending' as const };
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
				getAttendanceClass(
					student,
					this.currentClass,
					this.isCardReaderMode,
					this.activeClass,
					this.classById
				)
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
					this.rebuildAbsentFromEvents();
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
			this.rebuildAbsentFromEvents();
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
		await submitCard(this, serial);
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
		const lastIn = this.lastEventByStudentForSession.get(student.id);
		const lastAbsent = this.lastAbsentEventByStudentForSession.get(student.id);

		// Decide the target state: an explicit action wins; otherwise toggle
		// (card-reader double tap: present → absent → present).
		let target: AttendanceType;
		if (forcedType === 'in' || forcedType === 'absent') {
			target = forcedType;
		} else if (lastIn) {
			target = 'absent';
		} else if (lastAbsent) {
			target = 'in';
		} else {
			target = 'in';
		}

		if (target === 'in' && lastIn) {
			this.attendanceLog?.showToast('Already recorded for this session', false);
			return;
		}
		if (target === 'absent' && lastAbsent) {
			this.attendanceLog?.showToast(`${student.name} already marked absent`, false);
			return;
		}

		// Remove the conflicting record (if any) so the student has exactly one
		// record per session, then write the new state. Only THIS student is
		// affected - no other attendance is touched.
		const eventIdsToDelete: string[] = [];
		if (target === 'absent' && lastIn) eventIdsToDelete.push(lastIn.id);
		if (target === 'in' && lastAbsent) eventIdsToDelete.push(lastAbsent.id);

		const ts =
			options.timestamp ??
			attendanceTimestampForSelectedDate(
				this.selectedDate,
				this.selectedDateIsToday,
				getAttendanceClass(
					student,
					this.currentClass,
					this.isCardReaderMode,
					this.activeClass,
					this.classById
				)
			);
		const draft = this.getAttendanceDraft(student, ts);
		const isLate = target === 'in' && draft.isLate && !options.suppressLate;

		try {
			if (eventIdsToDelete.length > 0) {
				await deleteEvents(eventIdsToDelete, 'Toggled by user');
			}
			const createdEvent = await addEvent({
				studentId: student.id,
				classId: draft.classId,
				type: target,
				note: isLate ? 'Late' : undefined,
				sessionKey: draft.sessionKey,
				timestamp: new Date(ts).toISOString()
			});

			this.events = [createdEvent, ...this.events.filter((e) => !eventIdsToDelete.includes(e.id))];
			for (const id of eventIdsToDelete) {
				this.attendanceLog?.removeLogEntry(id);
			}
			if (target === 'in') {
				this.absentStudentIds.delete(student.id);
			} else {
				this.absentStudentIds.add(student.id);
			}
			this.attendanceLog?.addLogEntry({
				id: createdEvent.id,
				studentName: student.name,
				type: target,
				isLate,
				message: target === 'in' ? (isLate ? 'Recorded late' : 'Recorded') : 'Marked absent',
				timestamp: ts
			});
			this.attendanceLog?.showToast(
				target === 'in'
					? `${student.name} - ${isLate ? 'Late attendance' : 'Recorded'}`
					: (options.message ?? `${student.name} marked absent`),
				target === 'in' ? !isLate : false
			);
			this.attendanceLog?.setUndo(createdEvent.id, {
				ok: true,
				name: student.name,
				type: target,
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
		await opMarkStudent(this, student, action, closePicker);
	}

	async markAbsent(student: Student) {
		await opMarkAbsent(this, student);
	}

	/**
	 * Rebuilds the session-absent highlight from persisted records so absent marks
	 * survive navigation and reloads.
	 *
	 * Only students with an explicit 'absent' record this session are highlighted -
	 * untouched students stay "Pending · Present by default" and are never affected
	 * by marking someone else absent.
	 */
	rebuildAbsentFromEvents() {
		opRebuildAbsent(this);
	}

	// ── Bulk operations ────────────────────────────────────────────────────────
	async handleUndo(eventId: string): Promise<boolean> {
		return opHandleUndo(this, eventId);
	}

	async presentAllStudents() {
		await opPresentAll(this);
	}

	async clearAllAttendance() {
		await opClearAll(this);
	}
}

export const attendanceState = new AttendancePageState();
