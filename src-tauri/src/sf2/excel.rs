use crate::domain::error::{AppError, Result};
use crate::sf2::logic::Sf2CellMark;
use crate::sf2::models::{Sf2WorkbookAnalysis, Sf2WorkbookMetadata};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const IMPORT_SCRIPT: &str = include_str!("../../scripts/sf2/import_sf2.ps1");
const EXPORT_SCRIPT: &str = include_str!("../../scripts/sf2/export_sf2.ps1");
const APPLY_SETTINGS_SCRIPT: &str = include_str!("../../scripts/sf2/apply_settings.ps1");

pub fn analyze_workbook(path: &Path) -> Result<Sf2WorkbookAnalysis> {
    let output = run_script(IMPORT_SCRIPT, &[path.to_path_buf()])?;
    serde_json::from_str(&output).map_err(|error| {
        AppError::Internal(format!("failed to parse SF2 workbook analysis: {error}"))
    })
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

    let marks_json = serde_json::to_string(marks)
        .map_err(|error| AppError::Internal(format!("failed to serialize SF2 marks: {error}")))?;
    let marks_path = write_temp_file("sf2-marks", ".json", &marks_json)?;

    let result = run_script(
        EXPORT_SCRIPT,
        &[
            workbook_path.to_path_buf(),
            marks_path.clone(),
            workbook_path.to_path_buf(),
        ],
    );
    let _ = std::fs::remove_file(&marks_path);

    result?;
    Ok(())
}

pub fn write_metadata(workbook_path: &Path, metadata: &Sf2WorkbookMetadata) -> Result<()> {
    let metadata_json = serde_json::to_string(metadata).map_err(|error| {
        AppError::Internal(format!("failed to serialize SF2 metadata: {error}"))
    })?;
    let metadata_path = write_temp_file("sf2-settings", ".json", &metadata_json)?;

    let result = run_script(
        APPLY_SETTINGS_SCRIPT,
        &[workbook_path.to_path_buf(), metadata_path.clone()],
    );
    let _ = std::fs::remove_file(&metadata_path);

    result?;
    Ok(())
}

fn run_script(script: &str, args: &[PathBuf]) -> Result<String> {
    if !cfg!(windows) {
        return Err(AppError::InvalidInput(
            "SF2 Excel automation requires Windows with Microsoft Excel installed".to_string(),
        ));
    }

    let script_path = write_temp_file("sf2-excel", ".ps1", script)?;
    let mut command = Command::new("powershell.exe");
    command
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&script_path);

    for arg in args {
        command.arg(arg);
    }

    let output = command.output().map_err(|error| {
        AppError::Internal(format!(
            "failed to start PowerShell Excel automation: {error}"
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let message = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(AppError::Internal(format!(
            "Excel automation failed: {message}"
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn write_temp_file(prefix: &str, extension: &str, contents: &str) -> Result<PathBuf> {
    let path = std::env::temp_dir().join(format!("{prefix}-{}{}", uuid::Uuid::new_v4(), extension));
    let mut file = std::fs::File::create(&path)
        .map_err(|error| AppError::Internal(format!("failed to create temp file: {error}")))?;
    file.write_all(contents.as_bytes())
        .map_err(|error| AppError::Internal(format!("failed to write temp file: {error}")))?;
    Ok(path)
}
