// Re-export facade for backward compatibility with consumers (commands/sf2.rs, commands/attendance.rs)
// Functions have been split into specialized modules:
//   - attendance_service.rs  — attendance event recording & mark computation
//   - calendar_service.rs    — template creation & roster management
//   - excel_service.rs       — export, preview & workbook settings
//   - validation_service.rs  — import validation orchestration

pub use super::attendance_service::{
    set_preview_attendance,
    set_preview_attendance_lightweight,
    sync_and_open_sf2_workbook,
    sync_attendance_to_sf2_workbook,
};

pub use super::template_ops::{
    create_workbook_from_template,
    set_report_month,
    update_workbook_settings,
};

pub use super::roster_sync::{
    sync_workbook_roster_for_class,
};

pub use super::excel_service::{
    export_preview,
    export_readiness,
    export_workbook,
    open_workbook,
    workbook_settings,
};

pub use super::validation_service::{
    import_workbook,
    validate_workbook_import,
};
