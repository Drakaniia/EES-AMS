-- v24 - give `sf2_month_date_mappings` its `sheet_name` back (spec section 0, A4).
--
-- v21 dropped this column on the reasoning that "a month file has exactly one
-- worksheet, so the sheet is derivable from the month row". The workbook model has
-- since been corrected (spec section 0): there is ONE `.xls` file holding TWELVE
-- month worksheets, each visible, each carrying its own day-number grid and its
-- own X marks. Deriving the sheet from the month row is therefore no longer
-- merely inconvenient - the derivation and the storage are the same value, so
-- keeping only the derivation is fine and keeping only the storage is fine, but a
-- reader that has to *write* a mark needs a column it can read back and compare.
-- The column comes back.
--
-- WHY v24 AND NOT v23
--
-- v23 is the canonical school-year label (`YYYY-YYYY`) backfill, landed in
-- parallel by the other workstream. Two workstreams reached for the same version
-- number; this one is second in the chain, so the order is
-- v22 -> v23 (school-year label) -> v24 (this). Nothing is lost by the ordering -
-- both migrations are additive and independent - and a database that already ran
-- v23 runs this one next.
--
-- SAFETY - this migration is strictly additive.
--
--   * Nothing is dropped. `sf2_templates`, `sf2_student_mappings` and
--     `sf2_date_mappings` are all still here and all still hold the user's data.
--   * No row is deleted and no row is blanked. Every statement is an `UPDATE`
--     that only ever writes `sheet_name`, and each one is guarded so a row it
--     cannot fill is left exactly as it was rather than set to an empty string.
--   * `sheet_name` is added as a NULLABLE column with no default, so adding it
--     cannot rewrite a single existing row's other values.
--
-- BACKFILL ORDER - the month row first, the legacy table second, the date third.
--
-- The legacy `sf2_date_mappings.sheet_name` is the name the analyzer saw on the
-- one sheet that was VISIBLE when the workbook was last analysed. Under the old
-- hide/rename/clear cycle that sheet was whatever month happened to be current,
-- which is not necessarily the month the row belongs to - copying it first would
-- put one month's sheet name on another month's grid. `sf2_month_templates` is
-- authoritative about which month a row is, so it is consulted first, and the
-- legacy table is only reached for rows whose month row is gone (an orphan left
-- by a row that was deleted), where any recorded sheet name beats none.
--
-- The third pass exists so that after this migration *no* row is left with a
-- blank sheet name: a date is a full `YYYY-MM-DD`, so the month and year can
-- always be read back off it. That makes `sheet_name` a total function of stored
-- data, which is what lets the read path trust it and fall back to the same
-- derivation when it ever sees a NULL.

ALTER TABLE sf2_month_date_mappings ADD COLUMN sheet_name TEXT;

-- 1. The month row knows its own month name and calendar year. This is the
--    authoritative answer, so it is written first.
UPDATE sf2_month_date_mappings
   SET sheet_name = (
        SELECT t.report_month || ' ' || t.report_year
          FROM sf2_month_templates AS t
         WHERE t.id = sf2_month_date_mappings.template_id
       )
 WHERE (sheet_name IS NULL OR TRIM(sheet_name) = '')
   AND EXISTS (
        SELECT 1
          FROM sf2_month_templates AS t
         WHERE t.id = sf2_month_date_mappings.template_id
           AND NULLIF(TRIM(t.report_month), '') IS NOT NULL
       );

-- 2. An orphan row - one whose month row is gone - takes whatever sheet name the
--    pre-split table recorded for the same template and date.
UPDATE sf2_month_date_mappings
   SET sheet_name = (
        SELECT d.sheet_name
          FROM sf2_date_mappings AS d
         WHERE d.template_id = sf2_month_date_mappings.template_id
           AND d.date = sf2_month_date_mappings.date
       )
 WHERE (sheet_name IS NULL OR TRIM(sheet_name) = '')
   AND EXISTS (
        SELECT 1
          FROM sf2_date_mappings AS d
         WHERE d.template_id = sf2_month_date_mappings.template_id
           AND d.date = sf2_month_date_mappings.date
           AND NULLIF(TRIM(d.sheet_name), '') IS NOT NULL
       );

-- 3. Last resort, from the date itself. `date` is stored as `YYYY-MM-DD`, so
--    SUBSTR(date, 6, 2) is the zero-padded month and SUBSTR(date, 1, 4) the
--    year. The names are the canonical full month names, which is what
--    `month_sheet_name` produces, so a value written here is the same string a
--    reader would have derived for itself.
UPDATE sf2_month_date_mappings
   SET sheet_name = CASE SUBSTR(date, 6, 2)
        WHEN '01' THEN 'JANUARY'
        WHEN '02' THEN 'FEBRUARY'
        WHEN '03' THEN 'MARCH'
        WHEN '04' THEN 'APRIL'
        WHEN '05' THEN 'MAY'
        WHEN '06' THEN 'JUNE'
        WHEN '07' THEN 'JULY'
        WHEN '08' THEN 'AUGUST'
        WHEN '09' THEN 'SEPTEMBER'
        WHEN '10' THEN 'OCTOBER'
        WHEN '11' THEN 'NOVEMBER'
        WHEN '12' THEN 'DECEMBER'
       END || ' ' || SUBSTR(date, 1, 4)
 WHERE (sheet_name IS NULL OR TRIM(sheet_name) = '')
   AND date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]';

-- The index the guard and the write path read a month through: "every mapped day
-- of this month, and which sheet each one lives on". Added alongside the column
-- because a twelve-sheet file resolves the sheet per month, and the per-month
-- reads are the hot path rather than the exception.
CREATE INDEX IF NOT EXISTS idx_sf2_month_date_mappings_sheet
    ON sf2_month_date_mappings(template_id, sheet_name);
