-- Refresh a month file's metadata after it was re-opened.
--
-- Keyed on the row id, and it refuses to touch `first_school_day`,
-- `first_school_day_override`, `workbook_x_count` or `workbook_scanned_at`:
-- those are provenance and measurement, not metadata.
UPDATE sf2_month_templates
SET
    school_year = ?2,
    report_month = ?3,
    report_year = ?4,
    source_path = ?5,
    source_hash = ?6,
    school_id = ?7,
    school_name = ?8,
    grade_level = ?9,
    section = ?10,
    adviser_name = ?11,
    school_head_name = ?12,
    imported_at = ?13,
    last_synced_at = ?14
WHERE id = ?1;
