import { useDriver, type SqlDriver } from '$lib/db';
import { NodeSqlDriver } from '$lib/db/node-driver';
import { afterEach, beforeEach } from 'vitest';

/**
 * The minimal schema these repo tests need, in the shape the Rust migration
 * chain leaves it in. The real migration runner belongs to the DB layer, so
 * tests declare the columns their own SQL touches and nothing else.
 */
const SCHEMA = `
CREATE TABLE students (
	id TEXT PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	card_serial TEXT UNIQUE,
	class_id TEXT,
	created_at INTEGER NOT NULL,
	gender TEXT CHECK(gender IS NULL OR gender IN ('male', 'female')),
	sf2_learner_id TEXT
);

CREATE TABLE classes (
	id TEXT PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	day_start TEXT NOT NULL,
	day_end TEXT NOT NULL,
	late_after TEXT NOT NULL,
	created_at INTEGER NOT NULL,
	room TEXT NOT NULL DEFAULT 'N/A',
	sessions TEXT,
	days TEXT
);

CREATE TABLE events (
	id TEXT PRIMARY KEY NOT NULL,
	student_id TEXT NOT NULL,
	class_id TEXT,
	event_type TEXT NOT NULL,
	timestamp INTEGER NOT NULL,
	note TEXT,
	session_key TEXT,
	override_reason TEXT,
	updated_at INTEGER
);

CREATE TABLE audit_events (
	id TEXT PRIMARY KEY NOT NULL,
	entity_type TEXT NOT NULL,
	entity_id TEXT,
	action TEXT NOT NULL,
	summary TEXT NOT NULL,
	before_json TEXT,
	after_json TEXT,
	metadata_json TEXT,
	created_at INTEGER NOT NULL,
	actor TEXT NOT NULL DEFAULT 'admin'
);

CREATE TABLE attendance_event_audit (
	id TEXT PRIMARY KEY NOT NULL,
	event_id TEXT,
	student_id TEXT NOT NULL,
	class_id TEXT,
	session_key TEXT,
	action TEXT NOT NULL CHECK(action IN ('create_override', 'update', 'delete')),
	reason TEXT NOT NULL,
	before_json TEXT,
	after_json TEXT,
	created_at INTEGER NOT NULL,
	actor TEXT NOT NULL DEFAULT 'admin'
);

CREATE TABLE sf2_student_mappings (
	id TEXT PRIMARY KEY NOT NULL,
	student_id TEXT NOT NULL
);

CREATE TABLE sf2_month_student_mappings (
	template_id TEXT NOT NULL,
	student_id TEXT NOT NULL,
	workbook_name TEXT NOT NULL,
	normalized_name TEXT NOT NULL,
	row_index INTEGER NOT NULL,
	gender_block TEXT,
	sf2_learner_id TEXT,
	PRIMARY KEY(template_id, student_id),
	UNIQUE(template_id, normalized_name)
);

CREATE TABLE sf2_templates (
	id TEXT PRIMARY KEY NOT NULL,
	active_class_id TEXT
);

CREATE TABLE attendance_day_status (
	id TEXT PRIMARY KEY NOT NULL,
	class_id TEXT
);
`;

let driver: SqlDriver;

export function useTestDb(): void {
	beforeEach(async () => {
		driver = new NodeSqlDriver();
		await driver.script(SCHEMA);
		useDriver(driver);
	});

	afterEach(async () => {
		useDriver(null);
		await driver.close();
	});
}

/** The driver the current test is running against, for direct SQL assertions. */
export function db(): SqlDriver {
	return driver;
}
