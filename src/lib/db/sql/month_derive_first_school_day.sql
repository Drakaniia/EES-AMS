-- Write a *derived* per-month first-school-day (D16).
--
-- The `first_school_day_override IS NULL` predicate is the whole point: a
-- re-derivation must never overwrite a value the user typed, and it reports
-- that by updating zero rows. The `?2 > 0` predicate refuses to write the
-- "not derived yet" sentinel over a real value.
UPDATE sf2_month_templates
SET first_school_day = ?2
WHERE id = ?1
  AND first_school_day_override IS NULL
  AND ?2 > 0;
