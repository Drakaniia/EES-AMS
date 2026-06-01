use crate::domain::error::{AppError, Result};
use crate::sf2::logic::Sf2CellMark;
use crate::sf2::models::{Sf2WorkbookAnalysis, Sf2WorkbookMetadata};
use std::path::Path;

pub fn analyze_workbook(path: &Path) -> Result<Sf2WorkbookAnalysis> {
    analyze_workbook_impl(path)
}

pub fn write_workbook(source_path: &Path, output_path: &Path, marks: &[Sf2CellMark]) -> Result<()> {
    std::fs::copy(source_path, output_path)
        .map_err(|error| AppError::Internal(format!("failed to copy SF2 workbook: {error}")))?;

    write_marks(output_path, marks)?;
    Ok(())
}

pub fn write_marks(workbook_path: &Path, marks: &[Sf2CellMark]) -> Result<()> {
    if marks.is_empty() {
        return Ok(());
    }

    write_marks_impl(workbook_path, marks)
}

pub fn write_metadata(workbook_path: &Path, metadata: &Sf2WorkbookMetadata) -> Result<()> {
    write_metadata_impl(workbook_path, metadata)
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
fn write_marks_impl(workbook_path: &Path, marks: &[Sf2CellMark]) -> Result<()> {
    super::excel_com::write_marks(workbook_path, marks)
}

#[cfg(not(target_os = "windows"))]
fn write_marks_impl(_workbook_path: &Path, _marks: &[Sf2CellMark]) -> Result<()> {
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
