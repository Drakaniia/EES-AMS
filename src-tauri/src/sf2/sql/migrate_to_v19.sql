-- v19 — `sf2_month_templates`: one row per month workbook file.
--
-- Replaces the "one template per class with a mutable `report_month`" model
-- (spec D2/D4). The UNIQUE constraint is what makes two months being selected
-- at once impossible by construction (edge case E15), and dropping
-- `sheet_name` from the date mappings in v21 is only sound because a month
-- file holds exactly one month tab.

CREATE TABLE IF NOT EXISTS sf2_month_templates (
    id                TEXT PRIMARY KEY NOT NULL,
    active_class_id   TEXT NOT NULL,
    school_year       TEXT NOT NULL,
    report_month      TEXT NOT NULL,          -- 'SEPTEMBER'
    report_year       INTEGER NOT NULL,       -- 2026
    source_path       TEXT NOT NULL,
    source_hash       TEXT NOT NULL,
    school_id         TEXT,
    school_name       TEXT,
    grade_level       TEXT,
    section           TEXT,
    adviser_name      TEXT,
    school_head_name  TEXT,
    first_school_day  INTEGER NOT NULL,       -- per-month, derived or overridden
    imported_at       INTEGER NOT NULL,
    last_synced_at    INTEGER,
    workbook_x_count  INTEGER NOT NULL DEFAULT 0,   -- last observed X count in the file
    workbook_scanned_at INTEGER,                    -- when workbook_x_count was measured
    UNIQUE(active_class_id, school_year, report_month),
    FOREIGN KEY(active_class_id) REFERENCES classes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sf2_month_templates_class
    ON sf2_month_templates(active_class_id, school_year);
