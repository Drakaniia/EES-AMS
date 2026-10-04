-- v5 - per-class sessions. The column itself is extracted from the Rust migrate_to_v5;
-- the backfill that gives every existing class a 'Full Day' session needs the
-- class's own day_start/day_end/late_after values, so it stays in TypeScript.
ALTER TABLE classes ADD COLUMN sessions TEXT;