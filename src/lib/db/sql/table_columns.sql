-- Column-list probe for the table-rebuild assertions in `migrations.ts`.
--
-- `{table}` is substituted by the caller before the query runs. The value always
-- comes from a hard-coded literal in `migrations.ts` - never from user input.
SELECT name
FROM pragma_table_info('{table}');
