-- v10 - the SF2 form metadata that used to be keyed off a workbook template.
-- Extracted from the Rust migrate_to_v10.
ALTER TABLE settings ADD COLUMN school_id TEXT;
ALTER TABLE settings ADD COLUMN school_name TEXT;
ALTER TABLE settings ADD COLUMN school_year TEXT;
ALTER TABLE settings ADD COLUMN report_month TEXT;
ALTER TABLE settings ADD COLUMN grade_level TEXT;
ALTER TABLE settings ADD COLUMN section TEXT;
ALTER TABLE settings ADD COLUMN adviser_name TEXT;
ALTER TABLE settings ADD COLUMN school_head_name TEXT;