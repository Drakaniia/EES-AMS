import { SvelteDate } from 'svelte/reactivity';
import type { AttendanceEvent, Class } from '$lib/api';

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

export function attendanceHref(classId?: string): '/attendance' | `/attendance?${string}` {
	const params: string[] = [];
	if (classId) params.push(`classId=${encodeURIComponent(classId)}`);
	const query = params.join('&');
	return query ? (`/attendance?${query}` as `/attendance?${string}`) : '/attendance';
}
