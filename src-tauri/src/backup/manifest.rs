use super::models::BackupKind;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

// ── Constants ─────────────────────────────────────────────────────────

/// The database file name inside a backup folder.
pub(crate) const BACKUP_DB_FILE_NAME: &str = "attendance.db";

/// The workbook subtree name inside a backup folder.
pub(crate) const WORKBOOK_DIR_NAME: &str = "workbooks";

/// The manifest file name inside a backup folder.
pub(crate) const MANIFEST_FILE_NAME: &str = "manifest.json";

// ── Types ─────────────────────────────────────────────────────────────

/// `manifest.json` — written next to every backup folder so a restore can tell
/// what the backup contains without opening Excel.
///
/// `workbooks[].xCount` is the number of cells holding `"X"` in the workbook's
/// month sheet at snapshot time. It is the cheapest possible proof that the
/// workbook holds absences, which is what the restore guard in
/// [`crate::backup::backup_ops::preview_backup`] compares against the
/// database's own absence count.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub schema_version: i32,
    pub created_at: String,
    pub kind: String,
    pub counts: ManifestCounts,
    #[serde(default)]
    pub workbooks: Vec<ManifestWorkbook>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestCounts {
    #[serde(default)]
    pub students: i64,
    #[serde(default)]
    pub events: i64,
    #[serde(default)]
    pub absent: i64,
    /// 0 when the backup's database predates the `sf2_month_templates`
    /// migration (v19), which is normal for an old backup.
    #[serde(default)]
    pub sf2_month_templates: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestWorkbook {
    /// Path relative to the backup folder, e.g. `workbooks/SF2-….xls`.
    pub path: String,
    pub bytes: u64,
    pub x_count: i64,
}

// ── Paths ─────────────────────────────────────────────────────────────

pub(crate) fn manifest_path(backup_folder: &Path) -> PathBuf {
    backup_folder.join(MANIFEST_FILE_NAME)
}

pub(crate) fn backup_db_path(backup_folder: &Path) -> PathBuf {
    backup_folder.join(BACKUP_DB_FILE_NAME)
}

pub(crate) fn workbooks_path(backup_folder: &Path) -> PathBuf {
    backup_folder.join(WORKBOOK_DIR_NAME)
}

// ── Read / Write ──────────────────────────────────────────────────────

/// Read `manifest.json` from a backup folder. `Ok(None)` when the folder has no
/// manifest — a folder written by an older build, or a workbooks-only backup
/// that never had a database.
pub(crate) fn read_manifest(backup_folder: &Path) -> Result<Option<BackupManifest>> {
    let path = manifest_path(backup_folder);
    if !path.is_file() {
        return Ok(None);
    }
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let manifest = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(manifest))
}

pub(crate) fn write_manifest(backup_folder: &Path, manifest: &BackupManifest) -> Result<()> {
    let path = manifest_path(backup_folder);
    let body = serde_json::to_string_pretty(manifest).context("failed to serialize manifest")?;
    fs::write(&path, body).with_context(|| format!("failed to write {}", path.display()))
}

// ── Kind Mapping ──────────────────────────────────────────────────────

/// The `kind` string written into the manifest. Distinct from
/// [`BackupKind`] because the manifest records *how* the backup was taken
/// (scheduled vs manual) rather than *why* it was taken.
pub(crate) fn manifest_kind(kind: BackupKind) -> &'static str {
    match kind {
        BackupKind::Auto => "scheduled",
        BackupKind::Manual => "manual",
        BackupKind::PreRestore => "pre-restore",
        BackupKind::PreWipe => "pre-wipe",
        BackupKind::PreInstall => "pre-install",
        BackupKind::ManualWorkbooks => "manual-workbooks",
        BackupKind::Unknown => "unknown",
    }
}
