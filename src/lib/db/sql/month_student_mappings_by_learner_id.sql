-- Every mapping that already knows this DepEd learner ID, across months.
--
-- This is the query that survives a roster reshuffle: a learner keeps the same
-- identity from one month file to the next even when the name is spelled
-- differently and the row moved (E7).
SELECT
    template_id,
    student_id,
    workbook_name,
    normalized_name,
    row_index,
    gender_block,
    sf2_learner_id
FROM sf2_month_student_mappings
WHERE sf2_learner_id = ?1
ORDER BY row_index ASC;
