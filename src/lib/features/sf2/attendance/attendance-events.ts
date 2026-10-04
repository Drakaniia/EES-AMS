/**
 * The attendance event writes behind every SF2 grid correction — the port of
 * `src-tauri/src/sf2/attendance/attendance_events.rs` and the day-level selectors
 * of `src-tauri/src/sf2/attendance/mod.rs`.
 *
 * Two rules in here are the load-bearing ones, and both are the reason this file
 * exists rather than being three calls into `$lib/db/repos/events`:
 *
 * 1. **Any existing record of either type is removed first.** A learner never
 *    holds both an `in` and an `absent` for the same day, so "set the mark" and
 *    "clear the mark" are the same write.
 * 2. **An event with no `class_id` counts as belonging to the class.** That is
 *    how a learner marked outside a class switch is stored. "Is there a record?"
 *    and "what would this write replace?" therefore use the same predicate and
 *    cannot disagree.
 */

import { getDriver } from '$lib/db';
import { recordAuditEvent } from '$lib/db/repos/audit';
import hasEventOfTypeForDaySql from '$lib/db/sql/has_event_of_type_for_day.sql?raw';
import type { AttendanceEvent, AttendanceType, Student } from '$lib/domain/models';
import type { Sf2AttendanceEvent } from '$lib/features/sf2/logic';

/**
 * The note and audit reason on every event the SF2 grid writes, so the trail says
 * the mark was corrected in the app rather than imported from the workbook.
 */
export const SF2_PREVIEW_CORRECTION = 'SF2 preview correction';

/**
 * The note and audit reason on every event the SF2 "present all" button writes,
 * so the trail says the marks were cleared in bulk in the app rather than
 * imported from the workbook.
 */
export const SF2_PRESENT_ALL_CORRECTION = 'SF2 present-all correction';

/**
 * `HH:MM` to `{ hour, minute }`, or `undefined` for anything else.
 *
 * Out-of-range values are refused rather than clamped: a class whose `day_start`
 * reads `25:00` should fall back to the 08:00 default and say so, not be silently
 * written as 01:00 the next day.
 */
export function parseClock(value: string): { hour: number; minute: number } | undefined {
	const separator = value.trim().indexOf(':');
	if (separator < 0) return undefined;
	const hour = Number(value.trim().slice(0, separator));
	const minute = Number(value.trim().slice(separator + 1));
	if (!/^\d+$/.test(value.trim().slice(0, separator))) return undefined;
	if (!/^\d+$/.test(value.trim().slice(separator + 1))) return undefined;
	if (hour > 23 || minute > 59) return undefined;
	return { hour, minute };
}

/**
 * The epoch seconds at which a local calendar day begins and the next one does.
 *
 * Local, not UTC: SQLite holds epoch seconds and the grid is keyed on the
 * teacher's own calendar, so this is where a day becomes a range. Building it
 * from two local midnights rather than by adding 86 400 keeps a DST day bounding
 * exactly one day.
 */
export function localDayBoundsTimestampsForDate(date: string): { start: number; end: number } {
	const day = parseIsoDay(date);
	const next = new Date(day.getTime());
	next.setDate(next.getDate() + 1);
	return { start: Math.floor(day.getTime() / 1000), end: Math.floor(next.getTime() / 1000) };
}

/**
 * The instant an attendance mark for `date` is stamped with.
 *
 * The class's own `day_start` time, so a mark is filed under the session the
 * teacher actually ran. An unreadable clock falls back to 08:00, which is the
 * same fallback Rust used.
 */
export function attendanceTimestampForDate(date: string, dayStart: string): number {
	const clock = parseClock(dayStart) ?? { hour: 8, minute: 0 };
	const day = parseIsoDay(date);
	day.setHours(clock.hour, clock.minute, 0, 0);
	return Math.floor(day.getTime() / 1000);
}

function parseIsoDay(date: string): Date {
	const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date.trim());
	if (match === null) throw new Error(`${date} is not a YYYY-MM-DD date`);
	return new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]));
}

/**
 * Set a learner's attendance record for one day to `eventType`, replacing
 * whatever was there.
 *
 * `reason` is recorded as the note *and* as the audit override reason, so the
 * trail names which flow wrote the record rather than leaving it to be guessed.
 */
export async function setAttendanceEventForDay(params: {
	studentId: string;
	classId: string;
	date: string;
	dayStart: string;
	eventType: AttendanceType;
	reason: string;
}): Promise<void> {
	const { studentId, classId, date, dayStart, eventType, reason } = params;
	const { start, end } = localDayBoundsTimestampsForDate(date);
	const timestamp = attendanceTimestampForDate(date, dayStart);
	const eventId = crypto.randomUUID();
	const driver = getDriver();

	await driver.transaction(async () => {
		const deleted = await driver.queryOne<{ count: number }>(
			`SELECT COUNT(*) AS count FROM events
			 WHERE student_id = ?
			   AND timestamp >= ?
			   AND timestamp < ?
			   AND (class_id IS NULL OR class_id = ?)`,
			[studentId, start, end, classId]
		);

		await driver.execute(
			`DELETE FROM events
			 WHERE student_id = ?
			   AND timestamp >= ?
			   AND timestamp < ?
			   AND (class_id IS NULL OR class_id = ?)`,
			[studentId, start, end, classId]
		);

		await driver.execute(
			`INSERT INTO events
			   (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
			 VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)`,
			[eventId, studentId, classId, eventType, timestamp, reason, `${date}|${classId}|day`, reason]
		);

		await recordAuditEvent({
			entityType: 'attendance_event',
			entityId: eventId,
			action: 'create',
			summary: `Set SF2 attendance for student ${studentId} on ${date} to ${eventType} (${reason})`,
			metadataJson: JSON.stringify({
				studentId,
				classId,
				date,
				eventType,
				deletedEvents: Number(deleted?.count ?? 0),
				createdEventId: eventId
			})
		});
	});
}

/**
 * The stored predicate, ready to run.
 *
 * Two substitutions, both of which the SQL file forces:
 *
 * - `'{event_type}'` is replaced as the **quoted** placeholder. The file's own
 *   comment mentions `{event_type}` too, and replacing that occurrence first would
 *   leave the real predicate unsubstituted — which counts every event of any type.
 * - The projection is aliased. `COUNT(*)` has no column name of its own, so the row
 *   would arrive keyed by the expression text; and the query is deliberately *not*
 *   wrapped in an outer `SELECT COUNT(*) FROM ( … )`, which would count the one row
 *   the inner aggregate always returns and answer `1` to everything.
 */
function hasEventOfTypeForDayQuery(eventType: AttendanceType): string {
	return hasEventOfTypeForDaySql
		.replaceAll("'{event_type}'", `'${eventType}'`)
		.replace(/^SELECT COUNT\(\*\) FROM/m, 'SELECT COUNT(*) AS count FROM')
		.replace(/;\s*$/, '');
}

/**
 * Does the learner already have an explicit record of `eventType` for that local
 * day?
 *
 * The shared predicate behind "is there an absence?" and "is there a presence?".
 * A blank SF2 cell means present, so a workbook `X` over a day the app holds a
 * `present` for is the app and the school disagreeing — which an unattended
 * process is not entitled to resolve.
 */
export async function hasEventOfTypeForDay(
	studentId: string,
	classId: string,
	date: string,
	eventType: AttendanceType
): Promise<boolean> {
	const { start, end } = localDayBoundsTimestampsForDate(date);
	const row = await getDriver().queryOne<{ count: number }>(hasEventOfTypeForDayQuery(eventType), [
		studentId,
		start,
		end,
		classId
	]);
	return Number(row?.count ?? 0) > 0;
}

/** Does the learner have an explicit `absent` record for that local day? */
export async function hasAbsentEventForDay(
	studentId: string,
	classId: string,
	date: string
): Promise<boolean> {
	return hasEventOfTypeForDay(studentId, classId, date, 'absent');
}

/** Does the learner have an explicit `in` record for that local day? */
export async function hasPresentEventForDay(
	studentId: string,
	classId: string,
	date: string
): Promise<boolean> {
	return hasEventOfTypeForDay(studentId, classId, date, 'in');
}

// ── The per-day selectors ─────────────────────────────────────────────────────

/**
 * The local calendar day an ISO instant falls on.
 *
 * The grid is keyed on the teacher's own date, so this is where UTC becomes
 * local. Reading the UTC day instead is what would file an afternoon's absence
 * under the next column.
 */
function localEventDate(event: AttendanceEvent): string {
	const local = new Date(event.timestamp);
	const pad = (value: number): string => String(value).padStart(2, '0');
	return `${local.getFullYear()}-${pad(local.getMonth() + 1)}-${pad(local.getDate())}`;
}

/**
 * Does this event belong to the class?
 *
 * Either it names the class, or the learner is on the class's roster — the second
 * half is how a mark recorded while a different class was selected still counts.
 */
function eventBelongsToClass(
	event: AttendanceEvent,
	classStudentIds: ReadonlySet<string>,
	classId: string
): boolean {
	return event.classId === classId || classStudentIds.has(event.studentId);
}

function dayEventsOfType(
	events: readonly AttendanceEvent[],
	students: readonly Student[],
	classId: string,
	date: string,
	eventType: AttendanceType
): Sf2AttendanceEvent[] {
	const studentIds = new Set(students.map((student) => student.id));
	return events
		.filter(
			(event) =>
				eventBelongsToClass(event, studentIds, classId) &&
				localEventDate(event) === date &&
				event.type === eventType
		)
		.map((event) => ({ studentId: event.studentId, eventType }));
}

/** The learners with an explicit `in` record for one day. */
export function presentEventsForDay(
	events: readonly AttendanceEvent[],
	students: readonly Student[],
	classId: string,
	date: string
): Sf2AttendanceEvent[] {
	return dayEventsOfType(events, students, classId, date, 'in');
}

/** The learners with an explicit `absent` record for one day — the `X` marks. */
export function absentEventsForDay(
	events: readonly AttendanceEvent[],
	students: readonly Student[],
	classId: string,
	date: string
): Sf2AttendanceEvent[] {
	return dayEventsOfType(events, students, classId, date, 'absent');
}

export function presentStudentIds(
	events: readonly AttendanceEvent[],
	students: readonly Student[],
	classId: string,
	date: string
): Set<string> {
	return new Set(presentEventsForDay(events, students, classId, date).map((e) => e.studentId));
}

export function absentStudentIds(
	events: readonly AttendanceEvent[],
	students: readonly Student[],
	classId: string,
	date: string
): Set<string> {
	return new Set(absentEventsForDay(events, students, classId, date).map((e) => e.studentId));
}
