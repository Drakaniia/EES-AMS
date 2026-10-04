/**
 * Domain models — the TypeScript port of `src-tauri/src/domain/models.rs`.
 *
 * The record types are NOT redeclared here. `$lib/types` already declares them
 * with exactly the shape `#[serde(rename_all = "camelCase")]` produced on the
 * Rust side, and the UI already depends on those, so they are re-exported as-is.
 * What is added here is the two things Rust had and `$lib/types` did not: the
 * row-level coercions (SQLite holds INTEGER epoch seconds; the UI sees ISO
 * strings) and the audit-trail request input.
 */

import type {
	AttendanceAuditEntry,
	AttendanceEvent,
	AttendanceType,
	AuditEvent,
	Class,
	CreateClassRequest,
	CreateEventRequest,
	CreateStudentRequest,
	Session,
	Student,
	StudentGender,
	UpdateClassRequest,
	UpdateEventRequest,
	UpdateStudentRequest
} from '$lib/types';

export type {
	AttendanceAuditEntry,
	AttendanceEvent,
	AttendanceType,
	AuditEvent,
	Class,
	CreateClassRequest,
	CreateEventRequest,
	CreateStudentRequest,
	Session,
	Student,
	StudentGender,
	UpdateClassRequest,
	UpdateEventRequest,
	UpdateStudentRequest
};

/**
 * `Student` plus the DepEd learner ID.
 *
 * The Rust `Student` serialized `sf2_learner_id` as `sf2LearnerId` (skipped
 * when NULL), so that is what crossed the wire; `$lib/types` simply never
 * declared the field because no UI component reads it. The SF2 roster sync
 * does, so the repos return it rather than making that layer re-query.
 */
export type StudentRecord = Student & { sf2LearnerId?: string };

/**
 * One `audit_events` row to write. Mirrors Rust `AuditEventInput`.
 *
 * The payloads are already-serialized JSON strings, not objects: the audit table
 * stores whatever the row writer serialized at the time, and a reader must hand
 * back that text byte-for-byte.
 */
export interface AuditEventInput {
	entityType: string;
	entityId?: string;
	action: string;
	summary: string;
	beforeJson?: string;
	afterJson?: string;
	metadataJson?: string;
}

/** Blank text means "no value" everywhere in the domain, never a stored `""`. */
export function normalizeOptionalText(value: string | undefined): string | undefined {
	const trimmed = value?.trim();
	return trimmed ? trimmed : undefined;
}

export function nowEpochSeconds(): number {
	return Math.floor(Date.now() / 1000);
}

export function epochSecondsToIso(seconds: number | bigint): string {
	return new Date(Number(seconds) * 1000).toISOString();
}

/**
 * NaN for text that is not a usable instant. Callers that accept a
 * user-typed date turn this into the `InvalidInput` the Rust `DateTime`
 * deserializer produced for the same input.
 */
export function isoToEpochSeconds(iso: string): number {
	return Math.floor(new Date(iso).getTime() / 1000);
}

/**
 * Mirrors `StudentGender::from_db_value`: anything that is not literally
 * `male`/`female` after trimming is "the school did not tell us", never a
 * guess in either direction.
 */
export function studentGenderFromDb(value: string | null): StudentGender | undefined {
	const trimmed = value?.trim().toLowerCase();
	return trimmed === 'male' || trimmed === 'female' ? trimmed : undefined;
}

/** Mirrors `AttendanceType::from_db_value`: only an explicit `absent` is absent. */
export function attendanceTypeFromDb(value: string): AttendanceType {
	return value === 'absent' ? 'absent' : 'in';
}
