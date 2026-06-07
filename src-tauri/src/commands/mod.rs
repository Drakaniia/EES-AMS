/// Tauri command interface.
mod attendance;
mod backup;
mod classes;
mod common;
mod data_transfer;
mod settings;
mod sf2;
mod students;
mod updates;

use common::*;

pub use attendance::*;
pub use backup::*;
pub use classes::*;
pub use data_transfer::*;
pub use settings::*;
pub use sf2::*;
pub use students::*;
pub use updates::*;

use crate::backup::models::{BackupPreview, BackupStatus, BackupSummary, RestoreResult};
use crate::backup::service as backup_service;
use crate::domain::models::*;
use crate::infrastructure::database::{
    record_audit_event, AuditEventInput, AuditRepository, ClassRepository, EventRepository,
    SettingsRepository, StudentRepository,
};
use crate::sf2::models::{
    Sf2CloseDaySummary, Sf2ExportPreview, Sf2ExportReadiness, Sf2ExportResult, Sf2ImportSummary,
    Sf2ImportValidation, Sf2TemplateDraft, Sf2WorkbookSettings,
};
use crate::sf2::service;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serde::Serialize;
use std::{fs, path::PathBuf};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_updater::UpdaterExt;
