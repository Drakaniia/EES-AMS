use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

// ── Constants ─────────────────────────────────────────────────────────

/// Sidecar file holding the baseline fingerprint, beside `attendance.db`.
///
/// The `settings` table is a single fixed-column row keyed by `id = 'app'`
/// (see `infrastructure/database/settings.rs`), not a key/value store, so a
/// fingerprint cannot live there without a schema change — and this phase
/// deliberately ships no schema change. A sidecar JSON file survives an update
/// install, which is all the §9.4 gate needs.
const FINGERPRINT_FILE_NAME: &str = "db-fingerprint.json";

const COUNT_EVENTS_SQL: &str = include_str!("sql/count_events.sql");
const COUNT_ABSENT_EVENTS_SQL: &str = include_str!("sql/count_absent_events.sql");
const COUNT_MONTH_DATE_MAPPINGS_SQL: &str = include_str!("sql/count_month_date_mappings.sql");

// ── Types ─────────────────────────────────────────────────────────────

/// The three counts the §9.4 update-time gate compares between versions.
///
/// A drop in [`absent`](Self::absent) between two versions means attendance
/// records were destroyed by something the user did not ask for — the exact
/// failure this whole spec exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbFingerprint {
    pub events: i64,
    pub absent: i64,
    /// 0 when the database predates the `sf2_month_date_mappings` migration
    /// (v21), which is normal for an old backup.
    #[serde(default)]
    pub month_date_mappings: i64,
}

impl DbFingerprint {
    /// The warning to show when absences went backwards between two versions.
    ///
    /// Returns `None` unless the absent count actually fell — a rise is normal
    /// use, and a fall is only meaningful alongside the pre-install backup the
    /// message points the user at.
    pub fn decrease_notice(previous: &DbFingerprint, current: &DbFingerprint) -> Option<String> {
        if current.absent >= previous.absent {
            return None;
        }
        Some(format!(
            "Attendance records decreased between versions ({} → {}). \
             {} attendance event(s) are missing. Restore from the pre-install backup \
             in Settings → Data Management to recover them.",
            previous.absent,
            current.absent,
            previous.absent - current.absent,
        ))
    }
}

// ── Read ──────────────────────────────────────────────────────────────

/// Read the fingerprint from an open database connection.
///
/// A missing `sf2_month_date_mappings` table counts as 0 rather than failing —
/// the table only exists from v21 onwards, and this must work on every version
/// between v18 and v22.
pub fn read_db_fingerprint(conn: &Connection) -> Result<DbFingerprint> {
    let events = scalar(conn, COUNT_EVENTS_SQL, "events")?;
    let absent = scalar(conn, COUNT_ABSENT_EVENTS_SQL, "absent events")?;
    let month_date_mappings =
        match conn.query_row(COUNT_MONTH_DATE_MAPPINGS_SQL, [], |row| row.get(0)) {
            Ok(count) => count,
            Err(error) => {
                log::debug!("sf2_month_date_mappings is unavailable ({error}); counting as 0");
                0_i64
            }
        };
    Ok(DbFingerprint {
        events,
        absent,
        month_date_mappings,
    })
}

fn scalar(conn: &Connection, sql: &str, label: &str) -> Result<i64> {
    conn.query_row(sql, [], |row| row.get(0))
        .with_context(|| format!("failed to count {label}"))
}

// ── Persist ───────────────────────────────────────────────────────────

pub fn fingerprint_path(app_dir: &Path) -> PathBuf {
    app_dir.join(FINGERPRINT_FILE_NAME)
}

/// Load the stored baseline, or `Ok(None)` when none has been recorded yet.
pub fn load_db_fingerprint(app_dir: &Path) -> Result<Option<DbFingerprint>> {
    let path = fingerprint_path(app_dir);
    if !path.is_file() {
        return Ok(None);
    }
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let fingerprint = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(fingerprint))
}

/// Record the baseline for the running version.
pub fn record_db_fingerprint(app_dir: &Path, fingerprint: &DbFingerprint) -> Result<()> {
    fs::create_dir_all(app_dir)
        .with_context(|| format!("failed to create app data directory {}", app_dir.display()))?;
    let path = fingerprint_path(app_dir);
    let temp_path = path.with_extension("json.tmp");
    let body = serde_json::to_string_pretty(fingerprint)
        .context("failed to serialize the database fingerprint")?;
    fs::write(&temp_path, body)
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("failed to replace {}", path.display()))?;
    }
    fs::rename(&temp_path, &path).with_context(|| format!("failed to finalize {}", path.display()))
}

/// Record the baseline only when there is none — the "first launch of the new
/// version" half of the §9.4 gate.
///
/// A failure here must never stop the app from starting; the gate simply has
/// no baseline to compare against on the next update.
pub fn record_db_fingerprint_if_absent(app_dir: &Path, conn: &Connection) -> Result<()> {
    if load_db_fingerprint(app_dir)?.is_some() {
        return Ok(());
    }
    let fingerprint = read_db_fingerprint(conn)?;
    record_db_fingerprint(app_dir, &fingerprint)
}
