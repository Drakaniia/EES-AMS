SELECT id, source_path, source_hash, school_id, school_name, school_year,
       report_month, grade_level, section, adviser_name, school_head_name,
       layout_fingerprint, active_class_id, imported_at
FROM sf2_templates
WHERE active_class_id = ?1
ORDER BY imported_at DESC
LIMIT 1;
