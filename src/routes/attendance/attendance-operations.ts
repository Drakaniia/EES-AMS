import { SvelteMap } from 'svelte/reactivity';
import {
	addEvents,
	deleteEvent,
	deleteEvents,
	type AttendanceEvent,
	type AttendanceType,
	type CreateEventRequest,
	type Class,
	type Student
} from '$lib/db-rust';
import {
	eventTime,
	attendanceTimestampForSelectedDate,
	getAttendanceClass,
	type LogLine,
	type LogOptions
} from './attendance-state.svelte';
import type { AttendanceLogHandle } from './attendance-page-state.svelte';

/**
 * Minimal page-state shape consumed by operation helpers.
 * Kept intentionally narrow so the module does not depend on the full class.
 */
export type PageState = {
	selectedDate: string;
	selectedDateIsToday: boolean;
	currentClass: Class | undefined;
	isCardReaderMode: boolean;
	activeClass: Class | null;
	classById: Map<string, Class>;
	students: Student[];
	events: AttendanceEvent[];
	absentStudentIds: Set<string>;
	studentById: Map<string, Student>;
	lastEventByStudentForSession: SvelteMap<string, AttendanceEvent>;
	lastAbsentEventByStudentForSession: SvelteMap<string, AttendanceEvent>;
	isProcessing: boolean;
	dateLoading: boolean;
	isPresentingAll: boolean;
	pickerOpen: boolean;
	attendanceLog: AttendanceLogHandle | undefined;
	matchesSelectedClass(student: Student): boolean;
	matchesCurrentSession(event: AttendanceEvent, student: Student, timestamp?: number): boolean;
	getAttendanceDraft(
		student: Student,
		timestamp?: number
	): {
		classObj: Class | undefined | null;
		classId: string | undefined;
		sessionKey: string;
		isLate: boolean;
		className: string;
	};
	logForStudent(
		student: Student,
		forcedType?: AttendanceType | null,
		options?: LogOptions
	): Promise<void>;
};

// ── Mark student ──────────────────────────────────────────────────────────

/**
 * Mark a single student with an explicit action (in / absent).
 * Extracted from `AttendancePageState.markStudent`.
 */
export async function markStudent(
	state: PageState,
	student: Student,
	action: AttendanceType | null,
	closePicker: boolean
): Promise<void> {
	if (state.isProcessing || state.dateLoading) {
		state.attendanceLog?.showToast('Please wait - processing previous request', false);
		return;
	}

	state.isProcessing = true;
	try {
		await state.logForStudent(student, action, {
			suppressLate: true,
			message: `${student.name} marked absent`
		});
		if (closePicker) state.pickerOpen = false;
	} finally {
		state.isProcessing = false;
	}
}

/**
 * Mark a single student absent.
 * Extracted from `AttendancePageState.markAbsent`.
 */
export async function markAbsent(state: PageState, student: Student): Promise<void> {
	if (state.isProcessing || state.dateLoading) {
		state.attendanceLog?.showToast('Please wait - processing previous request', false);
		return;
	}

	state.isProcessing = true;
	try {
		await state.logForStudent(student, 'absent', {
			suppressLate: true,
			message: `${student.name} marked absent`
		});
	} finally {
		state.isProcessing = false;
	}
}

// ── Rebuild absent highlights ─────────────────────────────────────────────

/**
 * Rebuild the session-absent highlight from persisted records so absent marks
 * survive navigation and reloads.
 *
 * Only students with an explicit 'absent' record this session are highlighted —
 * untouched students stay "Pending · Present by default" and are never affected
 * by marking someone else absent.
 *
 * Extracted from `AttendancePageState.rebuildAbsentFromEvents`.
 */
export function rebuildAbsentFromEvents(state: PageState): void {
	state.absentStudentIds.clear();
	for (const student of state.students) {
		if (
			state.matchesSelectedClass(student) &&
			state.lastAbsentEventByStudentForSession.has(student.id)
		) {
			state.absentStudentIds.add(student.id);
		}
	}
}

// ── Undo ──────────────────────────────────────────────────────────────────

/**
 * Delete a single attendance event by id and refresh the absent highlight.
 * Extracted from `AttendancePageState.handleUndo`.
 */
export async function handleUndo(state: PageState, eventId: string): Promise<boolean> {
	try {
		await deleteEvent(eventId);
		state.events = state.events.filter((e) => e.id !== eventId);
		rebuildAbsentFromEvents(state);
		return true;
	} catch {
		return false;
	}
}

// ── Build 'in' event requests ─────────────────────────────────────────────

/**
 * Build 'in' event requests for the given students using the current
 * session/class context (timestamp, classId, sessionKey).
 *
 * Extracted from `AttendancePageState.buildInEventRequests`.
 */
export function buildInEventRequests(state: PageState, students: Student[]): CreateEventRequest[] {
	return students.map((student) => {
		const timestamp = attendanceTimestampForSelectedDate(
			state.selectedDate,
			state.selectedDateIsToday,
			getAttendanceClass(
				student,
				state.currentClass,
				state.isCardReaderMode,
				state.activeClass,
				state.classById
			)
		);
		const draft = state.getAttendanceDraft(student, timestamp);
		return {
			studentId: student.id,
			classId: draft.classId,
			type: 'in' as const,
			sessionKey: draft.sessionKey,
			timestamp: new Date(timestamp).toISOString()
		};
	});
}

// ── Bulk: present all ─────────────────────────────────────────────────────

/**
 * Record attendance for the entire class roster. Students without a present
 * record get one, including students currently marked absent who are restored.
 *
 * Extracted from `AttendancePageState.presentAllStudents`.
 */
export async function presentAllStudents(state: PageState): Promise<void> {
	if (state.isProcessing || state.dateLoading) {
		state.attendanceLog?.showToast('Please wait - processing previous request', false);
		return;
	}

	const studentsToMark = state.students
		.filter((student) => state.matchesSelectedClass(student))
		.sort((a, b) => a.name.localeCompare(b.name))
		.filter((student) => getNextAttendanceType(state, student) === 'in');

	const restoredStudentIds = studentsToMark.filter((student) =>
		state.absentStudentIds.has(student.id)
	);
	const restoredIdSet = new Set(restoredStudentIds.map((student) => student.id));

	if (studentsToMark.length === 0) {
		state.attendanceLog?.showToast('All students are already recorded as present');
		return;
	}

	state.isProcessing = true;
	state.isPresentingAll = true;
	state.attendanceLog?.resetUndo();

	const eventMetadata = new SvelteMap<string, { student: Student }>();
	const createdEvents: AttendanceEvent[] = [];
	const createdLogLines: LogLine[] = [];

	try {
		const absentEventIds = restoredStudentIds
			.map((student) => state.lastAbsentEventByStudentForSession.get(student.id)?.id)
			.filter((id): id is string => !!id);
		if (absentEventIds.length > 0) {
			await deleteEvents(absentEventIds, 'Present all by user');
		}

		for (const student of studentsToMark) {
			eventMetadata.set(student.id, { student });
		}

		const batchEvents = await addEvents(buildInEventRequests(state, studentsToMark));
		createdEvents.push(...batchEvents);

		for (const createdEvent of batchEvents) {
			const metadata = eventMetadata.get(createdEvent.studentId);
			if (!metadata) continue;
			const restored = restoredIdSet.has(createdEvent.studentId);
			createdLogLines.push({
				id: createdEvent.id,
				studentName: metadata.student.name,
				type: 'in',
				isLate: false,
				message: restored ? 'Present all · restored from absent' : 'Recorded by Present all',
				timestamp: eventTime(createdEvent)
			});
		}

		if (createdEvents.length > 0) {
			state.events = [
				...createdEvents,
				...state.events.filter((e) => !absentEventIds.includes(e.id))
			];
			state.attendanceLog?.addLogEntries(createdLogLines);
		}

		for (const id of absentEventIds) {
			state.attendanceLog?.removeLogEntry(id);
		}
		for (const student of restoredStudentIds) {
			state.absentStudentIds.delete(student.id);
		}

		const restoredCount = restoredStudentIds.length;
		state.attendanceLog?.showToast(
			`${createdEvents.length} ${createdEvents.length === 1 ? 'student' : 'students'} marked present${
				restoredCount > 0 ? ` · ${restoredCount} restored from absent` : ''
			}`
		);
	} catch (err: unknown) {
		const message = err instanceof Error ? err.message : String(err);
		state.attendanceLog?.showToast(`Present all failed: ${message}`, false);
	} finally {
		state.isPresentingAll = false;
		state.isProcessing = false;
	}
}

// ── Bulk: clear all ───────────────────────────────────────────────────────

/**
 * Clear all attendance records for the current session.
 * Extracted from `AttendancePageState.clearAllAttendance`.
 */
export async function clearAllAttendance(state: PageState): Promise<void> {
	if (state.isProcessing || state.dateLoading) {
		state.attendanceLog?.showToast('Please wait - processing previous request', false);
		return;
	}

	const eventIdsToRemove: string[] = [];
	for (const [, event] of state.lastEventByStudentForSession) {
		const student = state.studentById.get(event.studentId);
		if (student && state.matchesCurrentSession(event, student)) {
			eventIdsToRemove.push(event.id);
		}
	}
	for (const [, event] of state.lastAbsentEventByStudentForSession) {
		const student = state.studentById.get(event.studentId);
		if (student && state.matchesCurrentSession(event, student)) {
			eventIdsToRemove.push(event.id);
		}
	}

	const absentToClear = state.absentStudentIds.size;
	if (eventIdsToRemove.length === 0 && absentToClear === 0) {
		state.attendanceLog?.showToast('No recorded attendance to clear');
		return;
	}

	state.isProcessing = true;
	state.attendanceLog?.resetState();

	try {
		if (eventIdsToRemove.length > 0) {
			await deleteEvents(eventIdsToRemove, 'Cleared all by user');
			state.events = state.events.filter((e) => !eventIdsToRemove.includes(e.id));
		}
		state.absentStudentIds.clear();
		if (eventIdsToRemove.length > 0) {
			state.attendanceLog?.showToast(
				`Cleared attendance for ${eventIdsToRemove.length} ${eventIdsToRemove.length === 1 ? 'student' : 'students'}`
			);
		} else {
			state.attendanceLog?.showToast(
				`Reset ${absentToClear} ${absentToClear === 1 ? 'absent mark' : 'absent marks'} to pending`
			);
		}
	} catch (err: unknown) {
		const message = err instanceof Error ? err.message : String(err);
		state.attendanceLog?.showToast(`Clear all failed: ${message}`, false);
	} finally {
		state.isProcessing = false;
	}
}

// ── Helpers ───────────────────────────────────────────────────────────────

/**
 * Determine the next attendance type for a student (used by presentAll).
 * Checks if the student already has an 'in' record this session.
 */
function getNextAttendanceType(state: PageState, student: Student): AttendanceType {
	if (state.lastEventByStudentForSession.has(student.id)) return 'absent';
	return 'in';
}
