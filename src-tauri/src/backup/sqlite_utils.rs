use anyhow::{bail, Result};
use rusqlite::Connection;

pub(crate) fn open_table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table_name],
        |row| row.get(0),
    )?;
    Ok(exists > 0)
}

pub(crate) fn require_core_tables(conn: &Connection) -> Result<()> {
    for table in ["classes", "students", "events", "settings"] {
        if !open_table_exists(conn, table)? {
            bail!("Backup is not an EES-AMS database: missing {table} table");
        }
    }
    Ok(())
}

pub(crate) fn count_table_rows(conn: &Connection, table_name: &str) -> Result<i64> {
    if !open_table_exists(conn, table_name)? {
        return Ok(0);
    }

    conn.query_row(&format!("SELECT COUNT(*) FROM {table_name}"), [], |row| {
        row.get(0)
    })
    .map_err(Into::into)
}

pub(crate) fn read_schema_version(conn: &Connection) -> Result<i32> {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(Into::into)
}

pub(crate) fn run_integrity_check(conn: &Connection) -> Result<()> {
    let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if result == "ok" {
        Ok(())
    } else {
        bail!("Backup failed SQLite integrity check: {result}");
    }
}
