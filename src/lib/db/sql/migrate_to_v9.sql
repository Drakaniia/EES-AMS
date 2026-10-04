CREATE TABLE IF NOT EXISTS sf2_templates (
    id TEXT PRIMARY KEY NOT NULL,
    source_path TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    school_year TEXT NOT NULL,
    grade_level TEXT NOT NULL,
    section TEXT NOT NULL,
    layout_fingerprint TEXT NOT NULL,
    active_class_id TEXT NOT NULL,
    imported_at INTEGER NOT NULL,
    UNIQUE(source_hash, grade_level, section)
);

CREATE INDEX IF NOT EXISTS idx_sf2_templates_class ON sf2_templates(active_class_id);

CREATE TABLE IF NOT EXISTS sf2_student_mappings (
    template_id TEXT NOT NULL,
    student_id TEXT NOT NULL,
    workbook_name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    row_index INTEGER NOT NULL,
    gender_block TEXT,
    PRIMARY KEY(template_id, student_id),
    UNIQUE(template_id, normalized_name),
    FOREIGN KEY(template_id) REFERENCES sf2_templates(id) ON DELETE CASCADE,
    FOREIGN KEY(student_id) REFERENCES students(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sf2_student_mappings_student
ON sf2_student_mappings(student_id);

CREATE TABLE IF NOT EXISTS sf2_date_mappings (
    template_id TEXT NOT NULL,
    sheet_name TEXT NOT NULL,
    date TEXT NOT NULL,
    column_letter TEXT NOT NULL,
    column_index INTEGER NOT NULL,
    PRIMARY KEY(template_id, date),
    FOREIGN KEY(template_id) REFERENCES sf2_templates(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sf2_date_mappings_date
ON sf2_date_mappings(date);

CREATE TABLE IF NOT EXISTS attendance_day_status (
    class_id TEXT NOT NULL,
    date TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('closed')),
    closed_at INTEGER NOT NULL,
    PRIMARY KEY(class_id, date),
    FOREIGN KEY(class_id) REFERENCES classes(id) ON DELETE CASCADE
);
