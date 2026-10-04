-- Record - or clear - the real date classes started (spec D16).
--
-- `NULL` means nobody has been entered it yet, which is edge case E3: the split
-- falls back to the legacy per-month day and the app prompts once. A guessed
-- default would silently mis-date every month of the year.
--
-- `UPDATE`, not `REPLACE`: the settings row also holds `last_report_month` and
-- `sf2_split_completed_at`, and a whole-row write is how a school start date
-- ends up costing the user their split state.
--
-- Guarded on the row existing. A database with no settings row is reported by
-- the caller rather than silently grown here - `init_db` seeds it, and a
-- missing row means the schema is not what this code was written against.
UPDATE settings
SET school_start_date = ?1
WHERE id = 'app'