use crate::domain::error::{AppError, Result};
use crate::sf2::logic::Sf2CellMark;
use crate::sf2::models::{Sf2WorkbookAnalysis, Sf2WorkbookMetadata};
use std::collections::HashSet;
use std::path::Path;

// Re-export the WorkbookSession type on Windows so callers can use it.
#[cfg(target_os = "windows")]
pub use super::excel_com::WorkbookSession;

/// Placeholder for non-Windows platforms.
#[cfg(not(target_os = "windows"))]
pub struct WorkbookSession;

/// Perform multiple Excel operations in a single Excel process.
///
/// Opens the workbook, runs `action` with a session handle that exposes
/// all the standard operations (analyze, write_marks, write_metadata, etc.),
/// then saves (if `save_on_close`) and closes.
///
/// This is the Phase 1 batching API — call once instead of 9 separate times.
pub fn batch_operations<T, F>(path: &Path, save_on_close: bool, action: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&WorkbookSession) -> Result<T> + Send + 'static,
{
    batch_operations_impl(path, save_on_close, action)
}

pub fn analyze_workbook(path: &Path) -> Result<Sf2WorkbookAnalysis> {
    analyze_workbook_impl(path)
}

pub fn write_workbook(source_path: &Path, output_path: &Path, marks: &[Sf2CellMark]) -> Result<()> {
    std::fs::copy(source_path, output_path)
        .map_err(|error| AppError::Internal(format!("failed to copy SF2 workbook: {error}")))?;

    write_marks(output_path, marks)?;
    Ok(())
}

pub fn write_formulas(workbook_path: &Path, formula_marks: &[Sf2CellMark]) -> Result<()> {
    if formula_marks.is_empty() {
        return Ok(());
    }
    write_formulas_impl(workbook_path, formula_marks)
}

pub fn write_marks(workbook_path: &Path, marks: &[Sf2CellMark]) -> Result<()> {
    if marks.is_empty() {
        return Ok(());
    }

    write_marks_impl(workbook_path, marks)
}

pub fn write_marks_force(workbook_path: &Path, marks: &[Sf2CellMark]) -> Result<()> {
    if marks.is_empty() {
        return Ok(());
    }

    write_marks_force_impl(workbook_path, marks)
}

#[cfg(target_os = "windows")]
fn write_marks_force_impl(workbook_path: &Path, marks: &[Sf2CellMark]) -> Result<()> {
    super::excel_com::write_marks_force(workbook_path, marks)
}

#[cfg(not(target_os = "windows"))]
fn write_marks_force_impl(_workbook_path: &Path, _marks: &[Sf2CellMark]) -> Result<()> {
    Err(unsupported_excel_automation())
}

pub fn hide_empty_learner_rows(
    workbook_path: &Path,
    male_total_row: u32,
    female_total_row: u32,
    occupied_rows: &HashSet<u32>,
) -> Result<()> {
    hide_empty_learner_rows_impl(
        workbook_path,
        male_total_row,
        female_total_row,
        occupied_rows,
    )
}

pub fn expand_roster_rows(
    workbook_path: &Path,
    extra_male_rows: u32,
    extra_female_rows: u32,
    male_total_row: Option<u32>,
    female_total_row: Option<u32>,
) -> Result<()> {
    expand_roster_rows_impl(
        workbook_path,
        extra_male_rows,
        extra_female_rows,
        male_total_row,
        female_total_row,
    )
}

pub fn write_metadata(workbook_path: &Path, metadata: &Sf2WorkbookMetadata) -> Result<()> {
    write_metadata_impl(workbook_path, metadata)
}

#[cfg(target_os = "windows")]
fn batch_operations_impl<T, F>(path: &Path, save_on_close: bool, action: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&super::excel_com::WorkbookSession) -> Result<T> + Send + 'static,
{
    super::excel_com::batch_operations(path, save_on_close, action)
}

#[cfg(not(target_os = "windows"))]
fn batch_operations_impl<T, F>(_path: &Path, _save_on_close: bool, _action: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&WorkbookSession) -> Result<T> + Send + 'static,
{
    Err(unsupported_excel_automation())
}

#[cfg(target_os = "windows")]
fn analyze_workbook_impl(path: &Path) -> Result<Sf2WorkbookAnalysis> {
    super::excel_com::analyze_workbook(path)
}

#[cfg(not(target_os = "windows"))]
fn analyze_workbook_impl(_path: &Path) -> Result<Sf2WorkbookAnalysis> {
    Err(unsupported_excel_automation())
}

#[cfg(target_os = "windows")]
fn write_formulas_impl(workbook_path: &Path, marks: &[Sf2CellMark]) -> Result<()> {
    super::excel_com::write_formulas(workbook_path, marks)
}

#[cfg(not(target_os = "windows"))]
fn write_formulas_impl(_workbook_path: &Path, _marks: &[Sf2CellMark]) -> Result<()> {
    Err(unsupported_excel_automation())
}

#[cfg(target_os = "windows")]
fn write_marks_impl(workbook_path: &Path, marks: &[Sf2CellMark]) -> Result<()> {
    super::excel_com::write_marks(workbook_path, marks)
}

#[cfg(not(target_os = "windows"))]
fn write_marks_impl(_workbook_path: &Path, _marks: &[Sf2CellMark]) -> Result<()> {
    Err(unsupported_excel_automation())
}

#[cfg(target_os = "windows")]
fn hide_empty_learner_rows_impl(
    workbook_path: &Path,
    male_total_row: u32,
    female_total_row: u32,
    occupied_rows: &HashSet<u32>,
) -> Result<()> {
    super::excel_com::hide_empty_learner_rows(
        workbook_path,
        male_total_row,
        female_total_row,
        occupied_rows,
    )
}

#[cfg(not(target_os = "windows"))]
fn hide_empty_learner_rows_impl(
    _workbook_path: &Path,
    _male_total_row: u32,
    _female_total_row: u32,
    _occupied_rows: &HashSet<u32>,
) -> Result<()> {
    Err(unsupported_excel_automation())
}

#[cfg(target_os = "windows")]
fn expand_roster_rows_impl(
    workbook_path: &Path,
    extra_male_rows: u32,
    extra_female_rows: u32,
    male_total_row: Option<u32>,
    female_total_row: Option<u32>,
) -> Result<()> {
    super::excel_com::expand_roster_rows(
        workbook_path,
        extra_male_rows,
        extra_female_rows,
        male_total_row,
        female_total_row,
    )
}

#[cfg(not(target_os = "windows"))]
fn expand_roster_rows_impl(
    _workbook_path: &Path,
    _extra_male_rows: u32,
    _extra_female_rows: u32,
    _male_total_row: Option<u32>,
    _female_total_row: Option<u32>,
) -> Result<()> {
    Err(unsupported_excel_automation())
}

#[cfg(target_os = "windows")]
fn write_metadata_impl(workbook_path: &Path, metadata: &Sf2WorkbookMetadata) -> Result<()> {
    super::excel_com::write_metadata(workbook_path, metadata)
}

#[cfg(not(target_os = "windows"))]
fn write_metadata_impl(_workbook_path: &Path, _metadata: &Sf2WorkbookMetadata) -> Result<()> {
    Err(unsupported_excel_automation())
}

#[cfg(not(target_os = "windows"))]
fn unsupported_excel_automation() -> AppError {
    AppError::InvalidInput(
        "SF2 Excel automation requires Windows with Microsoft Excel installed".to_string(),
    )
}
