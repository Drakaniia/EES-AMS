-- v1 - the initial schema: classes, students, events, settings.
--
-- Extracted from the `r#"..."#` literal in the Rust `migrate_to_v1`. The
-- `students_new`/`events_new` names and the v1 `CHECK(event_type IN ('in'))`
-- are what later versions rebuild from, so they must not be "tidied up" here:
-- v11 and v17 each rewrite these tables from their own shape.

-- Create classes table
CREATE TABLE IF NOT EXISTS classes (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    day_start TEXT NOT NULL,
    day_end TEXT NOT NULL,
    late_after TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Create indexes for classes
CREATE INDEX IF NOT EXISTS idx_classes_name ON classes(name);

-- Create students table with class support
CREATE TABLE IF NOT EXISTS students_new (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    card_serial TEXT UNIQUE,
    class_id TEXT,
    created_at INTEGER NOT NULL
);

-- Create indexes for students
CREATE INDEX IF NOT EXISTS idx_students_card_new ON students_new(card_serial);
CREATE INDEX IF NOT EXISTS idx_students_name_new ON students_new(name);
CREATE INDEX IF NOT EXISTS idx_students_class_new ON students_new(class_id);

-- Create events table with class support
CREATE TABLE IF NOT EXISTS events_new (
    id TEXT PRIMARY KEY NOT NULL,
    student_id TEXT NOT NULL,
    class_id TEXT,
    event_type TEXT NOT NULL CHECK(event_type IN ('in')),
    timestamp INTEGER NOT NULL,
    note TEXT,
    FOREIGN KEY (student_id) REFERENCES students_new(id) ON DELETE CASCADE
);

-- Create indexes for events
CREATE INDEX IF NOT EXISTS idx_events_student_new ON events_new(student_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp_new ON events_new(timestamp);

-- Create settings table
CREATE TABLE IF NOT EXISTS settings (
    id TEXT PRIMARY KEY NOT NULL,
    day_start TEXT NOT NULL,
    day_end TEXT NOT NULL,
    late_after TEXT NOT NULL,
    quarter TEXT NOT NULL DEFAULT '1st Quarter'
);

-- Insert default settings
INSERT OR IGNORE INTO settings (id, day_start, day_end, late_after, quarter)
VALUES ('app', '08:00', '15:00', '08:45', '1st Quarter');

-- name: legacy_copy
-- Copy the pre-v1 tables across, but only when there is one: this file also runs
-- on a brand-new database, where `students` and `events` do not exist yet and
-- SQLite would refuse to prepare the SELECT. The runner gates this group on the
-- old table being present.
--
-- Only 'in' events survive, because at v1 that was the only event type the
-- schema knew about.
INSERT INTO students_new (id, name, card_serial, created_at)
 SELECT id, name, card_serial, created_at FROM students;

INSERT INTO events_new (id, student_id, event_type, timestamp, note)
 SELECT id, student_id, event_type, timestamp, note FROM events
 WHERE event_type = 'in';

-- Drop old tables
DROP TABLE IF EXISTS students;
DROP TABLE IF EXISTS events;

-- name: finalize
-- Promote the new tables and swap the indexes onto their real names. Runs on
-- every path, legacy or fresh.
ALTER TABLE students_new RENAME TO students;
ALTER TABLE events_new RENAME TO events;

DROP INDEX IF EXISTS idx_students_card_new;
DROP INDEX IF EXISTS idx_students_name_new;
DROP INDEX IF EXISTS idx_students_class_new;
DROP INDEX IF EXISTS idx_events_student_new;
DROP INDEX IF EXISTS idx_events_timestamp_new;

CREATE INDEX IF NOT EXISTS idx_students_card ON students(card_serial);
CREATE INDEX IF NOT EXISTS idx_students_name ON students(name);
CREATE INDEX IF NOT EXISTS idx_students_class ON students(class_id);
CREATE INDEX IF NOT EXISTS idx_events_student ON events(student_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);