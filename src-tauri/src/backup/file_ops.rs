use super::manifest::{backup_db_path, read_manifest};
use super::models::{BackupKind, BackupSummary};
use anyhow::{bail, Context, Result};
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
    pub(crate) last_workbooks_backup_path: Option<String>,
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

// ── Backup Shape Resolution ───────────────────────────────────────────
//
// Backups exist in two shapes and both must keep working:
//
//   * the folder shape written today — `backups/attendance-<kind>-<ts>/`
//     holding `attendance.db`, `manifest.json` and `workbooks/`
//   * the legacy flat shape written by the previous build — a single
//     `backups/attendance-<kind>-<ts>.db` file with no workbooks
//
// Every read path resolves through [`resolve_backup_database`] so a caller
// never has to know which shape it was handed.

/// The `attendance.db` a backup path refers to.
///
/// A folder resolves to the database inside it; a legacy flat `.db` resolves to
/// itself.
pub(crate) fn resolve_backup_database(source_path: &Path) -> Result<PathBuf> {
    if source_path.is_file() {
        return Ok(source_path.to_path_buf());
    }
    if source_path.is_dir() {
        let database = backup_db_path(source_path);
        if database.is_file() {
            return Ok(database);
        }
    }
    bail!(
        "Backup has no {}: {}",
        super::manifest::BACKUP_DB_FILE_NAME,
        source_path.display()
    )
}

/// True when the path is a folder-shaped backup this app wrote.
pub(crate) fn is_app_backup_folder(file_name: &str) -> bool {
    file_name.starts_with(BACKUP_PREFIX)
        && !file_name.ends_with(".db")
        && !file_name.ends_with(".tmp")
}

/// True when the path is a legacy flat `*.db` backup this app wrote.
pub(crate) fn is_app_backup_file(file_name: &str) -> bool {
    file_name.starts_with(BACKUP_PREFIX) && file_name.ends_with(".db")
}

// ── File Naming & Listing ─────────────────────────────────────────────

/// Build a [`BackupSummary`] for either backup shape.
///
/// A folder summary prefers the manifest's `createdAt` and workbook list, and
/// falls back to the directory's own mtime and an empty workbook list when the
/// manifest is missing or unreadable — a folder without a readable manifest is
/// still a backup the user may need.
pub(crate) fn summary_from_path(path: &Path) -> Result<BackupSummary> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to inspect {}", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow::anyhow!("backup file name is invalid"))?
        .to_string();

    let manifest = if path.is_dir() {
        match read_manifest(path) {
            Ok(manifest) => manifest,
            Err(error) => {
                log::warn!(
                    "backup folder {} has an unreadable manifest: {error}",
                    path.display()
                );
                None
            }
        }
    } else {
        None
    };

    let includes_database = if path.is_dir() {
        backup_db_path(path).is_file()
    } else {
        true
    };

    let size_bytes = if path.is_dir() {
        directory_size(path)?
    } else {
        metadata.len()
    };

    let created_at = manifest
        .as_ref()
        .and_then(|manifest| {
            manifest
                .created_at
                .parse::<DateTime<FixedOffsetLike>>()
                .ok()
        })
        .map(|created_at| created_at.timestamp())
        .or_else(|| backup_timestamp_from_name(&file_name))
        .unwrap_or(metadata_timestamp(&metadata)?);

    let workbook_count = manifest
        .as_ref()
        .map(|manifest| manifest.workbooks.len())
        .unwrap_or_else(|| {
            if path.is_dir() {
                super::workbooks::list_files(&super::manifest::workbooks_path(path))
                    .map(|files| files.len())
                    .unwrap_or(0)
            } else {
                0
            }
        });

    let total_x_count = manifest
        .as_ref()
        .map(|manifest| {
            manifest
                .workbooks
                .iter()
                .map(|workbook| workbook.x_count)
                .sum()
        })
        .unwrap_or(0);

    Ok(BackupSummary {
        path: path.to_string_lossy().to_string(),
        file_name: file_name.clone(),
        created_at,
        size_bytes,
        kind: backup_kind_from_name(&file_name),
        includes_database,
        workbook_count,
        total_x_count,
    })
}

/// Total bytes of every file below a directory.
pub(crate) fn directory_size(dir: &Path) -> Result<u64> {
    let mut total = 0;
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current)
            .with_context(|| format!("failed to read {}", current.display()))?
        {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                pending.push(entry.path());
            } else if entry.file_type()?.is_file() {
                total += entry.metadata().map(|meta| meta.len()).unwrap_or(0);
            }
        }
    }
    Ok(total)
}

/// Copy a file or a whole directory tree, replacing whatever is at the
/// destination. Used by the sync-folder mirror, which now copies folders.
pub(crate) fn copy_path_recursive(source: &Path, destination: &Path) -> Result<()> {
    if source.is_dir() {
        if destination.exists() {
            fs::remove_dir_all(destination)
                .with_context(|| format!("failed to replace {}", destination.display()))?;
        }
        copy_dir_recursive(source, destination)
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::copy(source, destination).with_context(|| {
            format!(
                "failed to copy {} -> {}",
                source.display(),
                destination.display()
            )
        })?;
        Ok(())
    }
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    for entry in
        fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        let target = destination.join(&name);
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else if entry.file_type()?.is_file() {
            fs::copy(entry.path(), &target).with_context(|| {
                format!(
                    "failed to copy {} -> {}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

/// A folder path for a new backup, uniquified against whatever already exists.
pub(crate) fn unique_backup_path(
    backup_dir: &Path,
    kind: BackupKind,
    now: DateTime<Local>,
) -> PathBuf {
    let timestamp = now.format("%Y%m%d_%H%M%S");
    let stem = format!("{BACKUP_PREFIX}{}-{timestamp}", backup_kind_file_part(kind));
    let mut path = backup_dir.join(&stem);
    let mut suffix = 2;

    while path.exists() {
        path = backup_dir.join(format!("{stem}-{suffix}"));
        suffix += 1;
    }

    path
}

pub(crate) fn backup_kind_file_part(kind: BackupKind) -> &'static str {
    match kind {
        BackupKind::Auto => "auto",
        BackupKind::Manual => "manual",
        BackupKind::PreRestore => "pre-restore",
        BackupKind::PreWipe => "pre-wipe",
        BackupKind::PreInstall => "pre-install",
        BackupKind::ManualWorkbooks => "manual-workbooks",
        BackupKind::Unknown => "unknown",
    }
}

/// Classify a backup from its name.
///
/// `attendance-manual-workbooks-…` must be tested before `attendance-manual-…`,
/// otherwise the D13 workbooks-only folder is reported as a plain manual
/// backup.
pub(crate) fn backup_kind_from_name(name: &str) -> BackupKind {
    let stem = name.strip_suffix(".db").unwrap_or(name);
    if stem.starts_with("attendance-manual-workbooks-") {
        BackupKind::ManualWorkbooks
    } else if stem.starts_with("attendance-auto-") {
        BackupKind::Auto
    } else if stem.starts_with("attendance-manual-") {
        BackupKind::Manual
    } else if stem.starts_with("attendance-pre-restore-") {
        BackupKind::PreRestore
    } else if stem.starts_with("attendance-pre-wipe-") {
        BackupKind::PreWipe
    } else if stem.starts_with("attendance-pre-install-") {
        BackupKind::PreInstall
    } else {
        BackupKind::Unknown
    }
}

pub(crate) fn backup_timestamp_from_name(name: &str) -> Option<i64> {
    let stem = name.strip_suffix(".db").unwrap_or(name);
    let timestamp = stem
        .strip_prefix("attendance-manual-workbooks-")
        .or_else(|| stem.strip_prefix("attendance-auto-"))
        .or_else(|| stem.strip_prefix("attendance-manual-"))
        .or_else(|| stem.strip_prefix("attendance-pre-restore-"))
        .or_else(|| stem.strip_prefix("attendance-pre-wipe-"))
        .or_else(|| stem.strip_prefix("attendance-pre-install-"))?
        .split('-')
        .next()?;
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

// ── Helpers ───────────────────────────────────────────────────────────

/// `chrono::FixedOffset` under a short local alias, used only to parse the
/// manifest's RFC 3339 `createdAt`.
type FixedOffsetLike = chrono::FixedOffset;
