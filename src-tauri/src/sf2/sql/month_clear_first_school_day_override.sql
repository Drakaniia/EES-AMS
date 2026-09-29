-- Drop a per-month override and go back to the derived value.
UPDATE sf2_month_templates
SET
    first_school_day_override = NULL,
    first_school_day = ?2
WHERE id = ?1;
