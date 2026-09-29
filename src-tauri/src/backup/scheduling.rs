use super::backup_service::{self, load_state, save_state};
use crate::infrastructure::database::DbPool;
use anyhow::Result;
use chrono::{DateTime, Local};
use std::{
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

pub fn spawn_backup_scheduler(pool: DbPool, app_dir: PathBuf) {
    thread::spawn(move || {
        record_launch_fingerprint(&pool, &app_dir);

        if let Err(error) = ensure_daily_backup(&pool, &app_dir) {
            record_backup_error(&app_dir, error);
        }

        loop {
            thread::sleep(Duration::from_secs(60 * 60));
            if let Err(error) = ensure_daily_backup(&pool, &app_dir) {
                record_backup_error(&app_dir, error);
            }
        }
    });
}

/// Record the §9.4 baseline the first time this version runs.
///
/// This runs on every launch, and `record_db_fingerprint_if_absent` only writes
/// when there is no baseline, so it captures the fingerprint of whichever
/// version is running *before* that version is replaced. The next update then
/// compares the new fingerprint against it and can report a drop in attendance
/// records. Best-effort: a failure here must never stop the app from starting.
fn record_launch_fingerprint(pool: &DbPool, app_dir: &Path) {
    let conn = match pool.get() {
        Ok(conn) => conn,
        Err(error) => {
            log::warn!("failed to open the database to record its fingerprint: {error}");
            return;
        }
    };
    if let Err(error) = super::fingerprint::record_db_fingerprint_if_absent(app_dir, &conn) {
        log::warn!("failed to record the update-time database fingerprint: {error}");
    }
}

pub fn ensure_daily_backup(pool: &DbPool, app_dir: &Path) -> Result<()> {
    let now = Local::now();
    ensure_daily_backup_at(pool, app_dir, now).map(|_| ())
}

pub fn ensure_daily_backup_at(pool: &DbPool, app_dir: &Path, now: DateTime<Local>) -> Result<()> {
    let today = now.date_naive();
    // A D13 workbook-only folder is not a database backup, so it must not
    // satisfy the daily-backup check.
    let has_backup_today = backup_service::list_backups(app_dir)?.iter().any(|backup| {
        backup.includes_database
            && chrono::DateTime::from_timestamp(backup.created_at, 0)
                .map(|timestamp| timestamp.with_timezone(&Local).date_naive() == today)
                .unwrap_or(false)
    });

    if has_backup_today {
        return Ok(());
    }

    backup_service::create_backup_at(pool, app_dir, super::models::BackupKind::Auto, now)?;
    Ok(())
}

fn record_backup_error(app_dir: &Path, error: anyhow::Error) {
    let mut state = load_state(app_dir).unwrap_or_default();
    state.last_error = Some(error.to_string());
    if let Err(write_error) = save_state(app_dir, &state) {
        log::warn!("failed to record backup error: {write_error}");
    }
    log::warn!("automatic backup failed: {error}");
}
