-- D14: the guard that makes the split one-time.
--
-- `NULL` means the split has never completed - either it has never run, or it
-- was interrupted, or a month failed verification. All three are the same
-- state to the caller, and all three are resumed by running it again. The
-- value is written only after all twelve months verify (spec E12).
SELECT sf2_split_completed_at
FROM settings
WHERE id = 'app'
