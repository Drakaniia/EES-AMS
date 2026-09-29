-- The legacy roster: which student sits in which workbook row, per template.
--
-- Read-only. These rows are the only thing that can say "the X in row 9 is this
-- child", so the diagnostic falls back to them when the per-month table is
-- empty (spec §0 A3: an existing install's data lives here).
--
-- The column list is deliberately identical, in the same order, to
-- `diag_month_student_mappings.sql`: one Rust reader serves both, and a file
-- that grew or reordered a column would otherwise fail with a column *type*
-- error about a column the caller never asked for.
SELECT
    student_id,
    workbook_name,
    row_index,
    gender_block
FROM sf2_student_mappings
WHERE template_id = ?1
ORDER BY row_index;
