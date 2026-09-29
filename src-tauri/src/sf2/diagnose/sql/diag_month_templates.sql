-- Every per-month template row, in the order the school year runs.
--
-- Read-only. One row per month workbook (spec §6.2 v19); the split that would
-- create the other eleven rows is not wired to anything, so this table is
-- expected to hold a single month on an existing install, and the row count is
-- part of the diagnostic's answer.
SELECT
    id,
    active_class_id,
    school_year,
    report_month,
    report_year,
    source_path,
    first_school_day,
    last_synced_at,
    workbook_x_count,
    workbook_scanned_at
FROM sf2_month_templates
ORDER BY report_year, report_month;
