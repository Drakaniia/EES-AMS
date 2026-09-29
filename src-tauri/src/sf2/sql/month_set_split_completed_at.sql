-- D14 / E12: record that the split finished.
--
-- Written only once all twelve month files have been built AND verified, which
-- is what makes a split interrupted by a full disk resume instead of corrupt:
-- a run that dies part-way leaves this `NULL` and the next run rebuilds the
-- months that never completed.
--
-- The row is addressed by id so the update can never create a second settings
-- row; zero rows changed means there is no settings row to record against and
-- the caller treats that as a failure rather than as "done".
UPDATE settings
SET sf2_split_completed_at = ?1
WHERE id = 'app'
