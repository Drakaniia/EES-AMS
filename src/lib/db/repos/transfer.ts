import {
	attendanceSessionKey,
	normalizeAttendanceMode,
	normalizeQuarter,
	type ExportDataRecord,
	type SettingsRecord
} from '$lib/domain/settings';
import { epochSecondsToIso, nowEpochSeconds } from '$lib/domain/models';
import type { AttendanceEvent, Session, WipeOutcome } from '$lib/types';
import { invalidInput } from '../error';
import { getDriver } from '../index';
import { listAllAuditEvents, recordAuditEvent } from './audit';
import { getSettings } from './settings';

/**
 * JSON export, JSON import, and "wipe all" — a port of
 * `commands/data_transfer.rs` and the `collect_export_data` half of
 * `commands/common.rs`.
 *
 * Ordering is the Rust repos' ordering, not a new one: students and classes by
 * name, events newest first, the audit trail oldest first. A snapshot that
 * re-sorts itself between two exports diffs as noise in the teacher's restore.
 */

type StudentRow = {
	id: string;
	name: string;
	gender: string | null;
	class_id: string | null;
	sf2_learner_id: string | null;
	created_at: number;
};

type ClassRow = {
	id: string;
	name: string;
	room: string | null;
	day_start: string;
	day_end: string;
	late_after: string;
	created_at: number;
	sessions: string | null;
	days: string | null;
};

type EventRow = {
	id: string;
	student_id: string;
	class_id: string | null;
	event_type: string;
	timestamp: number;
	note: string | null;
	session_key: string | null;
	override_reason: string | null;
	updated_at: number | null;
};

function optional(value: string | null): string | undefined {
	return value === null ? undefined : value;
}

/** RFC 3339 → the unix seconds every timestamp column stores. */
function requireUnixSeconds(iso: string | undefined, label: string): number {
	if (!iso) throw invalidInput(`missing ${label} timestamp`);
	const millis = Date.parse(iso);
	if (Number.isNaN(millis)) throw invalidInput(`invalid ${label} timestamp: ${iso}`);
	return Math.floor(millis / 1000);
}

function optionalUnixSeconds(iso: string | undefined, label: string): number | null {
	return iso === undefined ? null : requireUnixSeconds(iso, label);
}

/** Table names cannot be bound parameters; these all come from literals in this file. */
function tableRef(name: string): string {
	if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) throw invalidInput(`invalid table name: ${name}`);
	return `"${name}"`;
}

function parseSessions(json: string | null): Session[] {
	if (!json) return [];
	try {
		const parsed: unknown = JSON.parse(json);
		return Array.isArray(parsed) ? (parsed as Session[]) : [];
	} catch {
		return [];
	}
}

function parseDays(json: string | null): number[] {
	if (!json) return [1, 2, 3, 4, 5];
	try {
		const parsed: unknown = JSON.parse(json);
		if (Array.isArray(parsed) && parsed.length > 0) return parsed as number[];
	} catch {
		// fall through to the school week default
	}
	return [1, 2, 3, 4, 5];
}

/** Every table's contents as one JSON snapshot. */
export async function exportAll(): Promise<ExportDataRecord> {
	const driver = getDriver();
	const students = await driver.query<StudentRow>(
		'SELECT id, name, gender, class_id, sf2_learner_id, created_at FROM students ORDER BY name ASC'
	);
	const classes = await driver.query<ClassRow>(
		'SELECT id, name, room, day_start, day_end, late_after, created_at, sessions, days FROM classes ORDER BY name ASC'
	);
	const events = await driver.query<EventRow>(
		'SELECT id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at FROM events ORDER BY timestamp DESC'
	);

	return {
		students: students.map((row) => ({
			id: row.id,
			name: row.name,
			gender: normalizeGender(row.gender),
			classId: optional(row.class_id),
			sf2LearnerId: optional(row.sf2_learner_id),
			createdAt: epochSecondsToIso(row.created_at)
		})),
		classes: classes.map((row) => ({
			id: row.id,
			name: row.name,
			room: row.room ? row.room : undefined,
			dayStart: row.day_start,
			dayEnd: row.day_end,
			lateAfter: row.late_after,
			sessions: parseSessions(row.sessions),
			days: parseDays(row.days),
			createdAt: epochSecondsToIso(row.created_at)
		})),
		events: events.map((row) => ({
			id: row.id,
			studentId: row.student_id,
			classId: optional(row.class_id),
			type: row.event_type === 'absent' ? 'absent' : 'in',
			timestamp: epochSecondsToIso(row.timestamp),
			note: optional(row.note),
			sessionKey: optional(row.session_key),
			overrideReason: optional(row.override_reason),
			updatedAt: row.updated_at === null ? undefined : epochSecondsToIso(row.updated_at)
		})),
		settings: [await getSettings()],
		auditEvents: await listAllAuditEvents(),
		exportedAt: nowEpochSeconds()
	};
}

/** `StudentGender::from_db_value` — case-insensitive, anything else is unknown. */
function normalizeGender(value: string | null): 'male' | 'female' | undefined {
	const trimmed = value?.trim().toLowerCase();
	return trimmed === 'male' || trimmed === 'female' ? trimmed : undefined;
}

/** `serde_json::to_string_pretty` — the bytes `export_json_with_folder` wrote. */
export function toPrettyJson(payload: ExportDataRecord): string {
	return JSON.stringify(payload, null, 2);
}

/**
 * Merge a JSON snapshot back in. Every row is upserted by primary key, so an
 * import adds what is missing and corrects what drifted; it never deletes.
 */
export async function importAll(payload: ExportDataRecord): Promise<void> {
	const driver = getDriver();
	const settings = payload.settings?.[0];
	const auditEvents = payload.auditEvents ?? [];

	return driver.transaction(async () => {
		for (const row of payload.classes) {
			await driver.execute(
				`INSERT INTO classes (id, name, room, day_start, day_end, late_after, sessions, days, created_at)
				 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
				 ON CONFLICT(id) DO UPDATE SET
					name = excluded.name,
					room = excluded.room,
					day_start = excluded.day_start,
					day_end = excluded.day_end,
					late_after = excluded.late_after,
					sessions = excluded.sessions,
					days = excluded.days,
					created_at = excluded.created_at`,
				[
					row.id,
					row.name,
					row.room ?? null,
					row.dayStart,
					row.dayEnd,
					row.lateAfter,
					JSON.stringify(row.sessions ?? []),
					JSON.stringify(row.days ?? []),
					requireUnixSeconds(row.createdAt, 'class')
				]
			);
		}

		for (const row of payload.students) {
			await driver.execute(
				`INSERT INTO students (id, name, gender, class_id, sf2_learner_id, created_at)
			 VALUES (?, ?, ?, ?, ?, ?)
			 ON CONFLICT(id) DO UPDATE SET
				name = excluded.name,
				gender = excluded.gender,
				class_id = excluded.class_id,
				sf2_learner_id = excluded.sf2_learner_id,
				created_at = excluded.created_at`,
				[
					row.id,
					row.name,
					row.gender ?? null,
					row.classId ?? null,
					row.sf2LearnerId ?? null,
					requireUnixSeconds(row.createdAt, 'student')
				]
			);
		}

		for (const row of payload.events) {
			await insertEvent(row);
		}

		if (settings) {
			await insertImportedSettings(settings);
		}

		for (const event of auditEvents) {
			await driver.execute(
				`INSERT OR IGNORE INTO audit_events (id, entity_type, entity_id, action, summary, before_json, after_json, metadata_json, created_at, actor)
				 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
				[
					event.id,
					event.entityType,
					event.entityId ?? null,
					event.action,
					event.summary,
					event.beforeJson ?? null,
					event.afterJson ?? null,
					event.metadataJson ?? null,
					requireUnixSeconds(event.createdAt, 'audit'),
					event.actor
				]
			);
		}

		await recordAuditEvent({
			entityType: 'data_import',
			action: 'import',
			summary: 'Imported JSON backup merge',
			metadataJson: JSON.stringify({
				sourceExportedAt: payload.exportedAt,
				students: payload.students.length,
				classes: payload.classes.length,
				events: payload.events.length,
				settings: payload.settings?.length ?? 0,
				importedAuditEvents: auditEvents.length
			})
		});
	});
}

/**
 * Insert one exported attendance event, preserving its `event_type`.
 *
 * The event type used to be hardcoded to `"in"` on this path, which made
 * `Export JSON` → `wipe all` → `Import JSON` silently rewrite every recorded
 * absence as a present. Absence is the only record of an X mark, so that
 * round-trip was a one-click data-loss path; the type now round-trips.
 */
async function insertEvent(event: AttendanceEvent): Promise<void> {
	const timestamp = requireUnixSeconds(event.timestamp, 'event');
	await getDriver().execute(
		`INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
		 ON CONFLICT(id) DO UPDATE SET
			student_id = excluded.student_id,
			class_id = excluded.class_id,
			event_type = excluded.event_type,
			timestamp = excluded.timestamp,
			note = excluded.note,
			session_key = excluded.session_key,
			override_reason = excluded.override_reason,
			updated_at = excluded.updated_at`,
		[
			event.id,
			event.studentId,
			event.classId ?? null,
			event.type,
			timestamp,
			event.note ?? null,
			event.sessionKey ?? attendanceSessionKey(timestamp, event.classId),
			event.overrideReason ?? null,
			optionalUnixSeconds(event.updatedAt, 'event')
		]
	);
}

/** The snapshot's settings row. */
async function insertImportedSettings(settings: SettingsRecord): Promise<void> {
	await getDriver().execute(
		`INSERT INTO settings (id, day_start, day_end, late_after, quarter, q1_start, q1_end, q2_start, q2_end, q3_start, q3_end, attendance_mode, school_id, school_name, school_year, report_month, grade_level, section, adviser_name, school_head_name)
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		 ON CONFLICT(id) DO UPDATE SET
			day_start = excluded.day_start,
			day_end = excluded.day_end,
			late_after = excluded.late_after,
			quarter = excluded.quarter,
			q1_start = excluded.q1_start,
			q1_end = excluded.q1_end,
			q2_start = excluded.q2_start,
			q2_end = excluded.q2_end,
			q3_start = excluded.q3_start,
			q3_end = excluded.q3_end,
			attendance_mode = excluded.attendance_mode,
			school_id = excluded.school_id,
			school_name = excluded.school_name,
			school_year = excluded.school_year,
			report_month = excluded.report_month,
			grade_level = excluded.grade_level,
			section = excluded.section,
			adviser_name = excluded.adviser_name,
			school_head_name = excluded.school_head_name`,
		[
			settings.id,
			settings.dayStart,
			settings.dayEnd,
			settings.lateAfter,
			normalizeQuarter(settings.quarter),
			settings.q1Start ?? null,
			settings.q1End ?? null,
			settings.q2Start ?? null,
			settings.q2End ?? null,
			settings.q3Start ?? null,
			settings.q3End ?? null,
			normalizeAttendanceMode(settings.attendanceMode),
			settings.schoolId ?? null,
			settings.schoolName ?? null,
			settings.schoolYear ?? null,
			settings.reportMonth ?? null,
			settings.gradeLevel ?? null,
			settings.section ?? null,
			settings.adviserName ?? null,
			settings.schoolHeadName ?? null
		]
	);
}

/** Tables a wipe empties. `audit_events` is absent on purpose: the trail survives. */
const WIPED_TABLES = [
	'attendance_event_audit',
	'sf2_student_mappings',
	'sf2_date_mappings',
	'attendance_day_status',
	'sf2_templates',
	'events',
	'students',
	'classes',
	'settings'
] as const;

/**
 * `wipe_all`. A wipe is the one destructive action in the app with no undo, and
 * it is the action that has actually cost this user their attendance marks. The
 * caller takes the labelled safety copy first and hands the path in here, so the
 * outcome can say where the copy went.
 */
export async function wipeAll(preWipeBackupPath: string | null = null): Promise<WipeOutcome> {
	const driver = getDriver();
	return driver.transaction(async () => {
		const students = await countRows('students');
		const classes = await countRows('classes');
		const events = await countRows('events');
		const settings = await countRows('settings');
		const attendanceAudit = await countRows('attendance_event_audit');
		const sf2Templates = await countRows('sf2_templates');

		for (const table of WIPED_TABLES) {
			await driver.execute(`DELETE FROM ${tableRef(table)}`);
		}
		// The post-wipe school day starts at 08:30, not the 08:00 a never-written
		// settings row reads as; the Rust wipe inserted this row verbatim.
		await driver.execute(
			`INSERT OR IGNORE INTO settings (id, day_start, day_end, late_after, quarter, attendance_mode)
			 VALUES ('app', '08:30', '15:30', '08:45', '1st Quarter', 'manual')`
		);

		await recordAuditEvent({
			entityType: 'database',
			action: 'wipe',
			summary: 'Wiped all application data',
			metadataJson: JSON.stringify({
				students,
				classes,
				events,
				settings,
				attendanceAuditEvents: attendanceAudit,
				sf2Templates,
				preWipeBackupPath
			})
		});

		return {
			deletedStudents: students,
			deletedClasses: classes,
			deletedEvents: events,
			preWipeBackupPath
		};
	});
}

/**
 * The `data_export` trail entry. `export_all` itself stays side-effect free —
 * the Rust command recorded the export *after* it had written a file, and the
 * file write is the shell's job now, so the caller records what it wrote.
 */
export async function recordDataExportAudit(
	metadata: Record<string, unknown>,
	summary = 'Exported JSON backup'
): Promise<void> {
	await recordAuditEvent({
		entityType: 'data_export',
		action: 'export',
		summary,
		metadataJson: JSON.stringify(metadata)
	});
}

/** Row counts for an export, the shape `export_all` logged in Rust. */
export function exportCounts(snapshot: ExportDataRecord): Record<string, unknown> {
	return {
		format: 'json',
		students: snapshot.students.length,
		classes: snapshot.classes.length,
		events: snapshot.events.length,
		settings: snapshot.settings.length,
		auditEvents: snapshot.auditEvents?.length ?? 0
	};
}

async function countRows(table: string): Promise<number> {
	const row = await getDriver().queryOne<{ total: number }>(
		`SELECT COUNT(*) AS total FROM ${tableRef(table)}`
	);
	return Number(row?.total ?? 0);
}
