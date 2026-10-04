-- Drop one month file's roster, never the class's.
DELETE FROM sf2_month_student_mappings
WHERE template_id = ?1;
