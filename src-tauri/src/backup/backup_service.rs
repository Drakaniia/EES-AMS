// Re-export facade for backward compatibility with commands/backup.rs and sibling modules.
// The file was split into:
//   - file_ops.rs       — file path/naming helpers, state load/save, BackupState type
//   - sqlite_utils.rs   — SQLite helper functions (integrity check, table queries)
//   - google_drive.rs   — Google Drive OAuth, folder/upload/token management
//   - backup_ops.rs     — core backup creation, listing, status, preview, sync folder

#![allow(unused_imports)]

pub use super::backup_ops::{
    backup_database_to_path,
    create_manual_backup,
    enforce_retention,
    get_status,
    list_backups,
    preview_backup,
    set_sync_folder,
};
pub(crate) use super::backup_ops::create_backup_at;

pub use super::google_drive::{
    connect_google_drive,
    disconnect_google_drive,
    upload_latest_backup_to_google_drive,
};

pub(crate) use super::sqlite_utils::{
    read_schema_version,
    run_integrity_check,
};

pub(crate) use super::file_ops::{
    load_state,
    save_state,
};
