PRAGMA foreign_keys = OFF;

DELETE FROM events
WHERE event_type <> 'in';

DROP TABLE IF EXISTS students_v11;

CREATE TABLE students_v11 (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    card_serial TEXT UNIQUE,
    class_id TEXT,
    created_at INTEGER NOT NULL
);

INSERT INTO students_v11 (id, name, card_serial, class_id, created_at)
SELECT id, name, card_serial, class_id, created_at
FROM students;

DROP TABLE students;
ALTER TABLE students_v11 RENAME TO students;

DROP INDEX IF EXISTS idx_students_card;
DROP INDEX IF EXISTS idx_students_name;
DROP INDEX IF EXISTS idx_students_class;

CREATE INDEX IF NOT EXISTS idx_students_card ON students(card_serial);
CREATE INDEX IF NOT EXISTS idx_students_name ON students(name);
CREATE INDEX IF NOT EXISTS idx_students_class ON students(class_id);

DROP TABLE IF EXISTS events_v11;

CREATE TABLE events_v11 (
    id TEXT PRIMARY KEY NOT NULL,
    student_id TEXT NOT NULL,
    class_id TEXT,
    event_type TEXT NOT NULL CHECK(event_type IN ('in')),
    timestamp INTEGER NOT NULL,
    note TEXT,
    FOREIGN KEY (student_id) REFERENCES students(id) ON DELETE CASCADE
);

INSERT INTO events_v11 (id, student_id, class_id, event_type, timestamp, note)
SELECT id, student_id, class_id, event_type, timestamp, note
FROM events
WHERE event_type = 'in';

DROP TABLE events;
ALTER TABLE events_v11 RENAME TO events;

DROP INDEX IF EXISTS idx_events_student;
DROP INDEX IF EXISTS idx_events_timestamp;

CREATE INDEX IF NOT EXISTS idx_events_student ON events(student_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);

PRAGMA foreign_keys = ON;
