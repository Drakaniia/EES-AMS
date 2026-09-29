-- Guarded delete: absence is a first-class event type from v17 onwards, so a
-- database crossing v10 -> v11 that already carries 'absent' rows must keep
-- them. Only rows whose event_type is not a value this schema knows about are
-- removed; the historical unconditional `DELETE ... WHERE event_type <> 'in'`
-- destroyed every attendance mark of a newer build.
--
-- This runs as its own statement, *before* the v11 rebuild, because it is the
-- one place the v11 upgrade deliberately changes the row count. Bracketing the
-- rebuild with `assert_row_count_preserved` therefore has to start after this
-- delete, or the guard would reject the deletion it is meant to protect.
DELETE FROM events
WHERE event_type IS NULL
   OR event_type NOT IN ('in', 'absent');
