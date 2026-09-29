-- How many rows `events` holds, split by event type.
--
-- Read-only. The absent count is the single number a write path should never
-- shrink without the user asking (spec §9.4's db_fingerprint), so it is
-- reported here next to the total rather than only as a per-month figure that
-- could hide a deletion inside a month nobody was looking at.
SELECT
    event_type,
    COUNT(*) AS rows
FROM events
GROUP BY event_type
ORDER BY event_type;
