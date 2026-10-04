-- Clear the mirror-freshness hint on every month of a class.
--
-- A grid correction changes the database without changing any file, so the next
-- workbook open has to believe the file is stale. This is scoped by class and
-- not by month on purpose: a correction lands in `events`, which no month owns,
-- and every one of the class's month files is now out of date with respect to it.
--
-- Only the timestamp moves. No mapping, no roster, and nothing on disk.
UPDATE sf2_month_templates
SET last_synced_at = NULL
WHERE active_class_id = ?1;
