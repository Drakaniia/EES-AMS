pub mod workbook;
pub mod worksheet;
pub mod calendar;
pub mod learners;

// Re-export public API surface (used by excel.rs)
pub use workbook::{analyze_workbook, write_marks, write_metadata};
