-- The legacy day grid, grouped by the sheet it was read off.
--
-- Read-only. The sheet name is the part that matters most here: a grid whose
-- `sheet_name` names a worksheet the file no longer contains is a grid the app
-- can no longer place a mark on, and the diagnostic has to be able to say so.
--
-- Same five columns in the same order as
-- `diag_month_date_mappings_by_month.sql`, for the same reason.
SELECT
    sheet_name,
    substr(date, 1, 7) AS year_month,
    MIN(date) AS first_date,
    MAX(date) AS last_date,
    COUNT(*) AS day_columns
FROM sf2_date_mappings
WHERE template_id = ?1
GROUP BY sheet_name, year_month
ORDER BY year_month, sheet_name;
