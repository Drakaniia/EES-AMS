-- Column-existence probe for the idempotent `ALTER TABLE ... ADD COLUMN` guard
-- in `execute_migration_ddl` (`migrations.ts`).
--
-- `{table}` and `{column}` are both substituted by the caller from hard-coded
-- literals - either from that module or from an `ADD COLUMN` statement in a
-- migration file it runs. Never from user input.
SELECT COUNT(*)
FROM pragma_table_info('{table}')
WHERE name = '{column}';
