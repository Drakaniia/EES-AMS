-- The row for (class, school year, month) - the read that a month switch is.
SELECT
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
    first_school_day_override,
    imported_at,
    last_synced_at,
    workbook_x_count,
    workbook_scanned_at
FROM sf2_month_templates
WHERE active_class_id = ?1
  AND school_year = ?2
  AND report_month = ?3;
