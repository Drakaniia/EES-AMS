pub mod backup_ops;
pub mod backup_service;
pub mod file_ops;
pub mod fingerprint;
pub mod google_drive;
pub mod manifest;
pub mod models;
pub mod restore_service;
pub mod scheduling;
pub(crate) mod sqlite_utils;
pub mod workbooks;
mod x_count;
mod zip_writer;

#[cfg(test)]
mod __tests__;

// Backward-compatible re-exports so callers using `backup::service::*` still compile.
// Used by commands/mod.rs (`use crate::backup::service as backup_service`)
// and lib.rs (`backup::service::spawn_backup_scheduler(...)`).
pub mod service {
    pub use super::backup_service::*;
    pub use super::scheduling::*;
}
