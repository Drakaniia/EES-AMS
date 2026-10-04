-- Row-count probe used by the table-rebuild assertions in `migrations.ts`.
--
-- `{table}` is substituted by the caller before the query runs. The value
-- always comes from a hard-coded literal in `migrations.ts` (or from a test in
-- `migrations.ts`'s colocated test module) - never from user input.
SELECT COUNT(*) FROM {table};
