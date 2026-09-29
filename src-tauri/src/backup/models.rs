use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupKind {
    Auto,
    Manual,
    /// Safety copy taken immediately before a restore replaces the database.
    PreRestore,
    /// Safety copy taken immediately before "wipe all data" destroys the
    /// contents of the live database.
    PreWipe,
    /// Safety copy taken immediately before an update installer replaces the
    /// app binaries. The comparison baseline for the §9.4 integrity gate.
    PreInstall,
    /// The D13 "Back up workbooks now" folder: the `workbooks/` subtree and a
    /// manifest, with no database copy.
    ManualWorkbooks,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub path: String,
    pub file_name: String,
    pub created_at: i64,
    pub size_bytes: u64,
    pub kind: BackupKind,
    /// False for the D13 workbooks-only folder, which carries no `attendance.db`.
    pub includes_database: bool,
    /// Workbook files inside the folder. Empty for a legacy flat `*.db` backup.
    pub workbook_count: usize,
    /// Sum of `xCount` across the manifest's workbooks. 0 when unknown.
    pub total_x_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub local_backup_dir: String,
    pub backup_count: usize,
    pub retention_limit: usize,
    pub last_backup_at: Option<i64>,
    pub last_backup_path: Option<String>,
    pub sync_folder_path: Option<String>,
    pub last_error: Option<String>,
    pub last_sync_error: Option<String>,
    pub google_drive_configured: bool,
    pub google_drive_connected: bool,
    pub google_drive_folder_id: Option<String>,
    pub google_drive_folder_name: Option<String>,
    pub last_google_drive_backup_at: Option<i64>,
    pub last_google_drive_file_id: Option<String>,
    pub last_google_drive_error: Option<String>,
    /// Path of the most recent workbook-only backup, surfaced for the
    /// "Back up workbooks now" button's confirmation.
    pub last_workbooks_backup_path: Option<String>,
}

/// One workbook listed in a backup's `manifest.json`, resolved to an absolute
/// path so the restore dialog can show what will be written back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupWorkbookPreview {
    pub file_name: String,
    /// Path relative to the backup folder, as stored in the manifest.
    pub relative_path: String,
    pub bytes: u64,
    pub x_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    /// The folder (or legacy file) the user picked.
    pub source_path: String,
    /// The `attendance.db` actually opened, which is `source_path` itself for a
    /// legacy flat backup.
    pub database_path: String,
    pub file_name: String,
    pub modified_at: i64,
    pub size_bytes: u64,
    pub schema_version: i32,
    pub student_count: i64,
    pub class_count: i64,
    pub event_count: i64,
    pub absent_count: i64,
    pub settings_count: i64,
    pub sf2_template_count: i64,
    /// False for the D13 workbooks-only folder, which has no database to restore.
    pub includes_database: bool,
    pub workbooks: Vec<BackupWorkbookPreview>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub restored_path: String,
    pub pre_restore_backup_path: String,
    pub restored_at: i64,
    pub schema_version: i32,
    pub migrated: bool,
    /// True when the backup carried a `workbooks/` subtree and it was written
    /// back over the live SF2 workbooks.
    pub workbooks_restored: bool,
    pub warnings: Vec<String>,
}
