pub mod backup_ops;
pub mod backup_service;
pub mod file_ops;
pub mod google_drive;
pub mod models;
pub mod restore_service;
pub mod scheduling;
mod sqlite_utils;

// Backward-compatible re-exports so callers using `backup::service::*` still compile.
// Used by commands/mod.rs (`use crate::backup::service as backup_service`)
// and lib.rs (`backup::service::spawn_backup_scheduler(...)`).
pub mod service {
    pub use super::backup_service::*;
    pub use super::restore_service::*;
    pub use super::scheduling::*;
}
