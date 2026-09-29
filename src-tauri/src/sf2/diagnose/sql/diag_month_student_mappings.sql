-- The per-month roster: which student sits in which workbook row, per month.
--
-- Read-only. Preferred over `diag_legacy_student_mappings.sql` when it has rows
-- for a month; the legacy table is the fallback, never the other way round.
-- Same columns, same order, as the legacy file - one Rust reader serves both.
SELECT
    student_id,
    workbook_name,
    row_index,
    gender_block
FROM sf2_month_student_mappings
WHERE template_id = ?1
ORDER BY row_index;
