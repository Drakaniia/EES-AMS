-- Create the row for a month file.
--
-- The conflict target is (active_class_id, school_year, report_month) - the
-- constraint that makes two selected months impossible (E15). Three columns are
-- deliberately absent from the DO UPDATE list:
--
--   * first_school_day / first_school_day_override - a value already on the row
--     is either derived or typed by the user, and re-deriving must never
--     overwrite a typed day. Use the explicit first-school-day statements.
--   * last_synced_at - sync state, not metadata. Writing it back to NULL
--     because a caller happened to pass None would force a needless Excel
--     re-sync. Use set_last_synced_at to reset it.
--   * workbook_x_count / workbook_scanned_at - a measurement, and only
--     record_workbook_x_count may write one.
INSERT INTO sf2_month_templates (
    id,
    active_class_id,
    school_year,
    report_month,
    report_year,
    source_path,
    source_hash,
    school_id,
    school_name,
    grade_level,
    section,
    adviser_name,
    school_head_name,
    first_school_day,
    imported_at,
    last_synced_at,
    workbook_x_count,
    workbook_scanned_at
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
ON CONFLICT(active_class_id, school_year, report_month) DO UPDATE SET
    report_year = excluded.report_year,
    source_path = excluded.source_path,
    source_hash = excluded.source_hash,
    school_id = excluded.school_id,
    school_name = excluded.school_name,
    grade_level = excluded.grade_level,
    section = excluded.section,
    adviser_name = excluded.adviser_name,
    school_head_name = excluded.school_head_name,
    imported_at = excluded.imported_at;
