//! Schema v24 - the sheet dimension comes back to `sf2_month_date_mappings`
//! (spec section 0, A4).
//!
//! ## Why this lives outside `migrations.rs`
//!
//! `migrations.rs` is owned by a parallel workstream, so this module owns the
//! v24 *body* and leaves `migrations.rs` a three-line registration. The version
//! number itself is [`SCHEMA_VERSION`] rather than a second literal, so the two
//! cannot drift:
//!
//! ```text
//! // brief-S1: migrations.rs
//! pub const CURRENT_SCHEMA_VERSION: i32 = crate::sf2::month::schema_v24::SCHEMA_VERSION;
//! if user_version < crate::sf2::month::schema_v24::SCHEMA_VERSION {
//!     crate::sf2::month::schema_v24::migrate_to_v24(conn)?;
//!     conn.execute("PRAGMA user_version = 24", [])?;
//! }
//! ```
//!
//! ## Why the column comes back at all
//!
//! v21 dropped `sheet_name` because the workbook was one file per month, and a
//! file with one worksheet makes the sheet derivable from the month row. The
//! workbook is now one file with twelve month worksheets (spec section 0). The
//! derivation is still correct - it is the same string - but a *write* path has
//! to read the sheet back out of the database and compare it against the
//! workbook, and a value that only exists as a derivation cannot be compared
//! against a stored one. Storing it makes the two checkable, which is the whole
//! point of a sheet dimension in a twelve-sheet file.
//!
//! ## Why this migration cannot lose anything
//!
//! The user this was written for is on v22 with 37 absences and 22 date
//! mappings in the database. So:
//!
//! * It is **additive only**. One `ADD COLUMN`, nullable, no default, so adding
//!   it rewrites no other value on any row. Three `UPDATE`s that touch nothing
//!   but `sheet_name`. One `CREATE INDEX`.
//! * **No row is ever blanked.** Every `UPDATE` is guarded on the row still
//!   being unset, and a row that none of the three passes can fill is left
//!   exactly as it was rather than set to `''`. [`unresolved_sheet_rows`] exists
//!   so a test - and, if it ever fires in the field, the log - can prove that
//!   number is zero.
//! * **The row count of every table involved is bracketed** by
//!   [`assert_row_count_preserved`], the same guard the v11 and v17 rebuilds
//!   use. A migration that quietly dropped a mapping would now be a hard error
//!   at install time, next to the pre-migration snapshot, instead of the empty
//!   September grid the user is actually reporting.
//! * **The pre-split tables are untouched.** `sf2_templates`,
//!   `sf2_student_mappings` and `sf2_date_mappings` are read, never written and
//!   never dropped. On the affected install they are still where some of the
//!   data lives.

use crate::domain::error::{AppError, Result};

/// The schema version this module brings an install up to.
pub const SCHEMA_VERSION: i32 = 24;

/// The v24 DDL and backfill.
const MIGRATE_TO_V24_SQL: &str = include_str!("../sql/migrate_to_v24.sql");

/// Row-count probe, shared with the v11 and v17 brackets. `{table}` is always
/// substituted from a literal in this module.
const ROW_COUNT_SQL: &str = include_str!("../sql/row_count.sql");

/// Column-existence probe for the idempotent `ADD COLUMN` skip.
const COLUMN_EXISTS_SQL: &str = include_str!("../sql/column_exists.sql");

/// Column-list probe for the precondition check.
const TABLE_COLUMNS_SQL: &str = include_str!("../sql/table_columns.sql");

/// The table v24 touches, named once so the bracket and the log cannot disagree.
const MONTH_DATE_MAPPINGS: &str = "sf2_month_date_mappings";

/// The tables whose rows v24 must not change the number of.
///
/// `sf2_month_date_mappings` is the one this migration writes, and the other two
/// are the ones it reads from: if either of those lost a row, the backfill
/// silently covered less ground than it looks like it did.
const BRACKETED_TABLES: [&str; 3] = [
    MONTH_DATE_MAPPINGS,
    "sf2_month_templates",
    "sf2_month_student_mappings",
];

/// Rows in `sf2_month_date_mappings` that v24 could not give a sheet name.
///
/// The migration leaves these alone rather than blanking them, so this is a
/// measurement and not a repair. It is expected to be `0`: the third backfill
/// pass derives a name from the row's own `date`, which is a full `YYYY-MM-DD`
/// and therefore always yields a month and a year. A non-zero count means some
/// row holds a `date` that is not an ISO date, and
/// [`Sf2MonthDateMapping::resolved_sheet_name`](crate::sf2::month::Sf2MonthDateMapping::resolved_sheet_name)
/// is what a reader falls back to for it.
pub fn unresolved_sheet_rows(conn: &rusqlite::Connection) -> Result<usize> {
    const UNRESOLVED_SQL: &str = "SELECT COUNT(*) FROM sf2_month_date_mappings \
         WHERE sheet_name IS NULL OR TRIM(sheet_name) = ''";
    let count: i64 = conn.query_row(UNRESOLVED_SQL, [], |row| row.get(0))?;
    Ok(count as usize)
}

/// Migrate a v23 database to v24.
///
/// Idempotent, and safe to replay: the `ADD COLUMN` is applied only when the
/// column is missing (SQLite has no `ADD COLUMN IF NOT EXISTS`, and a launch
/// that died before `PRAGMA user_version` was written will replay this), and
/// every backfill statement is guarded on the row still being unset.
pub fn migrate_to_v24(conn: &rusqlite::Connection) -> Result<()> {
    require_month_tables(conn)?;

    let before = BRACKETED_TABLES
        .iter()
        .map(|table| row_count(conn, table))
        .collect::<Result<Vec<_>>>()?;

    for statement in split_statements(MIGRATE_TO_V24_SQL) {
        if is_add_column(&statement) && added_column_already_exists(conn, &statement)? {
            log::info!("v24: {statement} - the column is already there, skipping the add");
            continue;
        }
        conn.execute_batch(&statement)?;
    }

    for (table, expected) in BRACKETED_TABLES.iter().zip(before) {
        assert_count_preserved(table, expected, row_count(conn, table)?)?;
    }

    let unresolved = unresolved_sheet_rows(conn)?;
    if unresolved > 0 {
        // Not an error: the migration is additive and refuses to invent a value.
        // But a grid cell whose sheet is unknown is a cell nothing may write to,
        // so it is loud rather than silent.
        log::warn!(
            "v24: {unresolved} of the day mappings have no sheet name, because their `date` is \
             not a YYYY-MM-DD value. They keep every other field; those months are not writable \
             until they are re-derived."
        );
    }
    Ok(())
}

fn row_count(conn: &rusqlite::Connection, table: &str) -> Result<i64> {
    let sql = ROW_COUNT_SQL.replace("{table}", table);
    Ok(conn.query_row(&sql, [], |row| row.get::<_, i64>(0))?)
}

/// Fail the migration when it changed the number of rows in a table.
///
/// The same guard the v11 and v17 rebuilds apply, restated here so this module
/// does not have to reach into `infrastructure::database`'s private migration
/// module. A migration that silently drops a mapping is the exact failure the
/// whole sheet dimension is meant to stop, and making it an error means the
/// damage surfaces at install time - next to the pre-migration snapshot - rather
/// than as a month with no day columns weeks later.
fn assert_count_preserved(table: &str, before: i64, after: i64) -> Result<()> {
    if before == after {
        return Ok(());
    }
    log::error!("schema v{SCHEMA_VERSION} changed the row count of `{table}`: {before} -> {after}");
    Err(AppError::Internal(format!(
        "migration v{SCHEMA_VERSION} changed the row count of table `{table}`: {before} rows \
         before, {after} rows after. Refusing to continue on a database that no longer holds every \
         attendance record. Restore the pre-migration snapshot stored next to the database file \
         and report this."
    )))
}

/// Is this statement an `ALTER TABLE ... ADD COLUMN ...`?
fn is_add_column(statement: &str) -> bool {
    let trimmed = statement.trim_start();
    trimmed.len() >= "ALTER TABLE ".len()
        && trimmed[.."ALTER TABLE ".len()].eq_ignore_ascii_case("ALTER TABLE ")
        && trimmed.to_ascii_uppercase().contains(" ADD COLUMN ")
}

/// The `(table, column)` an `ADD COLUMN` statement adds, and whether they are
/// already there.
fn added_column_already_exists(conn: &rusqlite::Connection, statement: &str) -> Result<bool> {
    let Some((table, column)) = added_column_parts(statement) else {
        return Ok(false);
    };
    let sql = COLUMN_EXISTS_SQL
        .replace("{table}", &table)
        .replace("{column}", &column);
    let matches: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
    Ok(matches > 0)
}

fn added_column_parts(statement: &str) -> Option<(String, String)> {
    const TABLE: &str = "ALTER TABLE ";
    const COLUMN: &str = " ADD COLUMN ";

    let trimmed = statement.trim();
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
    if table.is_empty() || column.is_empty() {
        return None;
    }
    Some((table.to_string(), column.to_string()))
}

/// Split a migration file into statements.
///
/// Deliberately small: `migrate_to_v24.sql` is this module's own file, and the
/// only properties it relies on are the ones the file actually has - one
/// statement per `;`, and no semicolon inside a string literal. The quote
/// tracking is here so a comment mentioning a semicolon, or a future literal
/// holding one, cannot split a statement in half.
fn split_statements(file: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut sql = String::new();
    let mut quote: Option<char> = None;

    for line in file.lines() {
        let trimmed = line.trim_start();
        if quote.is_none() && trimmed.starts_with("--") {
            continue;
        }
        for ch in line.chars() {
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
                    push(&mut statements, &mut sql);
                }
                _ => sql.push(ch),
            }
        }
        sql.push('\n');
    }
    push(&mut statements, &mut sql);
    statements
}

fn push(statements: &mut Vec<String>, sql: &mut String) {
    if !sql.trim().is_empty() {
        statements.push(sql.trim().to_string());
    }
    sql.clear();
}

/// Refuse to run v24 against a database that does not have the tables it
/// touches.
///
/// A v22 database always has them; this exists so a much older install, whose
/// migration chain failed for an unrelated reason, gets a named error next to
/// its pre-migration snapshot instead of a bare SQLite one.
fn require_month_tables(conn: &rusqlite::Connection) -> Result<()> {
    for table in BRACKETED_TABLES {
        let probe = TABLE_COLUMNS_SQL.replace("{table}", table);
        let columns = conn
            .prepare(&probe)?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if columns.is_empty() {
            return Err(AppError::Internal(format!(
                "schema v{SCHEMA_VERSION} needs the table `{table}`, which this database does not \
                 have. Restore the pre-migration snapshot stored next to the database file."
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "__tests__/schema_v24_tests.rs"]
mod tests;
