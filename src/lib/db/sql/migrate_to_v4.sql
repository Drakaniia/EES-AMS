-- v4 - the six quarter start/end dates. Extracted from the Rust migrate_to_v4.
ALTER TABLE settings ADD COLUMN q1_start TEXT;
ALTER TABLE settings ADD COLUMN q1_end TEXT;
ALTER TABLE settings ADD COLUMN q2_start TEXT;
ALTER TABLE settings ADD COLUMN q2_end TEXT;
ALTER TABLE settings ADD COLUMN q3_start TEXT;
ALTER TABLE settings ADD COLUMN q3_end TEXT;