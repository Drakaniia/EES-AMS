import { SvelteDate } from 'svelte/reactivity';
import { fmtDate } from '$lib/csv';
import type { AttendanceEvent, AttendanceType, Student, Class } from '$lib/db-rust';

export type LogLine = {
	id: string;
	studentName: string;
	type: AttendanceType | 'error';
	isLate?: boolean;
	message: string;
	timestamp: number | string;
};

export type ManualViewMode = 'boxes' | 'list';

export type LogOptions = {
	timestamp?: number;
};

export type LastResult = {
	ok: boolean;
	name: string;
	type: AttendanceType;
	time: number;
	isLate?: boolean;
	eventId?: string;
};

// ── Pure utility functions ──────────────────────────────────────────────────────

export function getTimeOfDay(): 'Morning' | 'Afternoon' {
	return new SvelteDate().getHours() < 12 ? 'Morning' : 'Afternoon';
}

export function getActiveClass(classes: Class[]): Class | null {
	const now = new SvelteDate();
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

export function eventTime(event: AttendanceEvent) {
	return typeof event.timestamp === 'string'
		? new SvelteDate(event.timestamp).getTime()
		: event.timestamp;
}

export function parseDateKey(dateKey: string) {
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

export function adjustDate(dateKey: string, offsetDays: number): string {
	const parts = parseDateKey(dateKey);
	if (!parts) return dateKey;

	const date = new SvelteDate(parts.year, parts.monthIndex, parts.day);
	date.setDate(date.getDate() + offsetDays);
	return fmtDate(date.getTime());
}

export function formatAttendanceDate(dateKey: string) {
	const parts = parseDateKey(dateKey);
	if (!parts) return dateKey;

	return new SvelteDate(parts.year, parts.monthIndex, parts.day).toLocaleDateString(undefined, {
		weekday: 'short',
		month: 'short',
		day: 'numeric',
		year: 'numeric'
	});
}

export function firstClassTime(classObj: Class | undefined) {
	return classObj?.sessions?.[0]?.startTime ?? classObj?.dayStart ?? '08:00';
}

export function attendanceTimestampForSelectedDate(
	selectedDate: string,
	selectedDateIsToday: boolean,
	classObj: Class | undefined
) {
	if (selectedDateIsToday) return Date.now();

	const parts = parseDateKey(selectedDate);
	if (!parts) return Date.now();

	const [hourValue, minuteValue] = firstClassTime(classObj).split(':').map(Number);
	const hour = typeof hourValue === 'number' && Number.isFinite(hourValue) ? hourValue : 8;
	const minute = typeof minuteValue === 'number' && Number.isFinite(minuteValue) ? minuteValue : 0;

	return new SvelteDate(parts.year, parts.monthIndex, parts.day, hour, minute, 0, 0).getTime();
}

export function studentName(studentId: string, studentById: Map<string, Student>) {
	return studentById.get(studentId)?.name ?? 'Unknown student';
}

export function getStudentClass(student: Student, classById: Map<string, Class>) {
	return student.classId ? classById.get(student.classId) : undefined;
}

export function getAttendanceClass(
	student: Student,
	currentClass: Class | undefined,
	isCardReaderMode: boolean,
	activeClass: Class | null,
	classById: Map<string, Class>
) {
	return (
		currentClass ??
		(isCardReaderMode ? activeClass : undefined) ??
		getStudentClass(student, classById)
	);
}

export function getSessionSegment(classObj: Class | undefined, timestamp: number) {
	if (!classObj?.sessions || classObj.sessions.length <= 1) return 'day';

	const now = new SvelteDate(timestamp);
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

export function getSessionKey(classObj: Class | undefined, timestamp: number) {
	const classKey = classObj?.id || 'unassigned';
	const segment = getSessionSegment(classObj, timestamp) || 'day';
	return `${fmtDate(timestamp)}|${classKey}|${segment}`;
}

export function checkLate(classObj: Class | undefined, timestamp: number): boolean {
	if (!classObj) return false;

	const now = new SvelteDate(timestamp);
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
	const lateTime = new SvelteDate(now.getFullYear(), now.getMonth(), now.getDate(), h, m, 0, 0);
	return now > lateTime;
}

export function isWithinClassHours(classObj: Class | undefined, timestamp: number): boolean {
	if (!classObj) return false;

	const now = new SvelteDate(timestamp);
	const timeStr = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

	if (classObj.sessions && classObj.sessions.length > 0) {
		for (const session of classObj.sessions) {
			if (timeStr >= session.startTime && timeStr <= session.endTime) return true;
		}
		return false;
	}

	return timeStr >= classObj.dayStart && timeStr <= classObj.dayEnd;
}

export function getStudentInitials(name: string) {
	const initials = name
		.split(/\s+/)
		.filter(Boolean)
		.slice(0, 2)
		.map((part) => part[0]?.toUpperCase())
		.join('');

	return initials || 'ST';
}

export function getStudentClassName(student: Student, classById: Map<string, Class>) {
	return getStudentClass(student, classById)?.name ?? 'No class';
}

export function isScheduledDay(selectedDate: string, classObj: Class | undefined): boolean {
	if (!classObj?.days || classObj.days.length === 0) return true;

	const parts = parseDateKey(selectedDate);
	if (!parts) return true;

	const dayOfWeek = new SvelteDate(parts.year, parts.monthIndex, parts.day).getDay();
	return classObj.days.includes(dayOfWeek);
}
