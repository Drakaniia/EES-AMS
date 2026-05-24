INSERT INTO sf2_templates (
    id,
    source_path,
    source_hash,
    school_id,
    school_name,
    school_year,
    report_month,
    grade_level,
    section,
    adviser_name,
    school_head_name,
    layout_fingerprint,
    active_class_id,
    imported_at
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
ON CONFLICT(source_hash, grade_level, section) DO UPDATE SET
    source_path = excluded.source_path,
    school_id = excluded.school_id,
    school_name = excluded.school_name,
    school_year = excluded.school_year,
    report_month = excluded.report_month,
    grade_level = excluded.grade_level,
    section = excluded.section,
    adviser_name = excluded.adviser_name,
    school_head_name = excluded.school_head_name,
    layout_fingerprint = excluded.layout_fingerprint,
    active_class_id = excluded.active_class_id,
    imported_at = excluded.imported_at;
