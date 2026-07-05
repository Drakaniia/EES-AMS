import type { AttendanceEvent, Class } from '$lib/db-rust';
import { fmtDate, fmtTime } from '$lib/csv';

export type StudentAttendance = {
	studentId: string;
	studentName: string;
	classId?: string;
	className: string;
	date: string;
	checkInTime?: string;
	checkInTimestamp?: number;
	isLate?: boolean;
	events: AttendanceEvent[];
};

export function eventTime(event: AttendanceEvent) {
	return new Date(event.timestamp).getTime();
}

export function primaryEvent(record: StudentAttendance) {
	return [...record.events].sort((a, b) => eventTime(a) - eventTime(b))[0];
}

export function sessionSegment(classObj: Class | undefined, timestamp: Date) {
	if (!classObj?.sessions || classObj.sessions.length <= 1) return 'day';

	const timeStr = `${String(timestamp.getHours()).padStart(2, '0')}:${String(
		timestamp.getMinutes()
	).padStart(2, '0')}`;
	const session = classObj.sessions.find(
		(item) => timeStr >= item.startTime && timeStr <= item.endTime
	);

	return (session?.name || 'off-schedule')
		.trim()
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-|-$/g, '');
}

export function sessionKeyFor(classId: string, timestamp: Date, classMap: Map<string, Class>) {
	const classObj = classMap.get(classId);
	const classKey = classId || 'unassigned';
	const segment = sessionSegment(classObj, timestamp) || 'day';
	return `${fmtDate(timestamp.getTime())}|${classKey}|${segment}`;
}

export function getEventClassName(e: AttendanceEvent, classMap: Map<string, Class>, studentMap: Map<string, { id: string; name: string; classId?: string }>) {
	const id = e.classId || studentMap.get(e.studentId)?.classId;
	if (!id) return '—';
	return classMap.get(id)?.name ?? 'Unknown';
}

export function checkIsLate(event: AttendanceEvent, student: { classId?: string }, classes: Class[]): boolean {
	const studentClass = classes.find((c) => c.id === student.classId);
	if (!studentClass) return false;

	const eventTime = new Date(event.timestamp);
	const timeStr = `${String(eventTime.getHours()).padStart(2, '0')}:${String(eventTime.getMinutes()).padStart(2, '0')}`;

	let lateAfter = studentClass.lateAfter;
	if (studentClass.sessions && studentClass.sessions.length > 0) {
		for (const session of studentClass.sessions) {
			if (timeStr >= session.startTime && timeStr <= session.endTime) {
				lateAfter = session.lateAfter;
				break;
			}
		}
	}

	if (lateAfter) {
		const [h, m] = lateAfter.split(':').map(Number);
		const lateTime = new Date(
			eventTime.getFullYear(),
			eventTime.getMonth(),
			eventTime.getDate(),
			h,
			m,
			0,
			0
		);
		return eventTime > lateTime;
	}
	return false;
}
