-- D5: the month the app opens on the next launch.
--
-- The split writes the month the legacy workbook was on, so the first launch
-- after the upgrade lands on the same month the teacher was already using
-- rather than jumping to today's calendar month.
UPDATE settings
SET last_report_month = ?1
WHERE id = 'app'
