-- One month of a legacy template's day-number grid.
--
-- `sf2_date_mappings` is keyed by a full `YYYY-MM-DD` and is NOT scoped to a
-- month: a template that was analysed in several months holds rows for all of
-- them. Reading the whole table and letting the caller sort it out is how a
-- September grid ends up wearing October's columns - the grid then claims days
-- it has no mapping for, which is the one failure mode this whole read exists to
-- avoid.
--
-- So the month is a *range*: `[YYYY-MM-01, YYYY-MM-<last day>]`, closed on both
-- ends. `date` is fixed-width ISO-8601, so the string comparison is chronological
-- and no date function is needed - and no index has to be trusted to agree.
--
-- The GLOB is the same well-formedness guard `migrate_to_v22.sql` uses: a
-- malformed stored date is not a date, and letting one into a range comparison
-- would be a silent off-by-one on the boundary rows.
SELECT template_id, sheet_name, date, column_letter, column_index
FROM sf2_date_mappings
WHERE template_id = ?1
  AND date >= ?2
  AND date <= ?3
  AND date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
ORDER BY date ASC;
