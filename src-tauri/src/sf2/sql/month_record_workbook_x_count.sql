-- Store the last measured X count for a month file and when it was measured.
--
-- `workbook_scanned_at` is what tells the guard (spec §9.1, edge case E4) that
-- a file is `Unmeasured` rather than empty: a NULL here means "never counted",
-- and an absent file is never counted.
UPDATE sf2_month_templates
SET
    workbook_x_count = ?2,
    workbook_scanned_at = ?3
WHERE id = ?1;
