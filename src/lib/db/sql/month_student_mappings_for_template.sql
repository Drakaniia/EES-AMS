-- One month file's roster, in workbook row order.
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
ORDER BY row_index ASC;
