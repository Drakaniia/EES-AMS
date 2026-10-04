-- v7 - clamp the quarter to the three periods the app supports.
-- Extracted from the Rust migrate_to_v7.
UPDATE settings
 SET quarter = '3rd Quarter'
 WHERE quarter NOT IN ('1st Quarter', '2nd Quarter', '3rd Quarter');