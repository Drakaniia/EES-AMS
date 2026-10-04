-- The two v22 settings the per-month model depends on.
--
--   school_start_date  'YYYY-MM-DD', the real date classes started (D16).
--                      NULL stays NULL: an unset start date is unset, and the
--                      caller prompts once (E3) rather than guessing.
--   last_report_month  the month to fall back on when today's month has no
--                      file (D5, edge case E1). NULL on a fresh install.
--
-- A missing `settings` row reads as two NULLs rather than an error: a database
-- mid-migration has no row yet, and "we do not know" is the answer the caller
-- already has to handle.
SELECT
    NULLIF(TRIM(school_start_date), '') AS school_start_date,
    NULLIF(TRIM(last_report_month), '')  AS last_report_month
FROM settings
WHERE id = 'app';
