PRAGMA foreign_keys = OFF;

DROP TABLE IF EXISTS events_v17;

CREATE TABLE events_v17 (
    id TEXT PRIMARY KEY NOT NULL,
    student_id TEXT NOT NULL,
    class_id TEXT,
    event_type TEXT NOT NULL CHECK(event_type IN ('in', 'absent')),
    timestamp INTEGER NOT NULL,
    note TEXT,
    session_key TEXT,
    override_reason TEXT,
    updated_at INTEGER,
    FOREIGN KEY (student_id) REFERENCES students(id) ON DELETE CASCADE
);

INSERT INTO events_v17 (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
SELECT id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at
FROM events;

DROP TABLE events;
ALTER TABLE events_v17 RENAME TO events;

DROP INDEX IF EXISTS idx_events_student;
DROP INDEX IF EXISTS idx_events_timestamp;
DROP INDEX IF EXISTS idx_events_session_key;
DROP INDEX IF EXISTS idx_events_student_session;

CREATE INDEX IF NOT EXISTS idx_events_student ON events(student_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_session_key ON events(session_key);
CREATE INDEX IF NOT EXISTS idx_events_student_session ON events(student_id, session_key);

PRAGMA foreign_keys = ON;
