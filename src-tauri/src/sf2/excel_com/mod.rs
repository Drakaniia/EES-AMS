pub mod calendar;
pub mod com_session;
pub mod learners;
pub mod workbook;
pub mod workbook_analysis;
pub mod workbook_io;
pub mod workbook_ops;
pub mod workbook_utils;
pub mod worksheet;

// Re-export public API surface (used by excel.rs)
pub use workbook::WorkbookSession;
pub use workbook_analysis::analyze_workbook;
pub use workbook_io::{write_formulas, write_marks, write_marks_force, write_metadata};
pub use workbook_ops::{batch_operations, expand_roster_rows, hide_empty_learner_rows};
