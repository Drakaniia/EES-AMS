use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

/// The mark the SF2 workbook uses for an absence (`sf2/logic.rs:9`).
const ABSENT_MARK: &str = "X";

/// The row block a day column is counted over when the roster is unknown.
/// Covers the learner area of the bundled SF2 template.
const FALLBACK_FIRST_ROW: u32 = 1;
const FALLBACK_LAST_ROW: u32 = 200;

/// Count the `"X"` cells a workbook holds, without modifying the file.
///
/// This is a bulk range read: the whole day-column block for a sheet is handed
/// to Excel as one `COUNTIF` formula through `Application.Evaluate`, and Excel
/// counts the range itself. That is a single COM round-trip per sheet instead
/// of one per cell — ~1 call per workbook rather than ~1240 for a 40-student
/// month — and it reads no more of the file than the count requires.
///
/// The range comes from [`crate::sf2::excel::analyze_workbook`], the same
/// analysis the destructive-sync guard uses, so the count covers exactly the
/// cells the app considers attendance marks.
///
/// The workbook is opened read-only and closed without saving, so counting
/// never writes to the file it is measuring.
///
/// Returns `Ok(0)` for a workbook with no day columns. An error means Excel
/// could not read the file (Excel missing, file locked, unexpected layout) —
/// the caller decides how to treat that; the backup treats it as "unknown",
/// never as "zero absences".
#[cfg(target_os = "windows")]
pub fn count_x_marks(path: &Path) -> Result<i64> {
    use crate::sf2::excel::excel_com::com_session::{run_excel_task, with_workbook, ComVariant};

    let analysis = crate::sf2::excel::analyze_workbook(path)?;
    let formulas = countif_formulas(&analysis);
    if formulas.is_empty() {
        return Ok(0);
    }

    let path = path.to_path_buf();
    run_excel_task(move || {
        with_workbook(&path, true, false, |excel, _workbook| {
            let mut total = 0_i64;
            for formula in &formulas {
                let evaluated = excel
                    .app
                    .method("Evaluate", vec![ComVariant::bstr(formula.as_str())])?;
                total += i64::from(evaluated.to_i32()?);
            }
            Ok(total)
        })
    })
    .map_err(Into::into)
}

/// Non-Windows placeholder — Excel automation is unavailable, so no workbook
/// can be proven to hold absences.
#[cfg(not(target_os = "windows"))]
pub fn count_x_marks(_path: &Path) -> Result<i64> {
    Ok(0)
}

// ── Formula Building ──────────────────────────────────────────────────

/// One `COUNTIF` formula per sheet, covering the widest contiguous day-column
/// block over the learner rows.
#[cfg(target_os = "windows")]
fn countif_formulas(analysis: &crate::sf2::models::Sf2WorkbookAnalysis) -> Vec<String> {
    let (first_row, last_row) = learner_row_span(analysis);

    // group_by keeps the day's columns in workbook order, so the first and last
    // of each group are the true edges of the block.
    let mut grouped: BTreeMap<&str, Vec<&crate::sf2::models::Sf2WorkbookDate>> = BTreeMap::new();
    for date in &analysis.dates {
        if date.column_letter.trim().is_empty() || date.sheet_name.trim().is_empty() {
            continue;
        }
        grouped
            .entry(date.sheet_name.as_str())
            .or_default()
            .push(date);
    }

    grouped
        .into_iter()
        .filter_map(|(sheet_name, dates)| {
            let first_column = dates.first()?.column_letter.trim();
            let last_column = dates.last()?.column_letter.trim();
            if first_column.is_empty() || last_column.is_empty() {
                return None;
            }
            Some(format!(
                "COUNTIF({}!{first_column}{first_row}:{last_column}{last_row},\"{ABSENT_MARK}\")",
                quote_sheet_name(sheet_name),
            ))
        })
        .collect()
}

/// The learner row span to count over, widened to the whole block so an X mark
/// in a gap between roster entries is not missed.
#[cfg(target_os = "windows")]
fn learner_row_span(analysis: &crate::sf2::models::Sf2WorkbookAnalysis) -> (u32, u32) {
    let Some(first) = analysis.learners.iter().map(|l| l.row_index).min() else {
        return (FALLBACK_FIRST_ROW, FALLBACK_LAST_ROW);
    };
    let Some(last) = analysis.learners.iter().map(|l| l.row_index).max() else {
        return (FALLBACK_FIRST_ROW, FALLBACK_LAST_ROW);
    };
    if first == 0 || last < first {
        return (FALLBACK_FIRST_ROW, FALLBACK_LAST_ROW);
    }
    (first, last)
}

/// Quote a sheet name for use inside a formula, doubling any embedded single
/// quote the way Excel's own formula syntax requires.
#[cfg(target_os = "windows")]
fn quote_sheet_name(sheet_name: &str) -> String {
    format!("'{}'", sheet_name.replace('\'', "''"))
}
