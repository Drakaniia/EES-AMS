/**
 * The columns these repos touch that the shared repo fixture (`./schema.ts`)
 * does not declare: the `settings` row and `sf2_date_mappings`, which `wipeAll`
 * empties.
 *
 * The real migration chain belongs to the DB layer and is copied verbatim from
 * the Rust one (spec D9), so a test that depends on it would be testing someone
 * else's file. `sf2_split_completed_at` and `branding_logo` are here on purpose:
 * a column that quietly goes missing is exactly what turns "a save from the
 * Settings page re-ran the 12-month split" back on.
 */
export const SETTINGS_SCHEMA = `
CREATE TABLE settings (
	id TEXT PRIMARY KEY NOT NULL,
	day_start TEXT NOT NULL,
	day_end TEXT NOT NULL,
	late_after TEXT NOT NULL,
	quarter TEXT NOT NULL DEFAULT '1st Quarter',
	q1_start TEXT,
	q1_end TEXT,
	q2_start TEXT,
	q2_end TEXT,
	q3_start TEXT,
	q3_end TEXT,
	attendance_mode TEXT NOT NULL DEFAULT 'manual',
	school_id TEXT,
	school_name TEXT,
	school_year TEXT,
	report_month TEXT,
	grade_level TEXT,
	section TEXT,
	adviser_name TEXT,
	school_head_name TEXT,
	school_start_date TEXT,
	last_report_month TEXT,
	sf2_split_completed_at INTEGER,
	branding_logo_path TEXT,
	branding_title TEXT DEFAULT 'EES AMS',
	branding_logo BLOB
);

CREATE TABLE sf2_date_mappings (id TEXT PRIMARY KEY NOT NULL, template_id TEXT);
`;
