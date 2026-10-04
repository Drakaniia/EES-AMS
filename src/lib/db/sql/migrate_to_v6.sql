-- v6 - which weekdays a class meets. The column is extracted from the Rust
-- migrate_to_v6; the Monday-Friday backfill stays in TypeScript because it
-- writes one value for every row.
ALTER TABLE classes ADD COLUMN days TEXT;