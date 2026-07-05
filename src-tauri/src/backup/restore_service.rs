use super::backup_service::{self, preview_backup};
use super::models::{BackupKind, RestoreResult};
use crate::infrastructure::database::{migrate_db, DbPool};
use anyhow::{Context, Result};
use chrono::Local;
use rusqlite::{Connection, DatabaseName};
use std::path::Path;

pub fn restore_backup(pool: &DbPool, app_dir: &Path, source_path: &Path) -> Result<RestoreResult> {
    let preview = preview_backup(source_path)?;
    let pre_restore_backup = backup_service::create_backup_at(
        pool,
        app_dir,
        BackupKind::PreRestore,
        Local::now(),
    )
    .context("failed to create pre-restore safety backup")?;

    let mut pooled = pool.get().context("failed to get database connection")?;
    let conn: &mut Connection = &mut pooled;
    conn.restore(
        DatabaseName::Main,
        source_path,
        None::<fn(rusqlite::backup::Progress)>,
    )
    .with_context(|| format!("failed to restore backup {}", source_path.display()))?;
    migrate_db(conn).context("failed to migrate restored database")?;

    backup_service::run_integrity_check(conn)
        .context("restored database failed integrity check")?;

    let schema_version = backup_service::read_schema_version(conn)?;

    Ok(RestoreResult {
        restored_path: source_path.to_string_lossy().to_string(),
        pre_restore_backup_path: pre_restore_backup.path,
        restored_at: chrono::Utc::now().timestamp(),
        schema_version,
        migrated: preview.schema_version < crate::infrastructure::database::CURRENT_SCHEMA_VERSION,
        warnings: preview.warnings,
    })
}
