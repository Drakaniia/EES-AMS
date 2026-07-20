pub mod calendar;
pub mod learners;
pub mod workbook;
pub mod worksheet;

// Re-export public API surface (used by excel.rs)
pub use workbook::{
    analyze_workbook, batch_operations, expand_roster_rows, hide_empty_learner_rows,
    write_formulas, write_marks, write_marks_force, write_metadata, WorkbookSession,
};
