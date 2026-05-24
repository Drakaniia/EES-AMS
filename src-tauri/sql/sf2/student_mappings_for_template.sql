SELECT template_id, student_id, workbook_name, normalized_name, row_index, gender_block
FROM sf2_student_mappings
WHERE template_id = ?1
ORDER BY row_index ASC;
