// Re-export facade for backward compatibility.
// The file was split into:
//   - roster.rs          — roster slot assignment, name dedup, learner sync
//   - template_ops.rs    — workbook creation and update orchestration
//   - roster_sync.rs     — roster sync from class to workbook

// Allow unused imports: these re-exports maintain backward compatibility
// for sibling modules (attendance_service, excel_service, validation_service)
// that import through `super::calendar_service::`.
#![allow(unused_imports)]

pub(crate) use super::roster::{
    find_or_create_class, reject_duplicate_roster_names, sync_workbook_learner_mappings,
    template_owns_roster, template_roster_assignments, template_roster_slots,
    unique_normalized_name, TemplateRosterSlot, WorkbookLearnerSync,
};

pub(crate) use super::roster_sync::{
    sync_latest_workbook_roster_for_class, sync_template_roster_from_class,
    sync_workbook_roster_for_class,
};

pub(crate) use super::template_ops::create_workbook_from_template;
pub(crate) use super::template_update::update_workbook_settings;
