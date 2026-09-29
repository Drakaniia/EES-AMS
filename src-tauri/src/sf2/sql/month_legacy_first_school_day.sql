-- E3 fallback: the legacy workbook's own first attendance day for one month.
--
-- This is the value the split uses when `school_start_date` has not been
-- entered yet. It is *read from the legacy data*, never invented: the earliest
-- Monday-Friday date the legacy analysis already recorded inside the month.
--
-- Mirrors the identical expression in `migrate_to_v22.sql`, so the day the
-- v22 backfill stored on the row and the day the split would derive agree.
-- `NULL` means the legacy workbook holds no usable date for that month - the
-- month had no school days - and the split reports the month instead of
-- guessing (E2).
SELECT MIN(CAST(SUBSTR(d.date, 9, 2) AS INTEGER)) AS first_school_day
FROM sf2_date_mappings AS d
WHERE d.template_id = ?1
  AND SUBSTR(d.date, 6, 2) = ?2
  AND d.date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
  AND CAST(strftime('%w', d.date) AS INTEGER) BETWEEN 1 AND 5
