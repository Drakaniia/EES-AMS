-- Every absence the database holds, with the local calendar date it falls on.
--
-- Read-only. `event_type = 'absent'` is the only thing that is an X mark
-- (spec §3.2): a present student is a blank cell and has no row here, so this
-- result is the complete set of absences, not a sample.
--
-- The date is derived exactly the way `sf2::attendance::local_event_date`
-- derives it in Rust - `timestamp` is UTC seconds, the school day is the local
-- calendar day - so the months this query reports are the months the writer
-- would have written marks into. Using SQLite's own `localtime` modifier
-- instead of grouping in Rust keeps the whole read on one read-only
-- connection and one pass over the table.
--
-- `class_id` comes back so the caller can separate "absences for the class this
-- workbook is for" from "absences recorded against some other class". The
-- first is what a workbook comparison is about; the second is still worth
-- reporting, because an absence with no class is an absence nothing will ever
-- place on a grid.
SELECT
    student_id,
    class_id,
    strftime('%Y-%m-%d', timestamp, 'unixepoch', 'localtime') AS local_date
FROM events
WHERE event_type = 'absent'
ORDER BY local_date, student_id;
