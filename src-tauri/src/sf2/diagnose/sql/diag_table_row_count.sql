-- Row count of one table.
--
-- `{table}` is substituted from a hard-coded literal in `db_read.rs`, never
-- from user input and never from a value read out of the database. This is the
-- same substitution contract as `sf2/sql/row_count.sql`.
SELECT COUNT(*) FROM "{table}";
