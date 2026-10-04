import type {
	AttendanceEvent,
	AuditEvent,
	Class,
	ExportData,
	Settings,
	Student,
	WipeOutcome
} from '$lib/types';

/**
 * Settings, the audit trail and the JSON transfer payload — the values the Rust
 * `domain::models` structs carried, as the shapes `$lib/types` and therefore the
 * UI already speak.
 *
 * The records below are supersets of the `$lib/types` interfaces: they add the
 * columns the Rust structs had but the TS `Settings` interface never declared
 * (the SF2 header fields) plus `sf2LearnerId`, which a JSON round-trip must not
 * drop. A superset stays assignable to the interface, so every existing caller
 * keeps compiling.
 */

export type { AuditEvent, AttendanceEvent, Class, ExportData, Settings, Student, WipeOutcome };

/** Rust `AttendanceMode`. The card reader is gone: every stored value reads as manual. */
export type AttendanceMode = 'manual';

/** The single settings row. `settings.rs` selects `WHERE id = 'app'` everywhere. */
export const SETTINGS_ROW_ID = 'app';

export interface SettingsRecord extends Settings {
	/** SF2 header fields (v10). Carried by the Rust model, absent from the TS interface. */
	schoolId?: string;
	schoolName?: string;
	schoolYear?: string;
	reportMonth?: string;
	gradeLevel?: string;
	section?: string;
	adviserName?: string;
	schoolHeadName?: string;
}

/** A student plus the DepEd learner ID a JSON round-trip must preserve. */
export interface StudentRecord extends Student {
	sf2LearnerId?: string | null;
}

export interface ExportDataRecord {
	students: StudentRecord[];
	classes: Class[];
	events: AttendanceEvent[];
	settings: SettingsRecord[];
	auditEvents?: AuditEvent[];
	exportedAt: number;
}

/** The only quarter labels the app accepts; anything else becomes `3rd Quarter`. */
const QUARTERS = ['1st Quarter', '2nd Quarter', '3rd Quarter'] as const;

/** `AttendanceMode::normalize` — the card-reader value no longer exists, so everything is manual. */
export function normalizeAttendanceMode(_value: unknown): AttendanceMode {
	return 'manual';
}

/** The `quarter` guard shared by `SettingsRepository::get`, `update` and `import_all`. */
export function normalizeQuarter(quarter: string | undefined | null): string {
	return QUARTERS.includes(quarter as (typeof QUARTERS)[number])
		? (quarter as string)
		: '3rd Quarter';
}

/** `impl Default for Settings` — what a database with no settings row reads as. */
export function defaultSettings(): SettingsRecord {
	return {
		id: SETTINGS_ROW_ID,
		dayStart: '08:00',
		dayEnd: '15:00',
		lateAfter: '08:45',
		quarter: '1st Quarter',
		attendanceMode: 'manual'
	};
}

/** `chrono::Local` `YYYY-MM-DD` for a stored UTC timestamp. */
function localDateFromUnix(seconds: number): string {
	const date = new Date(seconds * 1000);
	const month = `${date.getMonth() + 1}`.padStart(2, '0');
	const day = `${date.getDate()}`.padStart(2, '0');
	return `${date.getFullYear()}-${month}-${day}`;
}

/**
 * `attendance_session_key`: the one session a day has per class. Deterministic on
 * the event's own local date and class, so an older export without a `session_key`
 * still means the same thing after an import.
 */
export function attendanceSessionKey(timestamp: number, classId?: string | null): string {
	const classKey = classId?.trim() ? classId.trim() : 'unassigned';
	return `${localDateFromUnix(timestamp)}|${classKey}|day`;
}
