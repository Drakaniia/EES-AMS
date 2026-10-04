-- The newest school year on record for a class.
--
-- A month switch is keyed on (class, school year, month). When the caller does
-- not name a school year - which is the normal case on a fresh launch - the
-- month switch resolves it to the year the class actually has months for, so
-- the read lands on a real row instead of a guess.
--
-- Ordered on the four-digit start year rather than the label, so '2026-2027'
-- sorts above '2015-2016' instead of below it.
SELECT school_year
FROM sf2_month_templates
WHERE active_class_id = ?1
  AND school_year GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9]'
ORDER BY CAST(SUBSTR(school_year, 1, 4) AS INTEGER) DESC
LIMIT 1;
