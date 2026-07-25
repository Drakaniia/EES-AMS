ALTER TABLE events ADD COLUMN session_key TEXT;
ALTER TABLE events ADD COLUMN override_reason TEXT;
ALTER TABLE events ADD COLUMN updated_at INTEGER;

UPDATE events
SET session_key =
    strftime('%Y-%m-%d', timestamp, 'unixepoch', 'localtime')
    || '|'
    || COALESCE(NULLIF(class_id, ''), 'unassigned')
    || '|day'
WHERE session_key IS NULL OR trim(session_key) = '';

CREATE INDEX IF NOT EXISTS idx_events_session_key ON events(session_key);
CREATE INDEX IF NOT EXISTS idx_events_student_session ON events(student_id, session_key);

CREATE TABLE IF NOT EXISTS attendance_event_audit (
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
    actor TEXT NOT NULL DEFAULT 'admin',
    FOREIGN KEY (student_id) REFERENCES students(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_attendance_event_audit_event
    ON attendance_event_audit(event_id);
CREATE INDEX IF NOT EXISTS idx_attendance_event_audit_student
    ON attendance_event_audit(student_id);
CREATE INDEX IF NOT EXISTS idx_attendance_event_audit_created
    ON attendance_event_audit(created_at);
