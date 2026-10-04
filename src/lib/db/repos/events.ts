import { appError, getDriver, invalidInput } from '$lib/db';
import type {
	AttendanceAuditEntry,
	AttendanceEvent,
	CreateEventRequest,
	UpdateEventRequest
} from '$lib/domain/models';
import {
	attendanceTypeFromDb,
	epochSecondsToIso,
	isoToEpochSeconds,
	normalizeOptionalText,
	nowEpochSeconds
} from '$lib/domain/models';
import { recordAuditEvent } from './audit';

/**
 * Event repository — the port of
 * `src-tauri/src/infrastructure/database/events/{mod,write}.rs`.
 *
 * The `session_key` is what makes "already recorded today" decidable: one key
 * per student per class per local calendar day. It is supplied when the caller
 * knows better (SF2 grid edits carry the key they edited) and derived from the
 * event's own local date otherwise.
 *
 * Two duplicate policies live here, and the difference matters:
 *   - `addEvent`  REJECTS a duplicate. One card swipe is one error message.
 *   - `addEvents` SKIPS  a duplicate. A bulk import replays the same rows on
 *     every run, and a half-applied import is worse than an ignored row.
 */

interface EventRow {
	id: string;
	student_id: string;
	class_id: string | null;
	event_type: string;
	timestamp: number;
	note: string | null;
	session_key: string | null;
	override_reason: string | null;
	updated_at: number | null;
}

interface AuditEntryRow {
	id: string;
	event_id: string | null;
	student_id: string;
	class_id: string | null;
	session_key: string | null;
	action: string;
	reason: string;
	before_json: string | null;
	after_json: string | null;
	created_at: number;
	actor: string;
}

const EVENT_COLUMNS =
	'id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at';
const AUDIT_ENTRY_COLUMNS =
	'id, event_id, student_id, class_id, session_key, action, reason, before_json, after_json, created_at, actor';

type DuplicatePolicy = 'reject' | 'skip';

function toEvent(row: EventRow): AttendanceEvent {
	return {
		id: row.id,
		studentId: row.student_id,
		classId: row.class_id ?? undefined,
		type: attendanceTypeFromDb(row.event_type),
		timestamp: epochSecondsToIso(row.timestamp),
		note: row.note ?? undefined,
		sessionKey: row.session_key ?? undefined,
		overrideReason: row.override_reason ?? undefined,
		updatedAt: row.updated_at === null ? undefined : epochSecondsToIso(row.updated_at)
	};
}

function toAuditEntry(row: AuditEntryRow): AttendanceAuditEntry {
	return {
		id: row.id,
		eventId: row.event_id ?? undefined,
		studentId: row.student_id,
		classId: row.class_id ?? undefined,
		sessionKey: row.session_key ?? undefined,
		// Constrained to exactly these three by the `attendance_event_audit`
		// CHECK, so this narrows rather than guesses.
		action: row.action as AttendanceAuditEntry['action'],
		reason: row.reason,
		beforeJson: row.before_json ?? undefined,
		afterJson: row.after_json ?? undefined,
		createdAt: epochSecondsToIso(row.created_at),
		actor: row.actor
	};
}

const pad2 = (value: number) => String(value).padStart(2, '0');

/**
 * `YYYY-MM-DD` at local midnight, rejecting text that does not round-trip —
 * `2026-02-30` is not a date, and Rust's `NaiveDate::parse_from_str` refused it.
 */
function parseLocalDate(date: string, label: string): Date {
	const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date);
	if (match === null) throw invalidInput(`invalid ${label}: expected YYYY-MM-DD`);
	const [, year, month, day] = match;
	const parsed = new Date(Number(year), Number(month) - 1, Number(day));
	if (parsed.getFullYear() !== Number(year) || parsed.getMonth() !== Number(month) - 1) {
		throw invalidInput(`invalid ${label}: expected YYYY-MM-DD`);
	}
	return parsed;
}

/** Local midnight on the following day, so a DST day still bounds exactly one day. */
function nextLocalDay(date: Date): Date {
	const next = new Date(date);
	next.setDate(next.getDate() + 1);
	return next;
}

/**
 * The key that defines "one attendance record per student per class per day".
 * Local, not UTC: a teacher's day starts at their midnight, and a UTC date would
 * split a class's morning and afternoon into two different sessions.
 */
function attendanceSessionKey(timestamp: number, classId: string | undefined): string {
	const local = new Date(timestamp * 1000);
	const day = `${local.getFullYear()}-${pad2(local.getMonth() + 1)}-${pad2(local.getDate())}`;
	return `${day}|${classId?.trim() || 'unassigned'}|day`;
}

async function findDuplicateCount(studentId: string, sessionKey: string): Promise<number> {
	const row = await getDriver().queryOne<{ count: number }>(
		'SELECT COUNT(*) AS count FROM events WHERE student_id = ? AND session_key = ?',
		[studentId, sessionKey]
	);
	return Number(row?.count ?? 0);
}

async function insertAttendanceAudit(entry: {
	eventId: string;
	studentId: string;
	classId: string | null;
	sessionKey: string | null;
	action: AttendanceAuditEntry['action'];
	reason: string;
	beforeJson: string | null;
	afterJson: string | null;
	createdAt: number;
}): Promise<void> {
	await getDriver().execute(
		`INSERT INTO attendance_event_audit (id, event_id, student_id, class_id, session_key, action, reason, before_json, after_json, created_at, actor)
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'admin')`,
		[
			crypto.randomUUID(),
			entry.eventId,
			entry.studentId,
			entry.classId,
			entry.sessionKey,
			entry.action,
			entry.reason,
			entry.beforeJson,
			entry.afterJson,
			entry.createdAt
		]
	);
}

/**
 * The one insert both `addEvent` and `addEvents` use. Returns `undefined` when a
 * duplicate was skipped, which only the bulk caller is allowed to see.
 */
async function insertCreateEvent(
	req: CreateEventRequest,
	policy: DuplicatePolicy
): Promise<AttendanceEvent | undefined> {
	const timestamp =
		req.timestamp === undefined ? nowEpochSeconds() : isoToEpochSeconds(req.timestamp);
	if (Number.isNaN(timestamp)) throw invalidInput('invalid event timestamp');

	const classId = normalizeOptionalText(req.classId);
	const overrideReason = normalizeOptionalText(req.overrideReason);
	const sessionKey =
		normalizeOptionalText(req.sessionKey) ?? attendanceSessionKey(timestamp, classId);

	// An override is the teacher saying "yes, this really is a second mark for
	// the same session", so it is exempt from the duplicate rule.
	if (overrideReason === undefined && (await findDuplicateCount(req.studentId, sessionKey)) > 0) {
		if (policy === 'reject') {
			throw appError('DuplicateAttendance', 'Student already recorded for this session');
		}
		return undefined;
	}

	const event: AttendanceEvent = {
		id: crypto.randomUUID(),
		studentId: req.studentId,
		classId,
		type: req.type,
		timestamp: epochSecondsToIso(timestamp),
		note: normalizeOptionalText(req.note),
		sessionKey,
		overrideReason,
		updatedAt: undefined
	};

	await getDriver().execute(
		`INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		[
			event.id,
			event.studentId,
			event.classId ?? null,
			event.type,
			timestamp,
			event.note ?? null,
			event.sessionKey ?? null,
			event.overrideReason ?? null,
			null
		]
	);

	if (overrideReason !== undefined) {
		await insertAttendanceAudit({
			eventId: event.id,
			studentId: event.studentId,
			classId: event.classId ?? null,
			sessionKey: event.sessionKey ?? null,
			action: 'create_override',
			reason: overrideReason,
			beforeJson: null,
			afterJson: JSON.stringify(event),
			createdAt: nowEpochSeconds()
		});
	}

	await recordAuditEvent({
		entityType: 'attendance_event',
		entityId: event.id,
		action: 'create',
		summary: `Recorded attendance for student ${event.studentId}`,
		afterJson: JSON.stringify(event),
		metadataJson: JSON.stringify({ overrideReason: event.overrideReason ?? null })
	});

	return event;
}

export async function listEvents(): Promise<AttendanceEvent[]> {
	const rows = await getDriver().query<EventRow>(
		`SELECT ${EVENT_COLUMNS} FROM events ORDER BY timestamp DESC`
	);
	return rows.map(toEvent);
}

/** Every event on one local calendar date, newest first. */
export async function listEventsForDate(date: string): Promise<AttendanceEvent[]> {
	const start = parseLocalDate(date, 'attendance date');
	const end = nextLocalDay(start);
	const rows = await getDriver().query<EventRow>(
		`SELECT ${EVENT_COLUMNS} FROM events
		 WHERE timestamp >= ? AND timestamp < ?
		 ORDER BY timestamp DESC`,
		[Math.floor(start.getTime() / 1000), Math.floor(end.getTime() / 1000)]
	);
	return rows.map(toEvent);
}

/** `endDate` is inclusive, so the range covers the whole of both dates. */
export async function listEventsForClassAndDateRange(
	classId: string,
	startDate: string,
	endDate: string
): Promise<AttendanceEvent[]> {
	const start = parseLocalDate(startDate, 'start date');
	const end = nextLocalDay(parseLocalDate(endDate, 'end date'));
	const rows = await getDriver().query<EventRow>(
		`SELECT ${EVENT_COLUMNS} FROM events
		 WHERE class_id = ?
		   AND timestamp >= ?
		   AND timestamp < ?
		 ORDER BY timestamp DESC`,
		[classId, Math.floor(start.getTime() / 1000), Math.floor(end.getTime() / 1000)]
	);
	return rows.map(toEvent);
}

export async function getEvent(id: string): Promise<AttendanceEvent> {
	const row = await getDriver().queryOne<EventRow>(
		`SELECT ${EVENT_COLUMNS} FROM events WHERE id = ?`,
		[id]
	);
	if (row === undefined) throw appError('EventNotFound', id);
	return toEvent(row);
}

export async function listEventsForStudent(studentId: string): Promise<AttendanceEvent[]> {
	const rows = await getDriver().query<EventRow>(
		`SELECT ${EVENT_COLUMNS} FROM events WHERE student_id = ? ORDER BY timestamp DESC`,
		[studentId]
	);
	return rows.map(toEvent);
}

export async function lastEventForStudent(studentId: string): Promise<AttendanceEvent | undefined> {
	const row = await getDriver().queryOne<EventRow>(
		`SELECT ${EVENT_COLUMNS} FROM events WHERE student_id = ? ORDER BY timestamp DESC LIMIT 1`,
		[studentId]
	);
	return row === undefined ? undefined : toEvent(row);
}

export async function addEvent(req: CreateEventRequest): Promise<AttendanceEvent> {
	return await getDriver().transaction(async () => {
		const event = await insertCreateEvent(req, 'reject');
		if (event === undefined)
			throw appError('Internal', 'attendance event was unexpectedly skipped');
		return event;
	});
}

export async function addEvents(reqs: CreateEventRequest[]): Promise<AttendanceEvent[]> {
	return await getDriver().transaction(async () => {
		const events: AttendanceEvent[] = [];
		for (const req of reqs) {
			const event = await insertCreateEvent(req, 'skip');
			if (event !== undefined) events.push(event);
		}
		return events;
	});
}

export async function updateEvent(id: string, req: UpdateEventRequest): Promise<AttendanceEvent> {
	const reason = normalizeOptionalText(req.reason);
	if (reason === undefined) throw invalidInput('audit reason is required');

	const hasClassUpdate = req.classId !== undefined;
	const hasSessionUpdate = req.sessionKey !== undefined;

	const before = await getEvent(id);
	const beforeTimestamp = isoToEpochSeconds(before.timestamp);
	const timestamp =
		req.timestamp === undefined ? beforeTimestamp : isoToEpochSeconds(req.timestamp);
	if (Number.isNaN(timestamp)) throw invalidInput('invalid event timestamp');

	const classId = hasClassUpdate ? normalizeOptionalText(req.classId) : before.classId;
	const note = req.note === undefined ? before.note : normalizeOptionalText(req.note);

	// The session key is re-derived whenever anything it is derived from moved,
	// or when the row somehow has none. Only a plain note/reason edit leaves it
	// alone — otherwise editing a student's note would re-file their morning
	// attendance under a new session.
	const sessionKey = hasSessionUpdate
		? (normalizeOptionalText(req.sessionKey) ?? attendanceSessionKey(timestamp, classId))
		: hasClassUpdate || req.timestamp !== undefined || before.sessionKey === undefined
			? attendanceSessionKey(timestamp, classId)
			: before.sessionKey;

	const updatedAt = nowEpochSeconds();
	const event: AttendanceEvent = {
		id: before.id,
		studentId: before.studentId,
		classId,
		type: before.type,
		timestamp: epochSecondsToIso(timestamp),
		note,
		sessionKey,
		overrideReason: reason,
		updatedAt: epochSecondsToIso(updatedAt)
	};

	await getDriver().transaction(async () => {
		await getDriver().execute(
			`UPDATE events
			 SET class_id = ?, timestamp = ?, note = ?, session_key = ?, override_reason = ?, updated_at = ?
			 WHERE id = ?`,
			[
				event.classId ?? null,
				timestamp,
				event.note ?? null,
				event.sessionKey ?? null,
				event.overrideReason ?? null,
				updatedAt,
				id
			]
		);
		await insertAttendanceAudit({
			eventId: event.id,
			studentId: event.studentId,
			classId: event.classId ?? null,
			sessionKey: event.sessionKey ?? null,
			action: 'update',
			reason,
			beforeJson: JSON.stringify(before),
			afterJson: JSON.stringify(event),
			createdAt: updatedAt
		});
		await recordAuditEvent({
			entityType: 'attendance_event',
			entityId: event.id,
			action: 'update',
			summary: `Updated attendance record for student ${event.studentId}`,
			beforeJson: JSON.stringify(before),
			afterJson: JSON.stringify(event),
			metadataJson: JSON.stringify({ reason })
		});
	});

	return event;
}

export async function deleteEvent(id: string, reason?: string): Promise<void> {
	const before = await getEvent(id);
	const auditReason = normalizeOptionalText(reason);

	await getDriver().transaction(async () => {
		const rows = await getDriver().execute('DELETE FROM events WHERE id = ?', [id]);
		if (rows === 0) throw appError('EventNotFound', id);

		if (auditReason !== undefined) {
			await insertAttendanceAudit({
				eventId: before.id,
				studentId: before.studentId,
				classId: before.classId ?? null,
				sessionKey: before.sessionKey ?? null,
				action: 'delete',
				reason: auditReason,
				beforeJson: JSON.stringify(before),
				afterJson: null,
				createdAt: nowEpochSeconds()
			});
		}

		await recordAuditEvent({
			entityType: 'attendance_event',
			entityId: before.id,
			action: 'delete',
			summary: `Deleted attendance record for student ${before.studentId}`,
			beforeJson: JSON.stringify(before),
			metadataJson: JSON.stringify({ reason: auditReason ?? null })
		});
	});
}

/**
 * Deletes what is there and ignores ids that are not, then reports nothing: the
 * caller asked for a bulk clear and a partially-known id list is still a
 * completed request. Every event that WAS deleted is audited individually.
 */
export async function deleteEvents(ids: string[], reason?: string): Promise<void> {
	const auditReason = normalizeOptionalText(reason);

	await getDriver().transaction(async () => {
		for (const id of ids) {
			const before = await getDriver().queryOne<EventRow>(
				`SELECT ${EVENT_COLUMNS} FROM events WHERE id = ?`,
				[id]
			);
			if (before === undefined) continue;

			const rows = await getDriver().execute('DELETE FROM events WHERE id = ?', [id]);
			if (rows === 0) continue;

			if (auditReason !== undefined) {
				await insertAttendanceAudit({
					eventId: before.id,
					studentId: before.student_id,
					classId: before.class_id,
					sessionKey: before.session_key,
					action: 'delete',
					reason: auditReason,
					beforeJson: JSON.stringify(toEvent(before)),
					afterJson: null,
					createdAt: nowEpochSeconds()
				});
			}

			await recordAuditEvent({
				entityType: 'attendance_event',
				entityId: before.id,
				action: 'delete',
				summary: `Deleted attendance record for student ${before.student_id}`,
				beforeJson: JSON.stringify(toEvent(before)),
				metadataJson: JSON.stringify({ reason: auditReason ?? null })
			});
		}
	});
}

/**
 * The exception trail behind the Records page. `eventId` and `studentId` are
 * filters, not a conjunction: passing both asks for either, because a teacher
 * looking at a student also wants the edits made to the mark they are looking
 * at.
 */
export async function listAttendanceAudit(filters?: {
	eventId?: string;
	studentId?: string;
}): Promise<AttendanceAuditEntry[]> {
	const eventId = filters?.eventId;
	const studentId = filters?.studentId;

	let sql = `SELECT ${AUDIT_ENTRY_COLUMNS} FROM attendance_event_audit`;
	let params: (string | number)[] = [];

	if (eventId !== undefined && studentId !== undefined) {
		sql += ' WHERE event_id = ? OR student_id = ?';
		params = [eventId, studentId];
	} else if (eventId !== undefined) {
		sql += ' WHERE event_id = ?';
		params = [eventId];
	} else if (studentId !== undefined) {
		sql += ' WHERE student_id = ?';
		params = [studentId];
	} else {
		sql += ' ORDER BY created_at DESC LIMIT 200';
	}

	const rows = await getDriver().query<AuditEntryRow>(sql, params);
	return rows.map(toAuditEntry);
}
