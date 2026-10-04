SELECT id, source_path, source_hash, school_id, school_name, school_year,
       report_month, grade_level, section, adviser_name, school_head_name,
       layout_fingerprint, active_class_id, imported_at, last_synced_at
FROM sf2_templates
WHERE source_hash = ?1 AND grade_level = ?2 AND section = ?3
LIMIT 1;
