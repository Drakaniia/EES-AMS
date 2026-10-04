-- Does the student have an explicit record of one event type for one local day?
--
-- The predicate is deliberately identical to the one the pre-existing
-- `has_absent_event_for_day` used inline: a learner-day is "recorded" when any
-- event of that type falls inside the local day's bounds and belongs to the
-- class (an event with no class counts, because that is how a student marked
-- outside a class switch is stored).
--
-- `{event_type}` is substituted by the caller before the query runs. The value
-- always comes from `AttendanceType::as_db_value()` — a hard-coded literal in
-- Rust, never user input.
SELECT COUNT(*) FROM events
WHERE student_id = ?1
AND event_type = '{event_type}'
AND timestamp >= ?2
AND timestamp < ?3
AND (class_id IS NULL OR class_id = ?4);
