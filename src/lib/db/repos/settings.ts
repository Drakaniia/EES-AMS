import {
	SETTINGS_ROW_ID,
	defaultSettings,
	normalizeAttendanceMode,
	normalizeQuarter,
	type SettingsRecord
} from '$lib/domain/settings';
import { getDriver } from '../index';
import { recordAuditEvent } from './audit';

/**
 * The settings row.
 *
 * The audit trail is `repos/audit.ts`; a save writes its trail through
 * `recordAuditEvent()` inside the same transaction, so a rolled-back save leaves
 * no orphan row saying it happened.
 */

const SETTINGS_COLUMNS =
	'id, day_start, day_end, late_after, quarter, q1_start, q1_end, q2_start, q2_end, q3_start, q3_end, attendance_mode, school_id, school_name, school_year, report_month, grade_level, section, adviser_name, school_head_name';

type SettingsRow = {
	id: string;
	day_start: string;
	day_end: string;
	late_after: string;
	quarter: string;
	q1_start: string | null;
	q1_end: string | null;
	q2_start: string | null;
	q2_end: string | null;
	q3_start: string | null;
	q3_end: string | null;
	attendance_mode: string;
	school_id: string | null;
	school_name: string | null;
	school_year: string | null;
	report_month: string | null;
	grade_level: string | null;
	section: string | null;
	adviser_name: string | null;
	school_head_name: string | null;
};

function optional(value: string | null): string | undefined {
	return value === null ? undefined : value;
}

function toSettings(row: SettingsRow): SettingsRecord {
	return {
		id: row.id,
		dayStart: row.day_start,
		dayEnd: row.day_end,
		lateAfter: row.late_after,
		quarter: normalizeQuarter(row.quarter),
		attendanceMode: normalizeAttendanceMode(row.attendance_mode),
		q1Start: optional(row.q1_start),
		q1End: optional(row.q1_end),
		q2Start: optional(row.q2_start),
		q2End: optional(row.q2_end),
		q3Start: optional(row.q3_start),
		q3End: optional(row.q3_end),
		schoolId: optional(row.school_id),
		schoolName: optional(row.school_name),
		schoolYear: optional(row.school_year),
		reportMonth: optional(row.report_month),
		gradeLevel: optional(row.grade_level),
		section: optional(row.section),
		adviserName: optional(row.adviser_name),
		schoolHeadName: optional(row.school_head_name)
	};
}

/** `SettingsRepository::get` — a missing row reads as `impl Default for Settings`. */
export async function getSettings(): Promise<SettingsRecord> {
	const row = await getDriver().queryOne<SettingsRow>(
		`SELECT ${SETTINGS_COLUMNS} FROM settings WHERE id = ?`,
		[SETTINGS_ROW_ID]
	);
	return row ? toSettings(row) : defaultSettings();
}

/**
 * `SettingsRepository::update` — a targeted upsert, and the reason is the class
 * of bug rather than the columns it bites today.
 *
 * `INSERT OR REPLACE` with a column list is a *whole-row* replace: SQLite
 * deletes the existing row and inserts a new one built from the listed columns,
 * and every column left off the list comes back at its DEFAULT - NULL, for the
 * v22 columns. `sf2_split_completed_at` going NULL re-runs the entire 12-month
 * split on the next launch, which rewrites every month file, so a save from the
 * Settings page was a destructive action. The next column anyone adds to
 * `settings` would have been destroyed the same way, with no error and no test
 * failing.
 *
 * `ON CONFLICT(id) DO UPDATE SET` names exactly the columns this method owns, so
 * a column the `SettingsRecord` does not carry is left alone whether it exists
 * today or is added by a later migration.
 */
export async function saveSettings(settings: SettingsRecord): Promise<SettingsRecord> {
	const driver = getDriver();
	const before = await getSettings();
	const saved: SettingsRecord = {
		...settings,
		quarter: normalizeQuarter(settings.quarter),
		attendanceMode: normalizeAttendanceMode(settings.attendanceMode)
	};

	return driver.transaction(async () => {
		await driver.execute(
			`INSERT INTO settings (${SETTINGS_COLUMNS})
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
				saved.id,
				saved.dayStart,
				saved.dayEnd,
				saved.lateAfter,
				saved.quarter,
				saved.q1Start ?? null,
				saved.q1End ?? null,
				saved.q2Start ?? null,
				saved.q2End ?? null,
				saved.q3Start ?? null,
				saved.q3End ?? null,
				saved.attendanceMode,
				saved.schoolId ?? null,
				saved.schoolName ?? null,
				saved.schoolYear ?? null,
				saved.reportMonth ?? null,
				saved.gradeLevel ?? null,
				saved.section ?? null,
				saved.adviserName ?? null,
				saved.schoolHeadName ?? null
			]
		);
		await recordAuditEvent({
			entityType: 'settings',
			entityId: saved.id,
			action: 'update',
			summary: 'Updated global settings',
			beforeJson: JSON.stringify(before),
			afterJson: JSON.stringify(saved)
		});
		return saved;
	});
}
