// Re-export facade for backward compatibility with commands/backup.rs and sibling modules.
// The file was split into:
//   - file_ops.rs       — file path/naming helpers, state load/save, BackupState type
//   - manifest.rs       — manifest.json types, read/write, backup folder layout
//   - workbooks.rs      — SF2 workbook tree snapshot and restore
//   - x_count.rs        — bulk COUNTIF read of the X marks in a workbook
//   - zip_writer.rs     — store-only ZIP used for the Google Drive upload
//   - fingerprint.rs    — the §9.4 update-time attendance fingerprint
//   - sqlite_utils.rs   — SQLite helper functions (integrity check, table queries)
//   - google_drive.rs   — Google Drive OAuth, folder/upload/token management
//   - backup_ops.rs     — core backup creation, listing, status, preview, sync folder

#![allow(unused_imports)]

pub use super::backup_ops::{
    backup_database_to_path, create_manual_backup, create_workbooks_backup, enforce_retention,
    get_status, list_backups, preview_backup, set_sync_folder, workbook_absence_warning,
};
pub(crate) use super::backup_ops::{create_backup_at, snapshot_workbooks_for};

pub use super::google_drive::{
    connect_google_drive, disconnect_google_drive, upload_latest_backup_to_google_drive,
};

pub use super::restore_service::restore_backup;

pub(crate) use super::fingerprint::{
    load_db_fingerprint, read_db_fingerprint, record_db_fingerprint,
    record_db_fingerprint_if_absent, DbFingerprint,
};

pub(crate) use super::manifest::{BackupManifest, ManifestCounts, ManifestWorkbook};

pub(crate) use super::sqlite_utils::{read_schema_version, run_integrity_check};

pub(crate) use super::file_ops::{load_state, save_state};

pub use super::workbooks::{restore_workbooks_from, snapshot_workbooks_into, WorkbookSnapshot};
