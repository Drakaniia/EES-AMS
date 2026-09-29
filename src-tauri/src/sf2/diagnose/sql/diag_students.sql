-- Every student, with the class they belong to.
--
-- Read-only. Needed for two things the mapping tables cannot answer:
-- which absences belong to this workbook's class, and which workbook row a
-- learner name refers to when a worksheet's own roster has to be matched
-- against the database by name.
SELECT id, name, class_id FROM students ORDER BY name;
