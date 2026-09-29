-- v20 — `sf2_month_student_mappings`: one roster per month file, plus the
-- DepEd learner ID (spec §6.3).

CREATE TABLE IF NOT EXISTS sf2_month_student_mappings (
    template_id     TEXT NOT NULL,
    student_id      TEXT NOT NULL,
    workbook_name   TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    row_index       INTEGER NOT NULL,
    gender_block    TEXT,
    sf2_learner_id  TEXT,                      -- NEW — see §6.3
    PRIMARY KEY(template_id, student_id),
    UNIQUE(template_id, normalized_name),
    FOREIGN KEY(template_id) REFERENCES sf2_month_templates(id) ON DELETE CASCADE,
    FOREIGN KEY(student_id)  REFERENCES students(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sf2_month_student_mappings_student
    ON sf2_month_student_mappings(student_id);

-- The same identity, stored on the student so a roster reshuffle between two
-- month files can be resolved without the workbook (E7). Nullable on purpose:
-- every existing row keeps working, and the split job backfills it because the
-- split already opens every month file and re-derives the roster.
--
-- Not a UNIQUE index on purpose. A duplicate ID is a data problem the split
-- must report, not a migration that refuses to start the app; the write path
-- in `sf2::month::student_repo` refuses to overwrite a different student's ID.
ALTER TABLE students ADD COLUMN sf2_learner_id TEXT;

CREATE INDEX IF NOT EXISTS idx_students_sf2_learner_id
    ON students(sf2_learner_id);
