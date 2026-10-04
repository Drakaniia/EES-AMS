-- Sidebar branding customization: logo path and custom title
ALTER TABLE settings ADD COLUMN branding_logo_path TEXT DEFAULT NULL;
ALTER TABLE settings ADD COLUMN branding_title TEXT DEFAULT 'EES AMS';
