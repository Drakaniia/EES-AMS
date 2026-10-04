-- Record a per-month first-school-day override (D16).
--
-- Both columns move together: `first_school_day` is the effective value every
-- reader uses, `first_school_day_override` is the provenance that keeps a
-- later re-derivation from touching it.
UPDATE sf2_month_templates
SET
    first_school_day_override = ?2,
    first_school_day = ?2
WHERE id = ?1;
