-- v2 - a room number per class. Extracted from the Rust migrate_to_v2.
ALTER TABLE classes ADD COLUMN room TEXT NOT NULL DEFAULT 'N/A';