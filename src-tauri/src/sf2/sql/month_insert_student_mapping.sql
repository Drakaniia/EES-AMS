INSERT OR REPLACE INTO sf2_month_student_mappings (
    template_id,
    student_id,
    workbook_name,
    normalized_name,
    row_index,
    gender_block,
    sf2_learner_id
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);
