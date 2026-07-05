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

pub fn ensure_daily_backup(pool: &DbPool, app_dir: &Path) -> Result<()> {
    let now = Local::now();
    ensure_daily_backup_at(pool, app_dir, now).map(|_| ())
}

pub fn ensure_daily_backup_at(
    pool: &DbPool,
    app_dir: &Path,
    now: DateTime<Local>,
) -> Result<()> {
    let today = now.date_naive();
    let has_backup_today = backup_service::list_backups(app_dir)?
        .iter()
        .any(|backup| {
            chrono::DateTime::from_timestamp(backup.created_at, 0)
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
