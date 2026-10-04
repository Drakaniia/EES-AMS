-- v3 - the active quarter. Extracted from the Rust migrate_to_v3.
--
-- A no-op in practice: v1 already creates settings.quarter, so the runner's
-- add-column guard skips this on every database, fresh or migrated. Kept so the
-- version chain has no hole - a database stamped v2 still steps through v3.
ALTER TABLE settings ADD COLUMN quarter TEXT NOT NULL DEFAULT '1st Quarter';