use crate::domain::error::{AppError, Result};
use crate::sf2::calendar::{sf2_month_name, sf2_month_number};
use crate::sf2::models::{Sf2TemplateRecord, Sf2WorkbookAnalysis};
use crate::sf2::month::first_school_day::report_year_for_school_month;
use crate::sf2::month::SF2_SCHOOL_YEAR_MONTHS;
use crate::sf2::naming::sanitize_file_part;
use chrono::{Datelike, Local};
use std::hash::Hasher;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

pub(super) const BUNDLED_TEMPLATE_BYTES: &[u8] =
    include_bytes!("../../resources/sf2/TEMPLATE_AUTOMATED_SF2.xls");

// ── The one workbook ────────────────────────────────────────────────────────
// The foundation the twelve month worksheets and the merge job are built on.
//
// The workbook is ONE file per class, holding twelve visible month worksheets
// (spec section 0 A1). The month-path helpers are still here because
// `crate::sf2::month_preview` and `crate::sf2::month::merge` both need to ask
// "where would this month's worksheet live" and "is there an old per-month file
// left over" - but a month no longer *has* a file of its own. The two `_legacy`
// helpers are called by `crate::sf2::month::merge`, which copies the pre-merge
// workbook into that folder and then reads from it on every re-run.

/// Folder inside `sf2-workbooks/` that keeps the pre-split per-class workbook.
///
/// The file is copied here by the merge and is never modified or deleted: it
/// stays the last authoritative copy of the original workbook, marks and all.
/// Backup snapshots the whole `sf2-workbooks` tree recursively, so this folder is
/// included without any special case.
pub(super) const LEGACY_WORKBOOK_DIR: &str = "_legacy";

/// The one workbook for a class: `SF2-GRADE-3-MATAPAT-3b635890.xls`.
///
/// The same name the pre-split per-class file had, which is the point: the file
/// on the user's disk today already has this name, and the merge rebuilds *that*
/// file into the twelve-sheet workbook rather than writing a new one beside it.
/// The pre-merge content is preserved first, in `_legacy/`, and the merge refuses
/// to proceed unless a byte-identical copy is there.
pub(super) fn single_workbook_file_name(
    template_id: &str,
    grade_level: &str,
    section: &str,
) -> String {
    legacy_workbook_file_name(template_id, grade_level, section)
}

/// Where the one workbook lives.
pub(super) fn single_workbook_path(
    workbook_dir: &Path,
    template_id: &str,
    grade_level: &str,
    section: &str,
) -> PathBuf {
    workbook_dir.join(single_workbook_file_name(template_id, grade_level, section))
}

pub(super) fn write_bundled_template_to_dir(
    dir: &Path,
    template_id: &str,
    grade_level: &str,
    section: &str,
) -> Result<PathBuf> {
    let file_name = legacy_workbook_file_name(template_id, grade_level, section);
    let working_copy_path = dir.join(file_name);
    write_bundled_template_to(&working_copy_path)?;
    Ok(working_copy_path)
}

/// The per-month files live directly in `sf2-workbooks/` - twelve of them, one
/// worksheet each - so the month directory *is* the workbook directory.
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

/// Where the pre-split per-class workbook is kept once the split has run.
pub(super) fn sf2_legacy_workbook_dir(workbook_dir: &Path) -> PathBuf {
    workbook_dir.join(LEGACY_WORKBOOK_DIR)
}

/// The one worksheet a month lives on inside the one workbook: `SEPTEMBER 2026`.
///
/// The same string [`crate::sf2::month::workbook_sheets::month_sheet_name`]
/// produces and the same string a `sf2_month_date_mappings.sheet_name` holds, so
/// a write addressed by a stored column finds the worksheet it was recorded
/// against.
pub(super) fn month_workbook_sheet_name(report_month: &str, report_year: i32) -> String {
    format!("{} {report_year}", canonical_month_name(report_month))
}

/// The file name of a **retired** per-month workbook: `SF2-SEPTEMBER-2026.xls`.
///
/// Under spec section 0 A1 a month has no file of its own. These helpers survive
/// for two honest reasons and no others:
///
/// 1. `crate::sf2::month::merge` looks for such a file so it can fold anything
///    unique in it into the single workbook - and then leaves it alone.
/// 2. `crate::sf2::month_preview` still reports whether one is on disk, so a
///    teacher whose install has one is told about it rather than having it vanish
///    from the conversation.
///
/// Nothing creates one any more, and nothing deletes one.
pub(super) fn month_workbook_file_name(report_month: &str, report_year: i32) -> String {
    format!(
        "SF2-{}-{report_year}.xls",
        canonical_month_name(report_month)
    )
}

/// Where a retired per-month workbook would be. May not exist; never created here.
pub(super) fn month_workbook_path(
    workbook_dir: &Path,
    report_month: &str,
    report_year: i32,
) -> PathBuf {
    workbook_dir.join(month_workbook_file_name(report_month, report_year))
}

/// The twelve `(month name, calendar year)` pairs of a school year, in the
/// order the school year runs them: SEPTEMBER -> AUGUST.
///
/// `school_year` is a label such as `2026-2027`; `fallback_year` is used only
/// when it holds no four-digit year.
///
/// Ordered by `(report_year, month)`, not by month number: the months are
/// 1..12 but the year wraps at September, so a plain month order would list
/// January first and read as a bug to the teacher.
pub(super) fn school_year_month_files(school_year: &str, fallback_year: i32) -> Vec<(String, i32)> {
    let mut months = SF2_SCHOOL_YEAR_MONTHS
        .iter()
        .map(|month| {
            let report_year = report_year_for_school_month(school_year, *month, fallback_year);
            (report_year, *month, sf2_month_name(*month))
        })
        .collect::<Vec<_>>();
    months.sort_by_key(|(report_year, month, _)| (*report_year, *month));
    months
        .into_iter()
        .map(|(report_year, _, name)| (name.to_string(), report_year))
        .collect()
}

/// The pre-split per-class file name: `SF2-1-A-1a2b3c4d.xls`.
///
/// Still resolved after the split, because the split job and any pre-split
/// install have to find the original workbook.
pub(super) fn legacy_workbook_file_name(
    template_id: &str,
    grade_level: &str,
    section: &str,
) -> String {
    let template_prefix = template_id.chars().take(8).collect::<String>();
    format!(
        "SF2-{}-{}-{}.xls",
        sanitized_or(grade_level, "GRADE"),
        sanitized_or(section, "SECTION"),
        template_prefix
    )
}

/// Write a fresh copy of the bundled template to `path`.
///
/// A month worksheet starts as a copy of the template and then loses everything
/// the template shipped in it - its sample roster and its 36 sample `X` marks -
/// before the class's own roster and absences are written. See
/// [`crate::sf2::month::workbook_sheets`] for why that emptying is structural
/// rather than a matter of care.
pub(super) fn write_bundled_template_to(path: &Path) -> Result<()> {
    std::fs::write(path, BUNDLED_TEMPLATE_BYTES).map_err(|error| {
        AppError::Internal(format!(
            "failed to create SF2 workbook from bundled template: {error}"
        ))
    })?;
    Ok(())
}

/// Create a **retired** per-month file from the bundled template.
///
/// Kept only so a test can put one on disk: nothing in the app calls it any more,
/// because a month has no file of its own under spec section 0 A1. The merge job
/// folds such a file in and leaves it alone rather than rewriting or removing it.
#[cfg(test)]
pub(super) fn write_month_template_to_dir(
    workbook_dir: &Path,
    report_month: &str,
    report_year: i32,
) -> Result<PathBuf> {
    let path = month_workbook_path(workbook_dir, report_month, report_year);
    write_bundled_template_to(&path)?;
    Ok(path)
}

/// Uppercase, canonical month name. `Sept.`, `sept` and `SEPTEMBER` all become
/// `SEPTEMBER`; anything unrecognised is uppercased as typed, and an empty month
/// keeps the same `MONTH` fallback the export path has always used.
fn canonical_month_name(report_month: &str) -> String {
    let canonical = sf2_month_number(report_month)
        .map(sf2_month_name)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| report_month.trim().to_ascii_uppercase());
    if canonical.is_empty() {
        "MONTH".to_string()
    } else {
        canonical
    }
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

/// Open a workbook path in the OS default app (Excel for .xls).
/// Uses the `open` crate (ShellExecuteW on Windows, `open`/`xdg-open`
/// elsewhere) so no console window flashes on Windows.
pub(super) fn open_path_in_default_app(path: &Path) -> Result<()> {
    open::that(path)
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

#[cfg(test)]
#[path = "month/__tests__/workbook_files_tests.rs"]
mod tests;
