import { SvelteDate, SvelteMap, SvelteSet } from 'svelte/reactivity';
import type { AttendanceEvent, Student, Class } from '$lib/db-rust';

export function getActiveClass(classes: Class[]): Class | null {
	const now = new SvelteDate();
	const currentTime = now.getHours() * 60 + now.getMinutes();
	const currentDay = now.getDay();

	for (const classItem of classes) {
		if (classItem.days && !classItem.days.includes(currentDay)) continue;

		const [startHour, startMin] = classItem.dayStart.split(':').map(Number);
		const [endHour, endMin] = classItem.dayEnd.split(':').map(Number);
		const startTime = startHour * 60 + startMin;
		const endTime = endHour * 60 + endMin;

		if (currentTime >= startTime && currentTime <= endTime) return classItem;
	}
	return null;
}

export function eventTime(event: AttendanceEvent): number {
	return typeof event.timestamp === 'string'
		? new SvelteDate(event.timestamp).getTime()
		: event.timestamp;
}

export function initials(name: string) {
	return (
		name
			.split(/\s+/)
			.filter(Boolean)
			.slice(0, 2)
			.map((part) => part[0]?.toUpperCase())
			.join('') || 'ST'
	);
}

export function getCheckedInEvents(relevantTodayEvents: AttendanceEvent[]): AttendanceEvent[] {
	const lastByStudent = new SvelteMap<string, AttendanceEvent>();
	for (const event of [...relevantTodayEvents].sort((a, b) => eventTime(a) - eventTime(b))) {
		lastByStudent.set(event.studentId, event);
	}
	return [...lastByStudent.values()].filter((event) => event.type === 'in');
}

export function getRelevantTodayEvents(
	todayEvents: AttendanceEvent[],
	assignedClass: Class | null,
	studentMap: Map<string, Student>,
	classStudents: Student[]
): AttendanceEvent[] {
	if (!assignedClass) return todayEvents;
	const classStudentIds = new SvelteSet(classStudents.map((student) => student.id));
	return todayEvents.filter((event) => {
		const student = studentMap.get(event.studentId);
		return (
			event.classId === assignedClass.id ||
			student?.classId === assignedClass.id ||
			classStudentIds.has(event.studentId)
		);
	});
}

export function attendanceHref(
	isCardReaderMode: boolean,
	classId?: string
): '/attendance' | `/attendance?${string}` {
	const params: string[] = [];
	if (classId) params.push(`classId=${encodeURIComponent(classId)}`);
	if (classId && isCardReaderMode) params.push('manual=true');
	const query = params.join('&');
	return query ? (`/attendance?${query}` as `/attendance?${string}`) : '/attendance';
}
