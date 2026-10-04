-- v22 — settings for the per-month model, the D16 override column, and the
-- backfill of the legacy single-template row into the per-month tables.
--
-- `sf2_templates`, `sf2_student_mappings` and `sf2_date_mappings` are NOT
-- dropped here. They stay so an install that has not run the split yet keeps
-- working (edge case E14); a later migration drops them once the split is
-- verified.
--
-- ── How this file is executed ───────────────────────────────────────────────
-- `migrations.ts` splits it into statements and runs them in two groups.
--
-- 1. Unmarked statements are DDL and run once, in order. An
--    `ALTER TABLE ... ADD COLUMN` is applied only when the column is missing,
--    so a launch that died before `PRAGMA user_version` reached 22 can replay
--    this file without failing on a duplicate column.
--
-- 2. Statements under a `-- name: <label>` marker are the backfill. They run
--    once per calendar month with these placeholders substituted from
--    hard-coded month constants in `migrations.ts` — never from a workbook,
--    a settings row, or any other user input:
--
--      {month_name}    canonical uppercase month name, e.g. SEPTEMBER
--      {month_abbr}    the three-letter form a `report_month` value uses, SEP
--      {month_number}  zero-padded month number, e.g. 09
--
--    Each backfill statement is written so that re-running it is a no-op
--    (`INSERT OR IGNORE` against the primary key / unique constraints), which
--    is what makes the whole migration restartable.

ALTER TABLE settings ADD COLUMN school_start_date TEXT DEFAULT NULL;   -- 'YYYY-MM-DD'
ALTER TABLE settings ADD COLUMN last_report_month TEXT DEFAULT NULL;   -- 'SEPTEMBER'
ALTER TABLE settings ADD COLUMN sf2_split_completed_at INTEGER;

-- Provenance for D16. `first_school_day` stays NOT NULL because every reader
-- (and the workbook writer) needs one effective number with no branching; this
-- nullable column records whether that number was derived from
-- `school_start_date` (NULL) or typed by the user. Re-derivation updates
-- `first_school_day` only while this column is NULL, so an override can never
-- be silently overwritten.
ALTER TABLE sf2_month_templates ADD COLUMN first_school_day_override INTEGER;

-- ── Backfill, per legacy template, for that template's own report month ────
--
-- The legacy workbook is a single file per class with a mutable
-- `report_month`, so each legacy row has exactly one month to become. The
-- month is matched on the template's own `report_month` (falling back to the
-- value in `settings`) by three-letter prefix, so a stored 'SEPT. 2025' and a
-- stored 'SEPTEMBER' both land in the SEPTEMBER row; a stored 'YYYY-MM' is
-- matched on the month number.

-- name: template
INSERT OR IGNORE INTO sf2_month_templates (
    id,
    active_class_id,
    school_year,
    report_month,
    report_year,
    source_path,
    source_hash,
    school_id,
    school_name,
    grade_level,
    section,
    adviser_name,
    school_head_name,
    first_school_day,
    imported_at,
    last_synced_at
)
SELECT
    t.id,
    t.active_class_id,
    t.school_year,
    '{month_name}',
    -- The school year wraps: September..December is the start year, the rest
    -- of the months is the following calendar year. The 9 is September - see
    -- `first-school-day.ts`'s `SCHOOL_YEAR_START_MONTH`, which is the same rule
    -- in TypeScript (`reportYearForSchoolMonth`).
    CAST(SUBSTR(t.school_year, 1, 4) AS INTEGER)
        + CASE WHEN {month_number} >= 9 THEN 0 ELSE 1 END,
    t.source_path,
    t.source_hash,
    NULLIF(t.school_id, ''),
    NULLIF(t.school_name, ''),
    NULLIF(t.grade_level, ''),
    NULLIF(t.section, ''),
    NULLIF(t.adviser_name, ''),
    NULLIF(t.school_head_name, ''),
    -- The legacy first attendance day for this month: the earliest Monday-Friday
    -- date the legacy analysis recorded inside the month. 0 means "not derived
    -- yet" - the month had no recorded dates - and the split job replaces it.
    COALESCE(
        (
            SELECT MIN(CAST(SUBSTR(d.date, 9, 2) AS INTEGER))
            FROM sf2_date_mappings AS d
            WHERE d.template_id = t.id
              AND SUBSTR(d.date, 6, 2) = '{month_number}'
              AND d.date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
              AND CAST(strftime('%w', d.date) AS INTEGER) BETWEEN 1 AND 5
        ),
        0
    ),
    t.imported_at,
    t.last_synced_at
FROM sf2_templates AS t
WHERE EXISTS (SELECT 1 FROM classes AS c WHERE c.id = t.active_class_id)
  AND SUBSTR(t.school_year, 1, 4) GLOB '[0-9][0-9][0-9][0-9]'
  AND CAST(SUBSTR(t.school_year, 1, 4) AS INTEGER) BETWEEN 1900 AND 2999
  AND (
        UPPER(
            COALESCE(
                NULLIF(TRIM(t.report_month), ''),
                (SELECT UPPER(NULLIF(TRIM(s.report_month), '')) FROM settings AS s WHERE s.id = 'app')
            )
        ) LIKE '%{month_abbr}%'
     OR SUBSTR(
            COALESCE(
                NULLIF(TRIM(t.report_month), ''),
                (SELECT TRIM(s.report_month) FROM settings AS s WHERE s.id = 'app')
            ), 6, 2
        ) = '{month_number}'
  );

-- name: students
INSERT OR IGNORE INTO sf2_month_student_mappings (
    template_id,
    student_id,
    workbook_name,
    normalized_name,
    row_index,
    gender_block,
    sf2_learner_id
)
SELECT
    s.template_id,
    s.student_id,
    s.workbook_name,
    s.normalized_name,
    s.row_index,
    s.gender_block,
    -- The DepEd ID was never read from the workbook (spec §6.3), so the legacy
    -- mappings have none of their own. Whatever a student already carries is
    -- the honest value to copy; the rest is backfilled during the split.
    st.sf2_learner_id
FROM sf2_student_mappings AS s
JOIN sf2_month_templates AS m
    ON m.id = s.template_id
   AND m.report_month = '{month_name}'
LEFT JOIN students AS st
    ON st.id = s.student_id;

-- name: dates
INSERT OR IGNORE INTO sf2_month_date_mappings (
    template_id,
    date,
    column_letter,
    column_index
)
SELECT
    d.template_id,
    d.date,
    d.column_letter,
    d.column_index
FROM sf2_date_mappings AS d
JOIN sf2_month_templates AS m
    ON m.id = d.template_id
   AND m.report_month = '{month_name}'
WHERE SUBSTR(d.date, 6, 2) = '{month_number}';

-- name: students_expected
-- Rows this month's student copy must produce. Counted before the copy is
-- asserted, so a mapping silently dropped by the copy is an error and not a
-- surprise discovered the first time a month is opened.
SELECT COUNT(*)
FROM sf2_student_mappings AS s
JOIN sf2_month_templates AS m
    ON m.id = s.template_id
   AND m.report_month = '{month_name}';

-- name: students_actual
SELECT COUNT(*)
FROM sf2_month_student_mappings AS sm
JOIN sf2_month_templates AS m
    ON m.id = sm.template_id
   AND m.report_month = '{month_name}';

-- name: dates_expected
SELECT COUNT(*)
FROM sf2_date_mappings AS d
JOIN sf2_month_templates AS m
    ON m.id = d.template_id
   AND m.report_month = '{month_name}'
WHERE SUBSTR(d.date, 6, 2) = '{month_number}';

-- name: dates_actual
SELECT COUNT(*)
FROM sf2_month_date_mappings AS dm
JOIN sf2_month_templates AS m
    ON m.id = dm.template_id
   AND m.report_month = '{month_name}'
WHERE SUBSTR(dm.date, 6, 2) = '{month_number}';
