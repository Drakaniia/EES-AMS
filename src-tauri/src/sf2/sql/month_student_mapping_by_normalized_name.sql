SELECT
    template_id,
    student_id,
    workbook_name,
    normalized_name,
    row_index,
    gender_block,
    sf2_learner_id
FROM sf2_month_student_mappings
WHERE template_id = ?1
  AND normalized_name = ?2;
