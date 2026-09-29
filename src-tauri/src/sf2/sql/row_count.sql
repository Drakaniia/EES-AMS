-- Row-count probe used by the table-rebuild assertions in
-- `src-tauri/src/infrastructure/database/migrations.rs`.
--
-- `{table}` is substituted by the caller before the query runs. The value
-- always comes from a hard-coded literal in `migrations.rs` (or from a test in
-- `migrations.rs`'s colocated test module) - never from user input.
SELECT COUNT(*) FROM {table};
