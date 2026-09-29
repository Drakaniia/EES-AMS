-- The per-month day grid, grouped by the month it covers.
--
-- Read-only. Note what is *not* selected: `sf2_month_date_mappings` has no
-- `sheet_name` column (spec §0 A4 restores one), so the first column is an
-- empty literal rather than a real one. It is there so this file returns the
-- same five columns in the same order as
-- `diag_legacy_date_mappings_by_sheet.sql` - one Rust reader serves both, and a
-- mismatched column list fails with a column *type* error about a column the
-- caller never asked for.
--
-- A month's grid also cannot be tied back to a worksheet from the database
-- alone, which is one of the reasons the diagnostic reads the workbook's own
-- day-number row instead of trusting the stored grid.
SELECT
    '' AS sheet_name,
    substr(date, 1, 7) AS year_month,
    MIN(date) AS first_date,
    MAX(date) AS last_date,
    COUNT(*) AS day_columns
FROM sf2_month_date_mappings
WHERE template_id = ?1
GROUP BY year_month
ORDER BY year_month;
