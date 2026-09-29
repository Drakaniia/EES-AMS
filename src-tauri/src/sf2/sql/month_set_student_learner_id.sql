-- Store a DepEd learner ID on a student (spec §6.3).
--
-- Two refusals, both deliberate:
--   * the row keeps an ID some *other* learner already holds, because a
--     duplicate ID is a roster problem for the split to report, not a reason to
--     silently move one learner's identity onto another;
--   * the row keeps a different ID already set, because the workbook is the
--     authority for the month it was read from and re-reading it must be
--     idempotent.
-- Zero updated rows is the signal the caller turns into that report.
UPDATE students
SET sf2_learner_id = ?2
WHERE id = ?1
  AND (sf2_learner_id IS NULL OR sf2_learner_id = '' OR sf2_learner_id = ?2)
  AND NOT EXISTS (
      SELECT 1
      FROM students AS other
      WHERE other.id <> ?1
        AND other.sf2_learner_id = ?2
  );
