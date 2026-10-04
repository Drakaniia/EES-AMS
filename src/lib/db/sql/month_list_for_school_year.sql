-- All 12 month rows of one school year. Ordered in Rust by
-- (report_year, month number) so the list reads SEPTEMBER -> AUGUST; an
-- alphabetical month order would be misleading.
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
ORDER BY report_year ASC;
