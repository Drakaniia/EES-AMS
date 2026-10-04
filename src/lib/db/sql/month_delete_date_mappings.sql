-- Drop one month file's day-number grid. Scoped to the template on purpose:
-- this is the delete that a re-analysis of one month must not widen into
-- "delete the class's calendar".
DELETE FROM sf2_month_date_mappings
WHERE template_id = ?1;
