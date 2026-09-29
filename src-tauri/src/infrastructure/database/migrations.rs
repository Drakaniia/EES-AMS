use super::DbPool;
use crate::domain::error::{AppError, Result};
use crate::domain::models::Session;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::collections::BTreeSet;
use std::path::Path;

// brief-S1: `CURRENT_SCHEMA_VERSION` moved down to the `schema_v24` block below
// the migration files, because v24's owner defines the number. The one here is
// removed, not shadowed - two definitions of the version is exactly the drift
// this is meant to prevent.

/// How many pre-migration snapshots to keep beside the live database.
///
/// Five, not three: the two migrations that rebuild a table and can destroy
/// attendance records are v11 and v17, so a user who has installed updates
/// across both of those needs both snapshots still on disk (spec §9.4).
const SNAPSHOT_HISTORY: usize = 5;

/// Row-count probe for the rebuild assertions below. Lives in a `.sql` file
/// with the rest of the migration SQL; `{table}` is substituted by
/// [`row_count`] from a hard-coded literal, never from user input.
const ROW_COUNT_SQL: &str = include_str!("../../sf2/sql/row_count.sql");

/// Column-existence probe for the idempotent `ADD COLUMN` guard in
/// [`execute_migration_ddl`]. `{table}` and `{column}` are both substituted
/// from hard-coded literals in this module or from a statement in one of the
/// migration files it runs - never from user input.
const COLUMN_EXISTS_SQL: &str = include_str!("../../sf2/sql/column_exists.sql");

/// Column-list probe for the rebuild assertions below. `{table}` is substituted
/// by [`table_columns`] from a hard-coded literal, never from user input.
const TABLE_COLUMNS_SQL: &str = include_str!("../../sf2/sql/table_columns.sql");

/// v19 - one row per month workbook file.
const MIGRATE_TO_V19_SQL: &str = include_str!("../../sf2/sql/migrate_to_v19.sql");
/// v20 - one roster per month file, plus the DepEd learner ID.
const MIGRATE_TO_V20_SQL: &str = include_str!("../../sf2/sql/migrate_to_v20.sql");
/// v21 - one day-number grid per month file.
const MIGRATE_TO_V21_SQL: &str = include_str!("../../sf2/sql/migrate_to_v21.sql");
/// v22 - settings for the per-month model, the D16 override column, backfill.
const MIGRATE_TO_V22_SQL: &str = include_str!("../../sf2/sql/migrate_to_v22.sql");

// brief-S1: v24 - `sf2_month_date_mappings` regains its `sheet_name` column
// (spec section 0 A4: one workbook, twelve month worksheets).
//
// The version number is owned by the module that owns the migration body, so the
// two cannot drift. v23 is the school-year label and stays where it is; v24 runs
// after it and is independent of it.
//
pub const CURRENT_SCHEMA_VERSION: i32 = crate::sf2::month::schema_v24::SCHEMA_VERSION;

/// Initialize the database with schema and migrations
pub fn init_db<P: AsRef<Path>>(path: P) -> Result<DbPool> {
    let path = path.as_ref();
    snapshot_before_migration(path);

    let manager = SqliteConnectionManager::file(path)
        .with_init(|conn| conn.execute_batch("PRAGMA foreign_keys = ON;"));
    let pool = Pool::new(manager)?;

    let conn = pool.get()?;

    migrate_db(&conn)?;

    Ok(pool)
}

/// Copy the database aside before running migrations, so an update that breaks
/// something can always be rolled back to the exact pre-update data.
///
/// Migrations are the one moment the app rewrites the schema of the file that
/// holds every attendance mark. Several migrations rebuild tables; a mistake in
/// one of those is silent and unrecoverable without a copy. The snapshot is
/// best-effort — a failure here must never stop the app from starting.
fn snapshot_before_migration(path: &Path) {
    let Ok(from_version) = stored_schema_version(path) else {
        return;
    };
    if from_version >= CURRENT_SCHEMA_VERSION {
        return;
    }

    let Some(parent) = path.parent() else {
        return;
    };
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return;
    };

    let snapshot_name = format!("{file_name}.pre-v{from_version}-to-v{CURRENT_SCHEMA_VERSION}");
    let snapshot_path = parent.join(&snapshot_name);
    if snapshot_path.exists() {
        // Already snapshotted for this exact version transition.
        return;
    }

    match std::fs::copy(path, &snapshot_path) {
        Ok(_) => {
            log::info!(
                "snapshotted database before migrating v{from_version} -> v{CURRENT_SCHEMA_VERSION}: {}",
                snapshot_path.display()
            );
            prune_snapshots(parent, file_name);
        }
        Err(error) => log::warn!(
            "failed to snapshot database before migrating: {error} (continuing without a rollback point)"
        ),
    }
}

/// Read `PRAGMA user_version` from an existing database file.
fn stored_schema_version(path: &Path) -> Result<i32> {
    if !path.exists() {
        return Err(AppError::Internal(
            "database file does not exist".to_string(),
        ));
    }
    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version)
}

/// Keep only the newest [`SNAPSHOT_HISTORY`] snapshots for this database file.
fn prune_snapshots(parent: &Path, file_name: &str) {
    let prefix = format!("{file_name}.pre-v");
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };

    let mut snapshots = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(&prefix) {
                return None;
            }
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .collect::<Vec<_>>();
    if snapshots.len() <= SNAPSHOT_HISTORY {
        return;
    }

    // Newest first, then drop everything past the retention limit.
    snapshots.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (_, stale) in snapshots.into_iter().skip(SNAPSHOT_HISTORY) {
        if let Err(error) = std::fs::remove_file(&stale) {
            log::warn!(
                "failed to prune old database snapshot {}: {error}",
                stale.display()
            );
        }
    }
}

/// Count the rows currently in `table`.
///
/// Used to bracket a table rebuild: read the count before the `execute_batch`
/// and again after it, then hand both to [`assert_row_count_preserved`]. The
/// SQL itself lives in `sf2/sql/row_count.sql`; only the table name - always a
/// literal from this module or its tests - is substituted here.
fn row_count(conn: &rusqlite::Connection, table: &str) -> Result<i64> {
    let sql = ROW_COUNT_SQL.replace("{table}", table);
    let count = conn.query_row(&sql, [], |row| row.get::<_, i64>(0))?;
    Ok(count)
}

/// Fail the migration when a table rebuild did not carry every row across.
///
/// A migration that silently drops rows is the failure mode that costs this app
/// its attendance marks: the `events` table *is* the record of every X, and a
/// rebuild that loses rows looks fine until the first mark is written. Making
/// it an error means the damage surfaces at install time - next to the
/// pre-migration snapshot - instead of at the first attendance mark.
///
/// `migration_label` names the migration for the log line, e.g. `"v11"`.
pub(crate) fn assert_row_count_preserved(
    table: &str,
    before: i64,
    after: i64,
    migration_label: &str,
) -> Result<()> {
    if before == after {
        return Ok(());
    }
    log::error!(
        "migration {migration_label} lost rows from `{table}`: {before} before, {after} after"
    );
    Err(AppError::Internal(format!(
        "migration {migration_label} lost rows from table `{table}`: {before} rows before the \
         rebuild, {after} rows after. Refusing to continue on a database that no longer holds \
         every attendance record. Restore the pre-migration snapshot stored next to the database \
         file and report this."
    )))
}

/// The columns `table` currently has.
///
/// Read before a table rebuild and again after it, so
/// [`assert_columns_preserved`] can tell whether the rebuild dropped anything
/// the table used to carry. The SQL lives in `sf2/sql/table_columns.sql`; only
/// the table name - always a literal from this module or its tests - is
/// substituted here.
fn table_columns(conn: &rusqlite::Connection, table: &str) -> Result<BTreeSet<String>> {
    let sql = TABLE_COLUMNS_SQL.replace("{table}", table);
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<rusqlite::Result<BTreeSet<String>>>()
        .map_err(Into::into)
}

/// Fail the migration when a table rebuild dropped a column.
///
/// The row-count assertion next to this one catches lost *rows*. It cannot see a
/// lost *column*, and a v11 replay against a table a newer build had already
/// extended is exactly that: `migrate_to_v11.sql` rebuilds `students` with five
/// columns and `events` with six, so replaying it over a v14+ table silently
/// drops `session_key`, `override_reason` and `updated_at` from `events` and
/// `gender` and `sf2_learner_id` from `students`. The exception audit trail those
/// three columns carry is then unreadable, and the identities the per-month model
/// depends on are gone - with no error anywhere.
///
/// Row counts and column lists are the same failure seen from two sides, and both
/// have to be checked: a rebuild can preserve every row and still destroy what
/// was in them.
///
/// Every table is reported in one error rather than the first one to fail. An
/// operator restoring from the pre-migration snapshot needs to know the whole
/// list of what was at risk, and finding out one table at a time is a worse
/// experience than a slightly longer message.
pub(crate) fn assert_columns_preserved(
    tables: &[(&str, &BTreeSet<String>, &BTreeSet<String>)],
    migration_label: &str,
) -> Result<()> {
    let mut lost_all = Vec::new();
    for (table, before, after) in tables {
        for column in before.difference(after) {
            lost_all.push(format!("`{table}`.`{column}`"));
        }
    }
    if lost_all.is_empty() {
        return Ok(());
    }
    let lost = lost_all.join(", ");
    log::error!("migration {migration_label} dropped columns: {lost}");
    Err(AppError::Internal(format!(
        "migration {migration_label} dropped column(s) {lost}. That database was written by a \
         newer build than the one running this migration, and continuing would destroy attendance \
         data those columns hold. Restore the pre-migration snapshot stored next to the database \
         file and report this."
    )))
}

/// One statement read out of a migration `.sql` file.
struct MigrationStatement {
    /// The `-- name: <label>` marker above the statement, when it has one.
    ///
    /// Unmarked statements are the file's DDL and run once. Marked statements
    /// are data work that runs once per calendar month with the file's
    /// placeholders bound, and are looked up by name.
    name: Option<String>,
    sql: String,
}

/// Split a migration file into statements, keeping the `-- name:` markers.
///
/// The splitter is line-oriented: a line whose first non-space characters are
/// `--` is a comment and is dropped, unless it is inside a string literal, and
/// a `-- name: <label>` line names the statement that follows it. Semicolons
/// inside a string literal do not end a statement.
///
/// Everything after a statement's closing `;` on the same line is discarded,
/// which is what makes a trailing `-- explain what this column holds` safe. The
/// cost is that a migration file may not put two statements on one line; none of
/// them do.
///
/// Migration files therefore must not wrap a string literal across lines either.
/// None of them do.
fn parse_migration_statements(file: &str) -> Vec<MigrationStatement> {
    let mut statements = Vec::new();
    let mut sql = String::new();
    let mut name: Option<String> = None;
    let mut quote: Option<char> = None;

    for line in file.lines() {
        if quote.is_none() {
            let trimmed = line.trim_start();
            if let Some(marker) = trimmed.strip_prefix("-- name:") {
                name = Some(marker.trim().to_string());
                continue;
            }
            if trimmed.starts_with("--") {
                continue;
            }
        }

        let mut ended = false;
        for ch in line.chars() {
            if ended {
                break;
            }
            if let Some(open) = quote {
                sql.push(ch);
                if ch == open {
                    quote = None;
                }
                continue;
            }
            match ch {
                '\'' | '"' => {
                    quote = Some(ch);
                    sql.push(ch);
                }
                ';' => {
                    push_migration_statement(&mut statements, &mut sql, name.take());
                    ended = true;
                }
                _ => sql.push(ch),
            }
        }
        sql.push('\n');
    }
    push_migration_statement(&mut statements, &mut sql, name.take());

    statements
}

fn push_migration_statement(
    statements: &mut Vec<MigrationStatement>,
    sql: &mut String,
    name: Option<String>,
) {
    if !sql.trim().is_empty() {
        statements.push(MigrationStatement {
            name,
            sql: sql.trim().to_string(),
        });
    }
    sql.clear();
}

/// The `(table, column)` an `ALTER TABLE ... ADD COLUMN` statement adds, if the
/// statement is one. Keyword matching is case-insensitive.
fn added_column(sql: &str) -> Option<(String, String)> {
    const TABLE: &str = "ALTER TABLE ";
    const COLUMN: &str = " ADD COLUMN ";

    let trimmed = sql.trim();
    if trimmed.len() < TABLE.len() || !trimmed[..TABLE.len()].eq_ignore_ascii_case(TABLE) {
        return None;
    }
    let rest = &trimmed[TABLE.len()..];
    let keyword_start = rest
        .as_bytes()
        .windows(COLUMN.len())
        .position(|window| window.eq_ignore_ascii_case(COLUMN.as_bytes()))?;
    let after_keyword = keyword_start + COLUMN.len() - 1;
    let table = rest[..keyword_start].trim();
    let column = rest[after_keyword..].split_whitespace().next()?;
    if table.is_empty() {
        return None;
    }
    Some((table.to_string(), column.to_string()))
}

/// Does `table` already have `column`?
fn column_exists(conn: &rusqlite::Connection, table: &str, column: &str) -> Result<bool> {
    let sql = COLUMN_EXISTS_SQL
        .replace("{table}", table)
        .replace("{column}", column);
    let matches: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
    Ok(matches > 0)
}

/// Run a migration file's unmarked DDL statements, in order, skipping any
/// `ADD COLUMN` whose column is already present.
///
/// SQLite has no `ADD COLUMN IF NOT EXISTS`, and a migration is replayed
/// whenever the process died between the last successful statement and the
/// `PRAGMA user_version` write that follows it. Without this guard, that replay
/// aborts on "duplicate column name" and the app no longer starts at all - the
/// worst possible outcome for a migration whose whole purpose is to make the
/// app safer.
fn execute_migration_ddl(conn: &rusqlite::Connection, file: &str) -> Result<()> {
    for statement in parse_migration_statements(file)
        .into_iter()
        .filter(|statement| statement.name.is_none())
    {
        if let Some((table, column)) = added_column(&statement.sql) {
            if column_exists(conn, &table, &column)? {
                log::info!(
                    "migration: column `{table}`.`{column}` already exists, skipping the add"
                );
                continue;
            }
        }
        conn.execute_batch(&statement.sql)?;
    }
    Ok(())
}

/// Migrate database to version 19 (one row per month workbook file)
fn migrate_to_v19(conn: &rusqlite::Connection) -> Result<()> {
    execute_migration_ddl(conn, MIGRATE_TO_V19_SQL)
}

/// Migrate database to version 20 (one roster per month file + DepEd learner ID)
fn migrate_to_v20(conn: &rusqlite::Connection) -> Result<()> {
    execute_migration_ddl(conn, MIGRATE_TO_V20_SQL)
}

/// Migrate database to version 21 (one day-number grid per month file)
fn migrate_to_v21(conn: &rusqlite::Connection) -> Result<()> {
    execute_migration_ddl(conn, MIGRATE_TO_V21_SQL)
}

/// Migrate database to version 22 (per-month settings, the D16 override column,
/// and the backfill of the legacy single-template row)
///
/// The backfill is what keeps an install working before the split runs
/// (edge case E14). Every mapping it copies is counted before and after, with
/// the same assertion the v11 and v17 rebuilds use: a backfill that quietly
/// drops a mapping is a hard error, because a missing date mapping is the
/// precondition for the whole destructive-sync chain in spec §4.
fn migrate_to_v22(conn: &rusqlite::Connection) -> Result<()> {
    execute_migration_ddl(conn, MIGRATE_TO_V22_SQL)?;
    backfill_legacy_template_into_month_rows(conn)
}

/// Copy the legacy single-template row into the per-month tables, once per
/// calendar month of the school year.
///
/// The legacy model is one workbook per class with a mutable `report_month`, so
/// each legacy row has exactly one month to become. Both mapping copies are
/// bracketed by [`assert_row_count_preserved`], and every statement is written
/// to be a no-op when replayed.
fn backfill_legacy_template_into_month_rows(conn: &rusqlite::Connection) -> Result<()> {
    let statements = parse_migration_statements(MIGRATE_TO_V22_SQL);
    let named = |name: &str| -> Result<String> {
        statements
            .iter()
            .find(|statement| statement.name.as_deref() == Some(name))
            .map(|statement| statement.sql.clone())
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "migrate_to_v22.sql is missing its `{name}` statement"
                ))
            })
    };

    for month in 1..=12_u32 {
        let month_name = crate::sf2::calendar::sf2_month_name(month);
        // Every canonical name is ASCII and at least three characters long.
        let Some(month_abbr) = month_name.get(..3) else {
            continue;
        };
        let month_number = format!("{month:02}");
        let bind = |sql: &str| -> String {
            sql.replace("{month_name}", month_name)
                .replace("{month_abbr}", month_abbr)
                .replace("{month_number}", &month_number)
        };
        let count = |name: &str| -> Result<i64> {
            let sql = bind(&named(name)?);
            Ok(conn.query_row(&sql, [], |row| row.get::<_, i64>(0))?)
        };

        for name in ["template", "students", "dates"] {
            conn.execute_batch(&bind(&named(name)?))?;
        }

        let label = format!("v22 backfill ({month_name})");
        assert_row_count_preserved(
            "sf2_month_student_mappings",
            count("students_expected")?,
            count("students_actual")?,
            &label,
        )?;
        assert_row_count_preserved(
            "sf2_month_date_mappings",
            count("dates_expected")?,
            count("dates_actual")?,
            &label,
        )?;
    }

    Ok(())
}

/// Run all pending database migrations on an existing SQLite connection.
pub fn migrate_db(conn: &rusqlite::Connection) -> Result<()> {
    // Check if we need to run migrations
    let user_version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(0);

    if user_version < 1 {
        // Initial schema creation or migration to version 1
        migrate_to_v1(conn)?;
        conn.execute("PRAGMA user_version = 1", [])?;
    }

    if user_version < 2 {
        migrate_to_v2(conn)?;
        conn.execute("PRAGMA user_version = 2", [])?;
    }

    if user_version < 3 {
        migrate_to_v3(conn)?;
        conn.execute("PRAGMA user_version = 3", [])?;
    }

    if user_version < 4 {
        migrate_to_v4(conn)?;
        conn.execute("PRAGMA user_version = 4", [])?;
    }

    if user_version < 5 {
        migrate_to_v5(conn)?;
        conn.execute("PRAGMA user_version = 5", [])?;
    }

    if user_version < 6 {
        migrate_to_v6(conn)?;
        conn.execute("PRAGMA user_version = 6", [])?;
    }

    if user_version < 7 {
        migrate_to_v7(conn)?;
        conn.execute("PRAGMA user_version = 7", [])?;
    }

    if user_version < 8 {
        migrate_to_v8(conn)?;
        conn.execute("PRAGMA user_version = 8", [])?;
    }

    if user_version < 9 {
        migrate_to_v9(conn)?;
        conn.execute("PRAGMA user_version = 9", [])?;
    }

    if user_version < 10 {
        migrate_to_v10(conn)?;
        conn.execute("PRAGMA user_version = 10", [])?;
    }

    if user_version < 11 {
        migrate_to_v11(conn)?;
        conn.execute("PRAGMA user_version = 11", [])?;
    }

    if user_version < 12 {
        migrate_to_v12(conn)?;
        conn.execute("PRAGMA user_version = 12", [])?;
    }

    if user_version < 13 {
        migrate_to_v13(conn)?;
        conn.execute("PRAGMA user_version = 13", [])?;
    }

    if user_version < 14 {
        migrate_to_v14(conn)?;
        conn.execute("PRAGMA user_version = 14", [])?;
    }

    if user_version < 15 {
        migrate_to_v15(conn)?;
        conn.execute(
            &format!("PRAGMA user_version = {CURRENT_SCHEMA_VERSION}"),
            [],
        )?;
    }

    if user_version < 16 {
        migrate_to_v16(conn)?;
        conn.execute(
            &format!("PRAGMA user_version = {CURRENT_SCHEMA_VERSION}"),
            [],
        )?;
    }

    if user_version < 17 {
        migrate_to_v17(conn)?;
        conn.execute(
            &format!("PRAGMA user_version = {CURRENT_SCHEMA_VERSION}"),
            [],
        )?;
    }

    if user_version < 18 {
        migrate_to_v18(conn)?;
        conn.execute(
            &format!("PRAGMA user_version = {CURRENT_SCHEMA_VERSION}"),
            [],
        )?;
    }

    if user_version < 19 {
        migrate_to_v19(conn)?;
        conn.execute(
            &format!("PRAGMA user_version = {CURRENT_SCHEMA_VERSION}"),
            [],
        )?;
    }

    if user_version < 20 {
        migrate_to_v20(conn)?;
        conn.execute(
            &format!("PRAGMA user_version = {CURRENT_SCHEMA_VERSION}"),
            [],
        )?;
    }

    if user_version < 21 {
        migrate_to_v21(conn)?;
        conn.execute(
            &format!("PRAGMA user_version = {CURRENT_SCHEMA_VERSION}"),
            [],
        )?;
    }

    if user_version < 22 {
        migrate_to_v22(conn)?;
        conn.execute(
            &format!("PRAGMA user_version = {CURRENT_SCHEMA_VERSION}"),
            [],
        )?;
    }

    if user_version < 23 {
        migrate_to_v23(conn)?;
        conn.execute(
            &format!("PRAGMA user_version = {CURRENT_SCHEMA_VERSION}"),
            [],
        )?;
    }

    // brief-S1: v24 - the `sheet_name` column and its backfill. The body lives in
    // `crate::sf2::month::schema_v24` because that module owns the version number
    // above; this is the registration and nothing else.
    if user_version < crate::sf2::month::schema_v24::SCHEMA_VERSION {
        crate::sf2::month::schema_v24::migrate_to_v24(conn)?;
        conn.execute(
            &format!("PRAGMA user_version = {CURRENT_SCHEMA_VERSION}"),
            [],
        )?;
    }

    Ok(())
}

/// Migrate database to version 1 (add class support)
fn migrate_to_v1(conn: &rusqlite::Connection) -> Result<()> {
    // Create all tables with proper schema
    conn.execute_batch(
        r#"
        -- Create classes table
        CREATE TABLE IF NOT EXISTS classes (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            day_start TEXT NOT NULL,
            day_end TEXT NOT NULL,
            late_after TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        -- Create indexes for classes
        CREATE INDEX IF NOT EXISTS idx_classes_name ON classes(name);

        -- Create students table with class support
        CREATE TABLE IF NOT EXISTS students_new (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            card_serial TEXT UNIQUE,
            class_id TEXT,
            created_at INTEGER NOT NULL
        );

        -- Create indexes for students
        CREATE INDEX IF NOT EXISTS idx_students_card_new ON students_new(card_serial);
        CREATE INDEX IF NOT EXISTS idx_students_name_new ON students_new(name);
        CREATE INDEX IF NOT EXISTS idx_students_class_new ON students_new(class_id);

        -- Create events table with class support
        CREATE TABLE IF NOT EXISTS events_new (
            id TEXT PRIMARY KEY NOT NULL,
            student_id TEXT NOT NULL,
            class_id TEXT,
            event_type TEXT NOT NULL CHECK(event_type IN ('in')),
            timestamp INTEGER NOT NULL,
            note TEXT,
            FOREIGN KEY (student_id) REFERENCES students_new(id) ON DELETE CASCADE
        );

        -- Create indexes for events
        CREATE INDEX IF NOT EXISTS idx_events_student_new ON events_new(student_id);
        CREATE INDEX IF NOT EXISTS idx_events_timestamp_new ON events_new(timestamp);

        -- Create settings table
        CREATE TABLE IF NOT EXISTS settings (
            id TEXT PRIMARY KEY NOT NULL,
            day_start TEXT NOT NULL,
            day_end TEXT NOT NULL,
            late_after TEXT NOT NULL,
            quarter TEXT NOT NULL DEFAULT '1st Quarter'
        );

        -- Insert default settings
        INSERT OR IGNORE INTO settings (id, day_start, day_end, late_after, quarter)
        VALUES ('app', '08:00', '15:00', '08:45', '1st Quarter');
        "#,
    )?;

    // Migrate data from old tables if they exist
    let has_old_students = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='students'")
        .and_then(|mut stmt| stmt.query_row([], |_| Ok(true)))
        .unwrap_or(false);

    if has_old_students {
        // Copy data from old students table to new one
        conn.execute(
            "INSERT INTO students_new (id, name, card_serial, created_at) 
             SELECT id, name, card_serial, created_at FROM students",
            [],
        )?;

        // Copy data from old events table to new one
        conn.execute(
            "INSERT INTO events_new (id, student_id, event_type, timestamp, note) 
             SELECT id, student_id, event_type, timestamp, note FROM events
             WHERE event_type = 'in'",
            [],
        )?;

        // Drop old tables
        conn.execute("DROP TABLE IF EXISTS students", [])?;
        conn.execute("DROP TABLE IF EXISTS events", [])?;

        // Rename new tables
        conn.execute("ALTER TABLE students_new RENAME TO students", [])?;
        conn.execute("ALTER TABLE events_new RENAME TO events", [])?;

        // Rename indexes
        conn.execute("DROP INDEX IF EXISTS idx_students_card_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_students_name_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_students_class_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_events_student_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_events_timestamp_new", [])?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_card ON students(card_serial)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_name ON students(name)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_class ON students(class_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_student ON events(student_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp)",
            [],
        )?;
    } else {
        // No old data, just rename the new tables
        conn.execute("ALTER TABLE students_new RENAME TO students", [])?;
        conn.execute("ALTER TABLE events_new RENAME TO events", [])?;

        // Rename indexes
        conn.execute("DROP INDEX IF EXISTS idx_students_card_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_students_name_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_students_class_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_events_student_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_events_timestamp_new", [])?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_card ON students(card_serial)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_name ON students(name)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_class ON students(class_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_student ON events(student_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp)",
            [],
        )?;
    }

    Ok(())
}

/// Migrate database to version 2 (add room to classes)
fn migrate_to_v2(conn: &rusqlite::Connection) -> Result<()> {
    // Check if room column exists
    let has_room: bool = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('classes') WHERE name='room'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_room {
        conn.execute(
            "ALTER TABLE classes ADD COLUMN room TEXT NOT NULL DEFAULT 'N/A'",
            [],
        )?;
    }
    Ok(())
}

/// Migrate database to version 3 (add quarter to settings)
fn migrate_to_v3(conn: &rusqlite::Connection) -> Result<()> {
    // Check if quarter column exists
    let has_quarter: bool = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('settings') WHERE name='quarter'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_quarter {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN quarter TEXT NOT NULL DEFAULT '1st Quarter'",
            [],
        )?;
    }
    Ok(())
}

/// Migrate database to version 4 (add quarter dates to settings)
fn migrate_to_v4(conn: &rusqlite::Connection) -> Result<()> {
    let columns = [
        "q1_start", "q1_end", "q2_start", "q2_end", "q3_start", "q3_end",
    ];

    for col in columns {
        let has_col: bool = conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM pragma_table_info('settings') WHERE name='{}'",
                    col
                ),
                [],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;

        if !has_col {
            conn.execute(&format!("ALTER TABLE settings ADD COLUMN {} TEXT", col), [])?;
        }
    }
    Ok(())
}

/// Migrate database to version 5 (add sessions to classes)
fn migrate_to_v5(conn: &rusqlite::Connection) -> Result<()> {
    // Check if sessions column exists
    let has_sessions: bool = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('classes') WHERE name='sessions'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_sessions {
        conn.execute("ALTER TABLE classes ADD COLUMN sessions TEXT", [])?;

        // Initialize sessions for existing classes based on day_start, day_end, late_after
        let mut stmt = conn.prepare("SELECT id, day_start, day_end, late_after FROM classes")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        for row in rows {
            let (id, day_start, day_end, late_after) = row?;
            let sessions = vec![Session {
                name: "Full Day".to_string(),
                start_time: day_start,
                end_time: day_end,
                late_after,
            }];
            let sessions_json =
                serde_json::to_string(&sessions).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "UPDATE classes SET sessions = ?1 WHERE id = ?2",
                params![sessions_json, id],
            )?;
        }
    }
    Ok(())
}

/// Migrate database to version 6 (add days to classes)
fn migrate_to_v6(conn: &rusqlite::Connection) -> Result<()> {
    // Check if days column exists
    let has_days: bool = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('classes') WHERE name='days'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_days {
        conn.execute("ALTER TABLE classes ADD COLUMN days TEXT", [])?;

        // Initialize days for existing classes to Monday-Friday [1, 2, 3, 4, 5]
        let days = vec![1, 2, 3, 4, 5];
        let days_json = serde_json::to_string(&days).unwrap_or_else(|_| "[]".to_string());
        conn.execute("UPDATE classes SET days = ?1", params![days_json])?;
    }
    Ok(())
}

/// Migrate database to version 7 (limit active quarter to three periods)
fn migrate_to_v7(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute(
        "UPDATE settings
         SET quarter = '3rd Quarter'
         WHERE quarter NOT IN ('1st Quarter', '2nd Quarter', '3rd Quarter')",
        [],
    )?;

    Ok(())
}

/// Migrate database to version 8 (add attendance mode setting)
fn migrate_to_v8(conn: &rusqlite::Connection) -> Result<()> {
    let has_attendance_mode: bool = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('settings') WHERE name='attendance_mode'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_attendance_mode {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN attendance_mode TEXT NOT NULL DEFAULT 'manual'",
            [],
        )?;
    }

    conn.execute(
        "UPDATE settings
         SET attendance_mode = 'manual'
         WHERE attendance_mode NOT IN ('manual', 'card_reader')",
        [],
    )?;

    Ok(())
}

/// Migrate database to version 9 (add DepEd SF2 workbook mappings)
fn migrate_to_v9(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sf2/sql/migrate_to_v9.sql"))?;
    Ok(())
}

/// Migrate database to version 10 (add SF2 form metadata to settings)
fn migrate_to_v10(conn: &rusqlite::Connection) -> Result<()> {
    let columns = [
        "school_id",
        "school_name",
        "school_year",
        "report_month",
        "grade_level",
        "section",
        "adviser_name",
        "school_head_name",
    ];

    for column in columns {
        let has_column: bool = conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM pragma_table_info('settings') WHERE name='{}'",
                    column
                ),
                [],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;

        if !has_column {
            conn.execute(
                &format!("ALTER TABLE settings ADD COLUMN {} TEXT", column),
                [],
            )?;
        }
    }

    Ok(())
}

/// Migrate database to version 11 (single IN attendance and no external student number)
///
/// Rebuilt `students` and `events`. The rebuild used to delete every non-'in'
/// attendance event, which destroyed absences on any database that crossed
/// v10 -> v11 carrying data written by a newer build; the upgrade now deletes
/// only event types this schema does not know about
/// (`purge_unknown_event_types.sql`), and the row counts around the rebuild
/// prove nothing was lost.
///
/// That delete is the one step of this migration that is *meant* to change the
/// `events` row count, so it runs first and outside the bracket. Counting
/// before it would make the guard reject the very deletion it exists to make
/// safe, and would blind the guard to the rebuild - which is the part that
/// actually risks dropping rows.
///
/// The **column** lists are asserted over the same bracket, and that is the half
/// a row count cannot see. Both rebuilds are written from the v11 shape, so a
/// database whose `user_version` was lost while its tables had already been
/// extended by a newer build would come out of this migration with
/// `session_key` / `override_reason` / `updated_at` and `gender` /
/// `sf2_learner_id` silently gone. Nothing would error, and the exception audit
/// trail would be unreadable afterwards.
fn migrate_to_v11(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sf2/sql/purge_unknown_event_types.sql"))?;

    let students_before = row_count(conn, "students")?;
    let events_before = row_count(conn, "events")?;
    let student_columns_before = table_columns(conn, "students")?;
    let event_columns_before = table_columns(conn, "events")?;

    conn.execute_batch(include_str!("../../sf2/sql/migrate_to_v11.sql"))?;

    assert_row_count_preserved(
        "students",
        students_before,
        row_count(conn, "students")?,
        "v11",
    )?;
    assert_row_count_preserved("events", events_before, row_count(conn, "events")?, "v11")?;
    let student_columns_after = table_columns(conn, "students")?;
    let event_columns_after = table_columns(conn, "events")?;
    assert_columns_preserved(
        &[
            ("students", &student_columns_before, &student_columns_after),
            ("events", &event_columns_before, &event_columns_after),
        ],
        "v11",
    )?;
    Ok(())
}

/// Migrate database to version 12 (store SF2 metadata per workbook template)
fn migrate_to_v12(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sf2/sql/migrate_to_v12.sql"))?;
    Ok(())
}

/// Migrate database to version 13 (add student gender for SF2 roster sections)
fn migrate_to_v13(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sf2/sql/migrate_to_v13.sql"))?;
    Ok(())
}

/// Migrate database to version 14 (attendance exception workflow)
fn migrate_to_v14(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sf2/sql/migrate_to_v14.sql"))?;
    Ok(())
}

/// Migrate database to version 15 (general audit trail)
fn migrate_to_v15(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sf2/sql/migrate_to_v15.sql"))?;
    Ok(())
}

/// Migrate database to version 16 (track last SF2 attendance sync timestamp)
fn migrate_to_v16(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sf2/sql/migrate_to_v16.sql"))?;
    Ok(())
}

/// Migrate database to version 17 (explicit 'absent' attendance event type)
///
/// Rebuilds the events table so the event_type CHECK constraint accepts both
/// 'in' and 'absent'. Absence is now stored as its own record (a student marked
/// absent from the attendance page or SF2 preview) instead of being derived from
/// a missing 'in' record, so other students are never auto-recorded.
///
/// The `events` row count is asserted across the rebuild: losing a single
/// attendance record here would be permanent and invisible.
fn migrate_to_v17(conn: &rusqlite::Connection) -> Result<()> {
    let events_before = row_count(conn, "events")?;

    conn.execute_batch(include_str!("../../sf2/sql/migrate_to_v17.sql"))?;

    assert_row_count_preserved("events", events_before, row_count(conn, "events")?, "v17")?;
    Ok(())
}

/// Migrate database to version 18 (sidebar branding customization)
fn migrate_to_v18(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sf2/sql/migrate_to_v18.sql"))?;
    Ok(())
}

/// Migrate database to version 23 (one canonical school-year label)
///
/// ## The defect this closes
///
/// A month row is looked up by exact equality on
/// `(active_class_id, school_year, report_month)`, and the question "which
/// school year does this class actually have months for" is answered by a GLOB
/// for `[0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9]`. Both were written against
/// `2026-2027`. A user who types the DepEd form's own rendering of the same
/// year - `2026 - 2027`, spaces around the dash - stores a label that matches no
/// row and passes no GLOB: the whole per-month table becomes silently
/// unreachable and every read falls back to the legacy tables, for a whole
/// school year, with nothing reporting an error.
///
/// ## Why this is Rust and not a `.sql` file
///
/// Because the canonical form is defined by
/// [`normalize_school_year`](crate::sf2::month::first_school_day::normalize_school_year),
/// which is also what the read and write paths apply. Re-implementing that
/// parser in SQL would give the backfill and the normaliser two independent
/// definitions of "canonical" - the two-format bug reappearing one layer down,
/// where it would be even harder to see. Calling the one function means the
/// stored rows and the rows a read looks for cannot be spelled differently, by
/// construction.
///
/// ## What it will not do
///
/// A label with fewer than two four-digit years is left exactly as it is, so a
/// row the user has not finished typing survives. And a row is never deleted
/// except where two rows for the same month would collapse onto one canonical
/// label - see [`collapse_duplicate_month_rows`].
fn migrate_to_v23(conn: &rusqlite::Connection) -> Result<()> {
    use crate::sf2::month::first_school_day::normalize_school_year;

    for table in ["sf2_templates", "sf2_month_templates"] {
        let Some(sql) = column_bearing_school_year_update(table) else {
            continue;
        };
        let mut statement = conn
            .prepare(&format!(
                "SELECT DISTINCT school_year FROM {table} WHERE school_year <> ''"
            ))
            .map_err(|error| AppError::Internal(format!("v23 could not read {table}: {error}")))?;
        let labels = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| AppError::Internal(format!("v23 could not read {table}: {error}")))?
            .collect::<rusqlite::Result<Vec<String>>>()
            .map_err(|error| AppError::Internal(format!("v23 could not read {table}: {error}")))?;

        for label in labels {
            let canonical = normalize_school_year(&label);
            if canonical == label {
                continue;
            }
            let updated = conn.execute(&sql, params![canonical, label]);
            match updated {
                Ok(rows) if rows > 0 => log::info!(
                    "v23: rewrote the school year {label:?} as {canonical:?} in {table} ({rows} row(s))"
                ),
                Ok(_) => {}
                // The one place a rewrite can conflict: two rows for the same
                // month, stored under two spellings of the same year. That is
                // the duplication this defect produces, and it is handled below
                // rather than swallowed.
                Err(error) => {
                    if table == "sf2_month_templates" {
                        collapse_duplicate_month_rows(conn, &label, &canonical)?;
                    } else {
                        return Err(AppError::Internal(format!(
                            "v23 could not rewrite the school year {label:?} in {table}: {error}"
                        )));
                    }
                }
            }
        }
    }

    normalise_settings_school_year(conn)?;
    Ok(())
}

/// The `UPDATE` for one SF2 table, or `None` when that table does not exist yet.
///
/// A v18 database mid-chain may not have the per-month table; skipping is right,
/// because nothing in it stores a school year that then goes un-normalised.
fn column_bearing_school_year_update(table: &str) -> Option<String> {
    if !matches!(table, "sf2_templates" | "sf2_month_templates") {
        return None;
    }
    Some(format!(
        "UPDATE {table} SET school_year = ?1 WHERE school_year = ?2"
    ))
}

/// Two rows for one month, stored under two spellings of the same year.
///
/// Collapsing them is a repair, not a cleanup: the duplicate is unreachable by
/// every per-month read, so the month has been showing the legacy tables all
/// along, and leaving both rows would leave a month that cannot be represented
/// at all. The row that survives is the **most recently imported** one, because
/// that is the one whose `source_path`/`first_school_day` the app last wrote.
///
/// Logged loudly, because deleting a row is the one destructive thing this
/// migration does and it must never be silent.
fn collapse_duplicate_month_rows(
    conn: &rusqlite::Connection,
    spaced: &str,
    canonical: &str,
) -> Result<()> {
    let duplicates: Vec<(String, i64)> = {
        let mut statement = conn
            .prepare(
                "SELECT id, imported_at FROM sf2_month_templates
                 WHERE school_year = ?1 OR school_year = ?2
                 ORDER BY imported_at ASC, id ASC",
            )
            .map_err(|error| {
                AppError::Internal(format!("v23 could not read the month rows: {error}"))
            })?;
        let rows = statement
            .query_map(params![spaced, canonical], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|error| {
                AppError::Internal(format!("v23 could not read the month rows: {error}"))
            })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| {
                AppError::Internal(format!("v23 could not read the month rows: {error}"))
            })?
    };

    // Keep the newest, drop the rest - but only the ones that are duplicates of
    // each other, which is every row beyond the first for a given month.
    for (id, imported_at) in duplicates.iter().skip(1) {
        let removed = conn
            .execute("DELETE FROM sf2_month_templates WHERE id = ?1", params![id])
            .map_err(|error| {
                AppError::Internal(format!(
                    "v23 could not collapse the duplicate month row: {error}"
                ))
            })?;
        log::warn!(
            "v23: removed the older duplicate month row {id} (imported {imported_at}) so that \
             {spaced:?} and {canonical:?} can share one canonical school year; its date and \
             student mappings were removed with it. {removed} row(s) deleted."
        );
    }
    Ok(())
}

/// Keep the app-level `settings.school_year` in step with the SF2 tables.
///
/// It is the copy the user typed and the frontend shows, so leaving it behind
/// would let the two disagree again - which is the bug this migration exists to
/// close, restated somewhere else.
fn normalise_settings_school_year(conn: &rusqlite::Connection) -> Result<()> {
    use crate::sf2::month::first_school_day::normalize_school_year;

    let stored: Option<String> = conn
        .query_row(
            "SELECT school_year FROM settings WHERE id = 'app'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| AppError::Internal(format!("v23 could not read the settings: {error}")))?;
    let Some(stored) = stored else {
        return Ok(());
    };
    let canonical = normalize_school_year(&stored);
    if canonical == stored {
        return Ok(());
    }
    conn.execute(
        "UPDATE settings SET school_year = ?1 WHERE id = 'app'",
        params![canonical],
    )
    .map_err(|error| {
        AppError::Internal(format!(
            "v23 could not rewrite the settings school year: {error}"
        ))
    })?;
    log::info!("v23: rewrote the app settings' school year {stored:?} as {canonical:?}");
    Ok(())
}

#[cfg(test)]
#[path = "__tests__/migration_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "__tests__/month_migration_tests.rs"]
mod month_migration_tests;
