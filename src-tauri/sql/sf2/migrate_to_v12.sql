ALTER TABLE sf2_templates ADD COLUMN school_id TEXT NOT NULL DEFAULT '';
ALTER TABLE sf2_templates ADD COLUMN school_name TEXT NOT NULL DEFAULT '';
ALTER TABLE sf2_templates ADD COLUMN report_month TEXT NOT NULL DEFAULT '';
ALTER TABLE sf2_templates ADD COLUMN adviser_name TEXT NOT NULL DEFAULT '';
ALTER TABLE sf2_templates ADD COLUMN school_head_name TEXT NOT NULL DEFAULT '';

UPDATE sf2_templates
SET
    school_id = COALESCE((SELECT school_id FROM settings WHERE id = 'app'), ''),
    school_name = COALESCE((SELECT school_name FROM settings WHERE id = 'app'), ''),
    report_month = COALESCE((SELECT report_month FROM settings WHERE id = 'app'), ''),
    adviser_name = COALESCE((SELECT adviser_name FROM settings WHERE id = 'app'), ''),
    school_head_name = COALESCE((SELECT school_head_name FROM settings WHERE id = 'app'), '')
WHERE
    school_id = ''
    AND school_name = ''
    AND report_month = ''
    AND adviser_name = ''
    AND school_head_name = '';
