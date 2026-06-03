use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupKind {
    Auto,
    Manual,
    PreRestore,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub source_path: String,
    pub file_name: String,
    pub modified_at: i64,
    pub size_bytes: u64,
    pub schema_version: i32,
    pub student_count: i64,
    pub class_count: i64,
    pub event_count: i64,
    pub settings_count: i64,
    pub sf2_template_count: i64,
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
    pub warnings: Vec<String>,
}
