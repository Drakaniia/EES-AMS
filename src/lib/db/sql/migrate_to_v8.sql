-- v8 - the attendance mode (manual marks vs card reader).
-- Extracted from the Rust migrate_to_v8. Both halves matter: the add-column
-- guard makes the DDL replayable, the UPDATE folds any value an older build
-- invented back onto 'manual' rather than letting it reach a CHECK-free column.
ALTER TABLE settings ADD COLUMN attendance_mode TEXT NOT NULL DEFAULT 'manual';

UPDATE settings
 SET attendance_mode = 'manual'
 WHERE attendance_mode NOT IN ('manual', 'card_reader');