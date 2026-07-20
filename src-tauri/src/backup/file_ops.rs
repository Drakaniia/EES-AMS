use super::models::{BackupKind, BackupSummary};
use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

// ── Constants ─────────────────────────────────────────────────────────

const BACKUP_DIR_NAME: &str = "backups";
const STATE_FILE_NAME: &str = "backup-state.json";
pub(crate) const BACKUP_PREFIX: &str = "attendance-";
pub(crate) const SYNC_BACKUP_DIR_NAME: &str = "EES-AMS Backups";
pub(crate) const KEYRING_SERVICE: &str = "ees-ams";
pub(crate) const KEYRING_REFRESH_TOKEN_USER: &str = "google-drive-refresh-token";

// ── State Types ───────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackupState {
    pub(crate) sync_folder_path: Option<String>,
    pub(crate) last_backup_at: Option<i64>,
    pub(crate) last_backup_path: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) last_sync_error: Option<String>,
    pub(crate) google_drive: Option<GoogleDriveState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GoogleDriveState {
    pub folder_id: String,
    pub folder_name: String,
    pub connected_at: i64,
    pub last_backup_at: Option<i64>,
    pub last_file_id: Option<String>,
    pub last_error: Option<String>,
}

// ── Path Helpers ──────────────────────────────────────────────────────

pub(crate) fn backup_dir(app_dir: &Path) -> PathBuf {
    app_dir.join(BACKUP_DIR_NAME)
}

fn state_path(app_dir: &Path) -> PathBuf {
    app_dir.join(STATE_FILE_NAME)
}

// ── State Load/Save ───────────────────────────────────────────────────

pub(crate) fn load_state(app_dir: &Path) -> Result<BackupState> {
    let path = state_path(app_dir);
    if !path.exists() {
        return Ok(BackupState::default());
    }

    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

pub(crate) fn save_state(app_dir: &Path, state: &BackupState) -> Result<()> {
    fs::create_dir_all(app_dir)
        .with_context(|| format!("failed to create app data directory {}", app_dir.display()))?;
    let path = state_path(app_dir);
    let temp_path = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(state)?;
    fs::write(&temp_path, content)
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("failed to replace {}", path.display()))?;
    }
    fs::rename(&temp_path, &path)
        .with_context(|| format!("failed to finalize {}", path.display()))?;
    Ok(())
}

// ── File Naming & Listing ─────────────────────────────────────────────

pub(crate) fn summary_from_path(path: &Path) -> Result<BackupSummary> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to inspect {}", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow::anyhow!("backup file name is invalid"))?
        .to_string();

    Ok(BackupSummary {
        path: path.to_string_lossy().to_string(),
        file_name: file_name.clone(),
        created_at: backup_timestamp_from_file_name(&file_name)
            .unwrap_or(metadata_timestamp(&metadata)?),
        size_bytes: metadata.len(),
        kind: backup_kind_from_file_name(&file_name),
    })
}

pub(crate) fn unique_backup_path(
    backup_dir: &Path,
    kind: BackupKind,
    now: DateTime<Local>,
) -> PathBuf {
    let timestamp = now.format("%Y%m%d_%H%M%S");
    let base_name = format!(
        "{BACKUP_PREFIX}{}-{timestamp}.db",
        backup_kind_file_part(kind)
    );
    let mut path = backup_dir.join(&base_name);
    let mut suffix = 2;

    while path.exists() {
        path = backup_dir.join(format!(
            "{BACKUP_PREFIX}{}-{timestamp}-{suffix}.db",
            backup_kind_file_part(kind)
        ));
        suffix += 1;
    }

    path
}

pub(crate) fn backup_kind_file_part(kind: BackupKind) -> &'static str {
    match kind {
        BackupKind::Auto => "auto",
        BackupKind::Manual => "manual",
        BackupKind::PreRestore => "pre-restore",
        BackupKind::Unknown => "unknown",
    }
}

pub(crate) fn backup_kind_from_file_name(file_name: &str) -> BackupKind {
    if file_name.starts_with("attendance-auto-") {
        BackupKind::Auto
    } else if file_name.starts_with("attendance-manual-") {
        BackupKind::Manual
    } else if file_name.starts_with("attendance-pre-restore-") {
        BackupKind::PreRestore
    } else {
        BackupKind::Unknown
    }
}

pub(crate) fn is_app_backup_file(file_name: &str) -> bool {
    file_name.starts_with(BACKUP_PREFIX) && file_name.ends_with(".db")
}

pub(crate) fn backup_timestamp_from_file_name(file_name: &str) -> Option<i64> {
    let timestamp = file_name
        .strip_prefix("attendance-auto-")
        .or_else(|| file_name.strip_prefix("attendance-manual-"))
        .or_else(|| file_name.strip_prefix("attendance-pre-restore-"))?
        .trim_end_matches(".db");
    let timestamp = timestamp.split('-').next().unwrap_or(timestamp);
    let naive = NaiveDateTime::parse_from_str(timestamp, "%Y%m%d_%H%M%S").ok()?;
    Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
        .map(|value| value.timestamp())
}

pub(crate) fn metadata_timestamp(metadata: &fs::Metadata) -> Result<i64> {
    let modified: DateTime<Utc> = metadata
        .modified()
        .context("failed to read file modified time")?
        .into();
    Ok(modified.timestamp())
}
