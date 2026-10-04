UPDATE sf2_templates
SET
    source_path = ?2,
    source_hash = ?3,
    school_id = ?4,
    school_name = ?5,
    school_year = ?6,
    report_month = ?7,
    grade_level = ?8,
    section = ?9,
    adviser_name = ?10,
    school_head_name = ?11,
    layout_fingerprint = ?12,
    active_class_id = ?13,
    imported_at = ?14,
    last_synced_at = ?15
WHERE id = ?1;
