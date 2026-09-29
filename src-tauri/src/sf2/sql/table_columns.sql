-- Column-list probe for the table-rebuild assertions in
-- `src-tauri/src/infrastructure/database/migrations.rs`.
--
-- `{table}` is substituted by the caller before the query runs. The value always
-- comes from a hard-coded literal in `migrations.rs` - never from user input.
SELECT name
FROM pragma_table_info('{table}');
