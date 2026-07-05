use super::file_ops::{
    backup_dir, is_app_backup_file, load_state, save_state, summary_from_path,
    unique_backup_path, BackupState, SYNC_BACKUP_DIR_NAME,
};
use super::models::{BackupKind, BackupPreview, BackupStatus, BackupSummary};
use super::sqlite_utils::{count_table_rows, read_schema_version, require_core_tables, run_integrity_check};
use crate::infrastructure::database::DbPool;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local};
use rusqlite::{Connection, DatabaseName, OpenFlags};
use std::{
    fs,
    path::{Path, PathBuf},
};

// ── Public API ────────────────────────────────────────────────────────

pub fn get_status(app_dir: &Path) -> Result<BackupStatus> {
    let backups = list_backups(app_dir)?;
    let backup_dir = backup_dir(app_dir);
    let state = load_state(app_dir).unwrap_or_else(|error| BackupState {
        last_error: Some(format!("Failed to read backup settings: {error}")),
        ..BackupState::default()
    });
    let latest = backups.first();
    let google_drive = state.google_drive.clone();

    Ok(BackupStatus {
        local_backup_dir: backup_dir.to_string_lossy().to_string(),
        backup_count: backups.len(),
        retention_limit: RETENTION_LIMIT,
        last_backup_at: state
            .last_backup_at
            .or_else(|| latest.map(|backup| backup.created_at)),
        last_backup_path: state
            .last_backup_path
            .or_else(|| latest.map(|backup| backup.path.clone())),
        sync_folder_path: state.sync_folder_path,
        last_error: state.last_error,
        last_sync_error: state.last_sync_error,
        google_drive_configured: google_drive_client_id().is_ok(),
        google_drive_connected: google_drive.is_some(),
        google_drive_folder_id: google_drive.as_ref().map(|drive| drive.folder_id.clone()),
        google_drive_folder_name: google_drive.as_ref().map(|drive| drive.folder_name.clone()),
        last_google_drive_backup_at: google_drive.as_ref().and_then(|drive| drive.last_backup_at),
        last_google_drive_file_id: google_drive
            .as_ref()
            .and_then(|drive| drive.last_file_id.clone()),
        last_google_drive_error: google_drive
            .as_ref()
            .and_then(|drive| drive.last_error.clone()),
    })
}

pub fn list_backups(app_dir: &Path) -> Result<Vec<BackupSummary>> {
    let backup_dir = backup_dir(app_dir);
    fs::create_dir_all(&backup_dir)
        .with_context(|| format!("failed to create backup directory {}", backup_dir.display()))?;

    let mut backups = Vec::new();
    for entry in fs::read_dir(&backup_dir)
        .with_context(|| format!("failed to read backup directory {}", backup_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("db") {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !is_app_backup_file(file_name) {
            continue;
        }

        backups.push(summary_from_path(&path)?);
    }

    backups.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.file_name.cmp(&left.file_name))
    });

    Ok(backups)
}

pub fn create_manual_backup(pool: &DbPool, app_dir: &Path) -> Result<BackupStatus> {
    create_backup_at(pool, app_dir, BackupKind::Manual, Local::now())?;
    get_status(app_dir)
}

pub fn backup_database_to_path(pool: &DbPool, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("backup destination has no parent directory"))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;

    let temp_path = destination.with_extension("db.tmp");
    if temp_path.exists() {
        fs::remove_file(&temp_path).with_context(|| {
            format!("failed to remove stale temp backup {}", temp_path.display())
        })?;
    }

    let source = pool.get().context("failed to get database connection")?;
    source
        .backup(
            DatabaseName::Main,
            &temp_path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .with_context(|| format!("failed to export database {}", temp_path.display()))?;
    preview_backup(&temp_path).context("exported database failed validation")?;

    if destination.exists() {
        fs::remove_file(destination)
            .with_context(|| format!("failed to replace {}", destination.display()))?;
    }
    fs::rename(&temp_path, destination).with_context(|| {
        format!(
            "failed to finalize database export {} -> {}",
            temp_path.display(),
            destination.display()
        )
    })?;

    Ok(())
}

pub fn set_sync_folder(app_dir: &Path, folder_path: Option<PathBuf>) -> Result<BackupStatus> {
    let mut state = load_state(app_dir).unwrap_or_default();
    state.sync_folder_path = folder_path
        .map(|path| prepare_sync_folder(&path))
        .transpose()?
        .map(|path| path.to_string_lossy().to_string());
    state.last_sync_error = None;
    save_state(app_dir, &state)?;
    get_status(app_dir)
}

pub fn preview_backup(source_path: &Path) -> Result<BackupPreview> {
    if !source_path.exists() {
        bail!("Backup file does not exist: {}", source_path.display());
    }

    let metadata = fs::metadata(source_path)
        .with_context(|| format!("failed to inspect backup {}", source_path.display()))?;
    let modified_at = super::file_ops::metadata_timestamp(&metadata)?;
    let file_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("backup.db")
        .to_string();

    let conn = Connection::open_with_flags(source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open backup {}", source_path.display()))?;
    run_integrity_check(&conn)?;

    let schema_version = read_schema_version(&conn)?;
    if schema_version > crate::infrastructure::database::CURRENT_SCHEMA_VERSION {
        bail!(
            "Backup schema version {schema_version} is newer than this app supports ({})",
            crate::infrastructure::database::CURRENT_SCHEMA_VERSION
        );
    }

    require_core_tables(&conn)?;

    let mut warnings = Vec::new();
    if schema_version < crate::infrastructure::database::CURRENT_SCHEMA_VERSION {
        warnings.push(format!(
            "Backup will be migrated from schema version {schema_version} to {} during restore.",
            crate::infrastructure::database::CURRENT_SCHEMA_VERSION
        ));
    }

    Ok(BackupPreview {
        source_path: source_path.to_string_lossy().to_string(),
        file_name,
        modified_at,
        size_bytes: metadata.len(),
        schema_version,
        student_count: count_table_rows(&conn, "students")?,
        class_count: count_table_rows(&conn, "classes")?,
        event_count: count_table_rows(&conn, "events")?,
        settings_count: count_table_rows(&conn, "settings")?,
        sf2_template_count: count_table_rows(&conn, "sf2_templates")?,
        warnings,
    })
}

pub fn enforce_retention(app_dir: &Path) -> Result<()> {
    let backups = list_backups(app_dir)?;
    for backup in backups.into_iter().skip(RETENTION_LIMIT) {
        fs::remove_file(&backup.path)
            .with_context(|| format!("failed to remove old backup {}", backup.path))?;
    }
    Ok(())
}

// ── Core Backup ───────────────────────────────────────────────────────

pub(crate) fn create_backup_at(
    pool: &DbPool,
    app_dir: &Path,
    kind: BackupKind,
    now: DateTime<Local>,
) -> Result<BackupSummary> {
    let backup_dir = backup_dir(app_dir);
    fs::create_dir_all(&backup_dir)
        .with_context(|| format!("failed to create backup directory {}", backup_dir.display()))?;

    let final_path = unique_backup_path(&backup_dir, kind, now);
    let temp_path = final_path.with_file_name(format!(
        "{}.tmp",
        final_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("attendance-backup.db")
    ));
    if temp_path.exists() {
        fs::remove_file(&temp_path).with_context(|| {
            format!("failed to remove stale temp backup {}", temp_path.display())
        })?;
    }

    let source = pool.get().context("failed to get database connection")?;
    source
        .backup(
            DatabaseName::Main,
            &temp_path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .with_context(|| format!("failed to create backup {}", temp_path.display()))?;

    preview_backup(&temp_path).context("created backup failed validation")?;
    fs::rename(&temp_path, &final_path).with_context(|| {
        format!(
            "failed to finalize backup {} -> {}",
            temp_path.display(),
            final_path.display()
        )
    })?;

    enforce_retention(app_dir)?;

    let summary = summary_from_path(&final_path)?;
    let mut state = load_state(app_dir).unwrap_or_default();
    state.last_backup_at = Some(summary.created_at);
    state.last_backup_path = Some(summary.path.clone());
    state.last_error = None;
    state.last_sync_error = copy_to_sync_folder(&state, &final_path).err().map(|error| {
        log::warn!("backup sync failed: {error}");
        error.to_string()
    });
    if let Err(error) = super::google_drive::upload_backup_to_google_drive(&mut state, &final_path) {
        if let Some(google_drive) = state.google_drive.as_mut() {
            google_drive.last_error = Some(error.to_string());
        }
        log::warn!("Google Drive backup upload failed: {error}");
    }
    save_state(app_dir, &state)?;

    Ok(summary)
}

// ── Sync Folder ───────────────────────────────────────────────────────

fn copy_to_sync_folder(state: &BackupState, source_path: &Path) -> Result<()> {
    let Some(sync_folder_path) = &state.sync_folder_path else {
        return Ok(());
    };

    let sync_folder = PathBuf::from(sync_folder_path);
    if !sync_folder.is_dir() {
        bail!("sync folder is unavailable: {}", sync_folder.display());
    }

    let file_name = source_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("backup file name is missing"))?;
    let destination = sync_folder.join(file_name);
    fs::copy(source_path, &destination).with_context(|| {
        format!(
            "failed to copy backup to sync folder {}",
            destination.display()
        )
    })?;

    Ok(())
}

fn prepare_sync_folder(selected_folder: &Path) -> Result<PathBuf> {
    let sync_folder = if selected_folder
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(SYNC_BACKUP_DIR_NAME))
    {
        selected_folder.to_path_buf()
    } else {
        selected_folder.join(SYNC_BACKUP_DIR_NAME)
    };

    fs::create_dir_all(&sync_folder)
        .with_context(|| format!("failed to create sync folder {}", sync_folder.display()))?;

    Ok(sync_folder)
}

// ── Helpers ───────────────────────────────────────────────────────────

fn google_drive_client_id() -> Result<String> {
    option_env!("EES_AMS_GOOGLE_CLIENT_ID")
        .map(str::to_string)
        .or_else(|| std::env::var("EES_AMS_GOOGLE_CLIENT_ID").ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Google Drive is not configured. Set EES_AMS_GOOGLE_CLIENT_ID before building the app."
            )
        })
}

const RETENTION_LIMIT: usize = 30;
