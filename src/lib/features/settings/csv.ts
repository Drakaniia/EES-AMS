import type { AttendanceEvent, Class, Student } from '$lib/types';

/**
 * The attendance CSV.
 *
 * One row per student per local day, built from the *earliest* check-in of that
 * day, because the school's question is "when did this child arrive" and a
 * student can be recorded more than once.
 */

const CSV_HEADER = 'Date,Class,Room,Name,IN,Late';

const UNKNOWN_CLASS = 'Unknown';
const NO_ROOM = 'N/A';

/**
 * RFC 4180 quoting: a field is quoted when it holds a comma, a quote, or a
 * newline, and an embedded quote is doubled. A student's name or a room name is
 * ordinary text that happens to contain these, so this is the difference between
 * a spreadsheet and a corrupted one.
 */
export function escapeCsvField(value: string): string {
	return /[",\n\r]/.test(value) ? `"${value.replace(/"/g, '""')}"` : value;
}

function csvRow(fields: string[]): string {
	return `${fields.map(escapeCsvField).join(',')}\n`;
}

/** `HH:MM` in local time, which is the unit `late_after` and the sessions use. */
function localTime(timestamp: string): string {
	const date = new Date(timestamp);
	return `${`${date.getHours()}`.padStart(2, '0')}:${`${date.getMinutes()}`.padStart(2, '0')}`;
}

function localDate(timestamp: string): string {
	const date = new Date(timestamp);
	const month = `${date.getMonth() + 1}`.padStart(2, '0');
	const day = `${date.getDate()}`.padStart(2, '0');
	return `${date.getFullYear()}-${month}-${day}`;
}

/** The student's class `late_after`, or the session's when the check-in falls inside one. */
function lateThreshold(schoolClass: Class, time: string): string {
	const session = schoolClass.sessions.find(
		(candidate) => time >= candidate.startTime && time <= candidate.endTime
	);
	return session?.lateAfter ?? schoolClass.lateAfter;
}

/**
 * `globalLateAfter` is the app-wide setting the Rust command took and never read —
 * lateness is a per-class rule. Kept in the signature so the caller does not
 * change when this module is swapped in.
 */
export function buildAttendanceCsv(
	events: AttendanceEvent[],
	students: Student[],
	classes: Class[],
	_globalLateAfter = ''
): string {
	const studentsById = new Map(students.map((student) => [student.id, student]));
	const classesById = new Map(classes.map((schoolClass) => [schoolClass.id, schoolClass]));

	// CSV rows describe check-in (IN) records only; explicit absent marks are not
	// check-ins and must not set a check-in time.
	const byStudentAndDay = new Map<string, AttendanceEvent[]>();
	for (const event of events) {
		if (event.type !== 'in') continue;
		if (!studentsById.has(event.studentId)) continue;
		const key = `${event.studentId}|${localDate(event.timestamp)}`;
		const group = byStudentAndDay.get(key);
		if (group) group.push(event);
		else byStudentAndDay.set(key, [event]);
	}

	let csv = csvRow(CSV_HEADER.split(','));
	for (const group of byStudentAndDay.values()) {
		const earliest = [...group].sort((a, b) => a.timestamp.localeCompare(b.timestamp))[0];
		const student = studentsById.get(earliest.studentId);
		if (!student) continue;
		const schoolClass = student.classId ? classesById.get(student.classId) : undefined;
		const time = localTime(earliest.timestamp);
		const late = schoolClass ? (time > lateThreshold(schoolClass, time) ? 'Yes' : 'No') : '';

		csv += csvRow([
			localDate(earliest.timestamp),
			schoolClass?.name ?? UNKNOWN_CLASS,
			schoolClass?.room ?? NO_ROOM,
			student.name,
			time,
			late
		]);
	}
	return csv;
}
