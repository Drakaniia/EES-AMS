use crate::domain::error::{AppError, Result};
use crate::sf2::calendar::{sf2_month_name, sf2_month_number};
use crate::sf2::models::{Sf2TemplateRecord, Sf2WorkbookAnalysis};
use crate::sf2::naming::sanitize_file_part;
use chrono::{Datelike, Local};
use std::hash::Hasher;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

pub(super) const BUNDLED_TEMPLATE_BYTES: &[u8] =
    include_bytes!("../../resources/sf2/TEMPLATE_AUTOMATED_SF2.xls");

pub(super) fn copy_workbook_to_app_data<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    source_path: &Path,
    template_id: &str,
    analysis: &Sf2WorkbookAnalysis,
) -> Result<PathBuf> {
    let dir = sf2_workbook_dir(app)?;
    let template_prefix = template_id.chars().take(8).collect::<String>();
    let file_name = format!(
        "SF2-{}-{}-{}.xls",
        sanitized_or(&analysis.grade_level, "GRADE"),
        sanitized_or(&analysis.section, "SECTION"),
        template_prefix
    );
    let working_copy_path = dir.join(file_name);

    if source_path != working_copy_path {
        std::fs::copy(source_path, &working_copy_path).map_err(|error| {
            AppError::Internal(format!(
                "failed to copy SF2 workbook into app data: {error}"
            ))
        })?;
    }

    Ok(working_copy_path)
}

pub(super) fn write_bundled_template_to_dir(
    dir: &Path,
    template_id: &str,
    grade_level: &str,
    section: &str,
) -> Result<PathBuf> {
    let template_prefix = template_id.chars().take(8).collect::<String>();
    let file_name = format!(
        "SF2-{}-{}-{}.xls",
        sanitized_or(grade_level, "GRADE"),
        sanitized_or(section, "SECTION"),
        template_prefix
    );
    let working_copy_path = dir.join(file_name);
    std::fs::write(&working_copy_path, BUNDLED_TEMPLATE_BYTES).map_err(|error| {
        AppError::Internal(format!(
            "failed to create SF2 workbook from bundled template: {error}"
        ))
    })?;
    Ok(working_copy_path)
}

pub(super) fn sf2_workbook_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Internal(format!("failed to get app data directory: {error}")))?
        .join("sf2-workbooks");
    std::fs::create_dir_all(&dir).map_err(|error| {
        AppError::Internal(format!("failed to create SF2 workbook directory: {error}"))
    })?;
    Ok(dir)
}

fn sanitized_or(value: &str, fallback: &str) -> String {
    let sanitized = sanitize_file_part(value);
    if sanitized.is_empty() {
        fallback.to_string()
    } else {
        sanitized
    }
}

pub(super) fn pick_workbook_path(app: &tauri::AppHandle) -> Result<PathBuf> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Excel 97-2003 Workbook", &["xls"])
        .pick_file(move |result| {
            let _ = tx.send(result);
        });

    dialog_path(rx.recv().map_err(|error| {
        AppError::Internal(format!("failed to receive workbook path: {error}"))
    })?)?
    .ok_or_else(|| AppError::InvalidInput("Import cancelled".to_string()))
}

pub(super) fn save_workbook_path(
    app: &tauri::AppHandle,
    template: &Sf2TemplateRecord,
) -> Result<PathBuf> {
    let file_name = export_workbook_file_name(template, Local::now().month());
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Excel 97-2003 Workbook", &["xls"])
        .set_file_name(file_name)
        .save_file(move |result| {
            let _ = tx.send(result);
        });

    dialog_path(
        rx.recv().map_err(|error| {
            AppError::Internal(format!("failed to receive output path: {error}"))
        })?,
    )?
    .ok_or_else(|| AppError::InvalidInput("Export cancelled".to_string()))
}

pub(super) fn export_workbook_file_name(
    template: &Sf2TemplateRecord,
    current_month: u32,
) -> String {
    let month = sf2_export_month_file_part(&template.report_month, current_month);
    format!(
        "SF2-{}-{}-{}-generated.xls",
        sanitized_or(&template.grade_level, "GRADE"),
        sanitized_or(&template.section, "SECTION"),
        month
    )
}

fn sf2_export_month_file_part(report_month: &str, current_month: u32) -> &'static str {
    sf2_month_number(report_month)
        .or(Some(current_month))
        .map(sf2_month_name)
        .filter(|month| !month.is_empty())
        .unwrap_or("MONTH")
}

fn dialog_path(path: Option<tauri_plugin_dialog::FilePath>) -> Result<Option<PathBuf>> {
    match path {
        Some(tauri_plugin_dialog::FilePath::Path(path)) => Ok(Some(path)),
        Some(tauri_plugin_dialog::FilePath::Url(url)) => Err(AppError::InvalidInput(format!(
            "URL file paths are not supported: {url}"
        ))),
        None => Ok(None),
    }
}

#[cfg(target_os = "windows")]
pub(super) fn open_path_in_default_app(path: &Path) -> Result<()> {
    let status = Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(path)
        .status()
        .map_err(|error| AppError::Internal(format!("failed to open SF2 workbook: {error}")))?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::Internal(format!(
            "failed to open SF2 workbook: default app returned {status}"
        )))
    }
}

#[cfg(target_os = "macos")]
pub(super) fn open_path_in_default_app(path: &Path) -> Result<()> {
    Command::new("open")
        .arg(path)
        .spawn()
        .map_err(|error| AppError::Internal(format!("failed to open SF2 workbook: {error}")))?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(super) fn open_path_in_default_app(path: &Path) -> Result<()> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|error| AppError::Internal(format!("failed to open SF2 workbook: {error}")))?;
    Ok(())
}

pub(super) fn write_temp_binary_file(
    prefix: &str,
    extension: &str,
    contents: &[u8],
) -> Result<PathBuf> {
    let path = std::env::temp_dir().join(format!("{prefix}-{}{}", uuid::Uuid::new_v4(), extension));
    let mut file = std::fs::File::create(&path)
        .map_err(|error| AppError::Internal(format!("failed to create temp file: {error}")))?;
    file.write_all(contents)
        .map_err(|error| AppError::Internal(format!("failed to write temp file: {error}")))?;
    Ok(path)
}

pub(super) fn file_hash(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)
        .map_err(|error| AppError::Internal(format!("failed to read SF2 workbook: {error}")))?;
    Ok(hash_bytes(&bytes))
}

pub(super) fn layout_fingerprint(analysis: &Sf2WorkbookAnalysis) -> String {
    let mut bytes = Vec::new();
    for sheet in &analysis.sheets {
        bytes.extend_from_slice(sheet.name.as_bytes());
        bytes.extend_from_slice(sheet.used_range.as_bytes());
    }
    for learner in &analysis.learners {
        bytes.extend_from_slice(learner.name.as_bytes());
        bytes.extend_from_slice(&learner.row_index.to_le_bytes());
    }
    for date in &analysis.dates {
        bytes.extend_from_slice(date.date.as_bytes());
        bytes.extend_from_slice(date.sheet_name.as_bytes());
        bytes.extend_from_slice(date.column_letter.as_bytes());
    }
    hash_bytes(&bytes)
}

pub(super) fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Fnva64::default();
    hasher.write(bytes);
    format!("{:016x}", hasher.finish())
}

#[derive(Default)]
struct Fnva64(u64);

impl Hasher for Fnva64 {
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = if self.0 == 0 {
            0xcbf29ce484222325
        } else {
            self.0
        };
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.0 = hash;
    }

    fn finish(&self) -> u64 {
        if self.0 == 0 {
            0xcbf29ce484222325
        } else {
            self.0
        }
    }
}
