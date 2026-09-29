-- Does `name` exist as a table?
--
-- The diagnostic has to run against a database that predates the per-month
-- tables (v19-v22), so "the table is missing" is a normal answer here and must
-- be reported as such - never as a row count of zero, and never as an error.
SELECT COUNT(*)
FROM sqlite_master
WHERE type = 'table'
  AND name = ?1;
