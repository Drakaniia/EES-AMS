// Re-export facade for backward compatibility with consumers (commands/sf2.rs, commands/attendance.rs)
// Functions have been split into specialized modules:
//   - attendance_service.rs  — attendance event recording & mark computation
//   - calendar_service.rs    — template creation & roster management
//   - excel_service.rs       — export, preview & workbook settings
//   - validation_service.rs  — import validation orchestration

pub use super::attendance_import::import_absent_marks_from_workbook;
pub use super::attendance_service::{
    set_all_students_present, set_preview_attendance, set_preview_attendance_lightweight,
    sync_and_open_sf2_workbook, sync_attendance_to_sf2_workbook,
};

pub use super::template_ops::create_workbook_from_template;
pub use super::template_update::update_workbook_settings;

pub use super::roster_sync::sync_workbook_roster_for_class;

pub use super::excel_preview::export_preview;

// `set_report_month` / `set_report_month_with_progress` are gone with the Excel
// month-switch path (spec §7.2). The read that replaced them is
// `crate::sf2::month_preview::month_preview`, reached from the command layer
// directly because it is not a "service" over shared mutable state - it is a
// query, and the command is the query.

pub use super::excel_service::{
    export_readiness, export_workbook, open_workbook, workbook_settings,
};

pub use super::validation_service::{import_workbook, validate_workbook_import};
