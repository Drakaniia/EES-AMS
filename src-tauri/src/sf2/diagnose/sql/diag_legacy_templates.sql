-- Every legacy per-class SF2 template row.
--
-- Read-only. `sf2_templates` is the pre-split model: one row per class, with a
-- mutable `report_month`. The diagnostic reads it to learn which workbook file
-- the app believes it owns, which class that workbook is for, and the school
-- year the twelve months hang off.
SELECT
    id,
    source_path,
    school_year,
    report_month,
    active_class_id,
    grade_level,
    section,
    last_synced_at
FROM sf2_templates
ORDER BY imported_at;
