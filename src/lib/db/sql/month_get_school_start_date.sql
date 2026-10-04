-- D16: the one date the per-month `first_school_day` is derived from.
--
-- Read as `TEXT` because the column holds 'YYYY-MM-DD'. `NULL` - and only
-- `NULL` - means the user has not entered it yet, which is edge case E3: the
-- split then falls back to the legacy per-month day and the app prompts once.
--
-- A missing settings row reads as `NULL` rather than erroring, so the split
-- degrades to the E3 fallback instead of failing the whole job.
SELECT school_start_date
FROM settings
WHERE id = 'app'
