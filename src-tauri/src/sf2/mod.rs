// ── Domain submodules ─────────────────────────────────────────────────────
pub(crate) mod attendance;
pub(crate) mod calendar;
pub mod excel;
pub mod logic;
pub mod models;
mod naming;
mod preview;
pub(crate) mod progress;
pub mod repository;
pub(crate) mod roster;
pub mod service;
pub(crate) mod sf2_metadata;
pub(crate) mod template;
mod validation;
pub(crate) mod validation_service;
mod workbook_files;

// ── Backward-compatible re-exports ────────────────────────────────────────
// These keep `crate::sf2::module_name::Item` paths working for code that
// references moved submodules through their old flat paths.
pub(crate) use attendance::attendance_events;
pub(crate) use attendance::attendance_marks;
pub(crate) use attendance::attendance_service;
pub(crate) use calendar::calendar_service;
pub(crate) use excel::excel_preview;
pub(crate) use excel::excel_service;
pub(crate) use excel::excel_service_helpers;
pub(crate) use roster::roster_parser;
pub(crate) use roster::roster_sync;
pub(crate) use template::template_create;
pub(crate) use template::template_ops;
pub(crate) use template::template_update;
