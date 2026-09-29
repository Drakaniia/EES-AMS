use super::backup_ops::NO_WORKBOOKS_MARKER;
use super::backup_service::{self, preview_backup};
use super::file_ops::resolve_backup_database;
use super::models::{BackupKind, RestoreResult};
use super::workbooks;
use crate::infrastructure::database::{migrate_db, DbPool};
use anyhow::{Context, Result};
use chrono::Local;
use rusqlite::{Connection, DatabaseName};
use std::path::Path;

/// Restore a backup: `attendance.db` **and** the SF2 workbooks.
///
/// The safety copy taken first is a full backup folder, so it already contains
/// the workbooks as they are right now — a restore can therefore never destroy
/// the only copy of an X mark.
///
/// The workbooks are written back *after* the database, so a workbook write
/// that fails leaves a consistent database rather than a half-restored pair.
/// The failure is reported in `RestoreResult::warnings` rather than as an
/// error: the database restore itself succeeded, and the safety backup still
/// holds everything.
pub fn restore_backup(pool: &DbPool, app_dir: &Path, source_path: &Path) -> Result<RestoreResult> {
    let preview = preview_backup(source_path)?;
    let pre_restore_backup =
        backup_service::create_backup_at(pool, app_dir, BackupKind::PreRestore, Local::now())
            .context("failed to create pre-restore safety backup")?;

    let mut warnings = preview.warnings.clone();

    // Re-checked here as well as in the preview: the preview is what the user
    // saw and confirmed, and this is the last point before the database is
    // overwritten.
    if let Some(warning) =
        backup_service::workbook_absence_warning(&preview.workbooks, preview.absent_count)
    {
        log::warn!("restoring a workbook-ahead-of-database backup: {warning}");
        if !warnings.contains(&warning) {
            warnings.push(warning);
        }
    }

    let database_path = resolve_backup_database(source_path)?;
    let mut pooled = pool.get().context("failed to get database connection")?;
    let conn: &mut Connection = &mut pooled;
    conn.restore(
        DatabaseName::Main,
        &database_path,
        None::<fn(rusqlite::backup::Progress)>,
    )
    .with_context(|| format!("failed to restore backup {}", database_path.display()))?;
    migrate_db(conn).context("failed to migrate restored database")?;

    backup_service::run_integrity_check(conn)
        .context("restored database failed integrity check")?;

    let schema_version = backup_service::read_schema_version(conn)?;
    drop(pooled);

    let workbooks_restored = match workbooks::restore_workbooks_from(app_dir, source_path) {
        Ok(count) => count > 0,
        Err(error) => {
            let warning = format!(
                "The database was restored but the SF2 workbooks could not be written back: \
                 {error}. The previous workbooks are unchanged, and the safety backup at {} \
                 holds both.",
                pre_restore_backup.path
            );
            log::error!("{warning}");
            warnings.push(warning);
            false
        }
    };

    if !workbooks_restored
        && !warnings
            .iter()
            .any(|warning| warning.contains(NO_WORKBOOKS_MARKER))
    {
        warnings.push(format!(
            "This backup held {NO_WORKBOOKS_MARKER}, so the current workbooks were left in place."
        ));
    }

    Ok(RestoreResult {
        restored_path: source_path.to_string_lossy().to_string(),
        pre_restore_backup_path: pre_restore_backup.path,
        restored_at: chrono::Utc::now().timestamp(),
        schema_version,
        migrated: preview.schema_version < crate::infrastructure::database::CURRENT_SCHEMA_VERSION,
        workbooks_restored,
        warnings,
    })
}
