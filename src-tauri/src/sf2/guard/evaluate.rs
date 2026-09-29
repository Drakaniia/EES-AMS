//! Measuring one month's workbook against the database (spec §9.1, F2, F6).
//!
//! Two counts, both scoped to the same month and the same roster rows:
//!
//! * the database's - every absence it holds on this month's mapped day columns
//!   and mapped learner rows, and
//! * the workbook's - every `X` in the same block.
//!
//! Comparing a workbook count for one month against a database count for
//! another is the same class of bug this whole guard exists to prevent, so both
//! sides are derived from the same pair of mapping sets.
//!
//! The workbook side is **one bulk range read per learner row**, not one COM
//! round-trip per cell. The old per-cell read over 33 columns x 40 students is
//! ~1,320 round-trips and feels like a hang - which is the exact symptom the
//! guard is fixing, so measuring must not reintroduce it. The row's whole
//! 33-column block is handed to Excel as a single formula through
//! `Worksheet.Evaluate` and Excel flattens it in one pass.
//!
//! ## "Measured zero" and "could not measure" are different types
//!
//! [`WorkbookMarkScan`] is an enum, not a struct with a `count: usize`. That is
//! not a style preference. A count that can be zero because the read failed is
//! indistinguishable from a count that is zero because the month is empty, and
//! the guard's rule is `db_count >= workbook_count`, which a fabricated zero
//! satisfies for **any** month. It is therefore permission to erase the grid,
//! issued by a measurement that never happened.
//!
//! So a read that errors, that returns an Excel error value, or that comes back
//! the wrong width is [`WorkbookMarkScan::Failed`] - which has no accessor that
//! can produce a number, so there is no way to ask it how many `X` marks the
//! workbook holds. The only way out of it is [`SyncPermit::Unmeasured`], which
//! means: do not clear, open read-only, tell the user.

use crate::domain::error::{AppError, Result};
use crate::infrastructure::database::DbPool;
use crate::sf2::attendance::attendance_marks::{
    attendance_block_columns, attendance_scope_cells, mapped_attendance_rows,
};
use crate::sf2::guard::{
    self, CellLabels, Sf2GridCell, SyncPermit, NO_MAPPED_DATES, NO_MAPPED_LEARNERS,
    WORKBOOK_NOT_READABLE,
};
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord, Sf2TemplateRecord};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The character the attendance block is flattened with.
///
/// It cannot occur in the block - which holds `X`, day numbers and formula
/// results - so a value's position in the flattened answer is its column. A
/// separator that *could* occur would shift every value after it by one column
/// and put every `X` on a day the app cannot name.
///
/// `ignore_empty` is `FALSE` for the same reason: with it `TRUE`, `TEXTJOIN`
/// drops empty cells and the delimiter count shrinks with them.
#[cfg(target_os = "windows")]
const BLOCK_SEPARATOR: char = '|';

/// What one bulk read of the attendance block found.
///
/// `Measured { x_cells: [] }` is a real answer: the month holds no `X` in scope.
/// `Failed { .. }` is the absence of an answer. They are not the same value, and
/// they are not the same type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkbookMarkScan {
    /// The workbook was read, and here is what it holds.
    Measured {
        /// Cells holding an `X`, restricted to the mapped learner rows and
        /// mapped day columns of the report month.
        ///
        /// An `X` the app cannot place - an unmapped day column, an unmapped
        /// learner row - is deliberately *not* reported. It is also not written
        /// over by the differential clear, so it survives untouched; counting it
        /// here would instead make every future sync `Stale` forever over a mark
        /// the app has no way to reproduce or clear.
        x_cells: Vec<Sf2GridCell>,
        /// Learner rows x day columns x sheets actually inspected. Diagnostic
        /// only, and proof that the read really looked at the grid.
        cells_scanned: usize,
    },
    /// The workbook could not be read, and this is why.
    ///
    /// Holds no count, and offers no way to derive one. Every caller must
    /// resolve it to [`SyncPermit::Unmeasured`].
    Failed { reason: String },
}

impl WorkbookMarkScan {
    /// The failure, with its reason. The conservative outcome: every error path
    /// funnels here and nowhere else.
    #[must_use]
    pub fn failed(reason: impl Into<String>) -> Self {
        Self::Failed {
            reason: reason.into(),
        }
    }

    /// The `X` cells, or `None` when the workbook could not be measured.
    ///
    /// The only accessor that yields a count, and it is `Option` for the same
    /// reason [`decide`](crate::sf2::guard::decide) takes one.
    #[must_use]
    pub fn x_cells(&self) -> Option<&[Sf2GridCell]> {
        match self {
            Self::Measured { x_cells, .. } => Some(x_cells),
            Self::Failed { .. } => None,
        }
    }

    /// The reason the scan failed, when it did.
    #[must_use]
    pub fn failure_reason(&self) -> Option<&str> {
        match self {
            Self::Measured { .. } => None,
            Self::Failed { reason } => Some(reason),
        }
    }
}

/// Measure the `X` marks a workbook holds, in scope, without modifying it.
///
/// Opens **read-only** and closes without saving, so measuring can never write
/// to the file it is measuring. A workbook that could not be read is
/// [`WorkbookMarkScan::Failed`], never a count of zero.
pub fn measure_workbook_marks(
    path: &Path,
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> WorkbookMarkScan {
    let rows = mapped_attendance_rows(student_mappings.iter().map(|m| m.row_index));
    if rows.is_empty() {
        return WorkbookMarkScan::failed(guard::NO_MAPPED_LEARNERS);
    }

    let sheets: Vec<&str> = date_mappings
        .iter()
        .map(|mapping| mapping.sheet_name.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    if sheets.is_empty() {
        return WorkbookMarkScan::failed(guard::NO_MAPPED_DATES);
    }

    // The scope decides which cells are counted *and* which cells the
    // differential clear is allowed to touch. Deriving both from one place is
    // what keeps "what we measured" and "what we may clear" the same set.
    let scope: HashSet<Sf2GridCell> = attendance_scope_cells(student_mappings, date_mappings)
        .into_iter()
        .collect();

    match read_x_cells_bulk(path, &sheets, &rows) {
        Ok(x_cells) => WorkbookMarkScan::Measured {
            x_cells: x_cells
                .into_iter()
                .filter(|cell| scope.contains(cell))
                .collect(),
            cells_scanned: scope.len(),
        },
        Err(error) => {
            log::warn!(
                "SF2 workbook '{}' could not be measured, so its marks are unknown: {error}",
                path.display()
            );
            WorkbookMarkScan::failed(WORKBOOK_NOT_READABLE)
        }
    }
}

/// The permit for one month, measured against the database.
///
/// Every failure resolves to [`SyncPermit::Unmeasured`]:
///
/// * no mapped dates - a degenerate workbook analysis (spec §4 step 2),
/// * no mapped learners,
/// * the month's file missing from disk (edge case E4), or
/// * Excel unable to read the workbook (absent Excel, or the user has it open -
///   edges E5 and E6).
///
/// `Proven` is returned only when the database was read *and* the workbook was
/// read *and* the database holds every `X` the workbook shows.
#[must_use]
pub fn evaluate(
    pool: &DbPool,
    template: &Sf2TemplateRecord,
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> SyncPermit {
    if date_mappings.is_empty() {
        log::warn!(
            "refusing to clear SF2 workbook '{}': no date mappings for report month '{}'",
            template.source_path,
            template.report_month
        );
        return SyncPermit::unmeasured(NO_MAPPED_DATES);
    }
    if student_mappings.is_empty() {
        log::warn!(
            "refusing to clear SF2 workbook '{}': no mapped learners",
            template.source_path
        );
        return SyncPermit::unmeasured(NO_MAPPED_LEARNERS);
    }

    let workbook_path = std::path::PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        // Edge case E4: the row exists, the file does not. The file cannot be
        // measured, so it cannot be cleared, and there is nothing to open.
        return SyncPermit::unmeasured(guard::missing_workbook_file_message(
            &template.report_month,
        ));
    }

    let db_x_cells = match database_x_cells(pool, template, student_mappings, date_mappings) {
        Ok(cells) => cells,
        Err(error) => {
            log::warn!(
                "refusing to clear SF2 workbook '{}': could not read the database: {error}",
                template.source_path
            );
            return SyncPermit::unmeasured(WORKBOOK_NOT_READABLE);
        }
    };

    let scan = measure_workbook_marks(&workbook_path, student_mappings, date_mappings);

    permit_from_scan(
        &db_x_cells,
        &scan,
        &cell_labels(student_mappings, date_mappings),
    )
}

/// The permit for one already-taken measurement, with no Excel in sight.
///
/// Split out of [`evaluate`] so the rule that connects "the read failed" to "do
/// not clear" is one function with no IO in it, and so it can be tested
/// directly. `WorkbookMarkScan::Failed` has no count to compare, so
/// [`WorkbookMarkScan::x_cells`] yields `None` and the only permit it can reach
/// is `Unmeasured` - there is no arm of [`guard::decide`] that a failed read
/// could satisfy.
#[must_use]
pub fn permit_from_scan(
    db_x_cells: &[Sf2GridCell],
    scan: &WorkbookMarkScan,
    labels: &CellLabels,
) -> SyncPermit {
    guard::decide(db_x_cells, scan.x_cells(), labels, WORKBOOK_NOT_READABLE)
}

/// The `X` marks the database proves, in the same cell vocabulary as the scan.
fn database_x_cells(
    pool: &DbPool,
    template: &Sf2TemplateRecord,
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> Result<Vec<Sf2GridCell>> {
    use crate::sf2::attendance::attendance_marks::export_marks;

    let days = date_mappings
        .iter()
        .map(|mapping| mapping.date.clone())
        .collect::<Vec<_>>();

    // `export_marks` is the same code the writer uses, so "what the database
    // holds" and "what the write path would put in the workbook" cannot drift.
    let marks = export_marks(
        pool.clone(),
        &template.active_class_id,
        &days,
        student_mappings,
        date_mappings,
    )?;

    Ok(marks.iter().filter_map(Sf2GridCell::from_mark).collect())
}

/// Name every in-scope cell, so a `Stale` permit can list learners and days
/// rather than just a count.
fn cell_labels(
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> CellLabels {
    let names: HashMap<u32, &str> = student_mappings
        .iter()
        .filter(|mapping| mapping.row_index > 0)
        .map(|mapping| (mapping.row_index, mapping.workbook_name.as_str()))
        .collect();

    let mut labels = CellLabels::new();
    for date in date_mappings {
        for mapping in student_mappings {
            let Some(student) = names.get(&mapping.row_index) else {
                continue;
            };
            labels.insert(
                &Sf2GridCell {
                    sheet_name: date.sheet_name.clone(),
                    column_letter: date.column_letter.clone(),
                    row_index: mapping.row_index,
                },
                *student,
                date.date.clone(),
            );
        }
    }
    labels
}

// ── the bulk read ───────────────────────────────────────────────────────

/// Read every `X` in the attendance block, one formula per learner row.
///
/// An `Err` here is the whole point of the function. It is returned for every way
/// the read can fail - Excel absent, file locked, sheet missing, formula
/// refused, answer the wrong width - and it is never converted into a short
/// list. The caller turns it into [`WorkbookMarkScan::Failed`], which cannot be
/// mistaken for a month with no absences.
#[cfg(target_os = "windows")]
fn read_x_cells_bulk(path: &Path, sheets: &[&str], rows: &[u32]) -> Result<Vec<Sf2GridCell>> {
    use crate::sf2::excel::excel_com::com_session::{run_excel_task, with_workbook};

    let block = attendance_block_columns();
    // Every formula is built up front so the COM closure owns plain data and
    // needs no borrow of the caller's frame. A plain nested loop rather than an
    // iterator chain, because the chain's closures capture `block` and cannot
    // hand a borrow of it to a `FnMut`.
    let mut plan: Vec<(String, u32, String)> = Vec::with_capacity(sheets.len() * rows.len());
    for sheet in sheets.iter().copied() {
        for row in rows {
            plan.push((
                sheet.to_string(),
                *row,
                row_read_formula(sheet, *row, &block),
            ));
        }
    }

    let path = path.to_path_buf();
    run_excel_task(move || {
        let block = block.clone();
        with_workbook(&path, true, false, |excel, workbook| {
            let mut x_cells = Vec::new();
            for (sheet_name, row, formula) in &plan {
                let sheet = worksheet(workbook, sheet_name)?;
                let answer = evaluate_on_sheet(&sheet, &excel.app, formula).ok_or_else(|| {
                    AppError::Internal(format!(
                        "Excel would not read row {row} of '{sheet_name}'; its X marks cannot be \
                         counted, so the month cannot be cleared"
                    ))
                })?;
                x_cells.extend(x_cells_in_row(sheet_name, *row, &answer, &block)?);
            }
            Ok(x_cells)
        })
    })
}

/// Excel automation is unavailable off Windows, so no workbook can be proven to
/// hold absences. A `Failed` here, not an empty list.
#[cfg(not(target_os = "windows"))]
fn read_x_cells_bulk(_path: &Path, _sheets: &[&str], _rows: &[u32]) -> Result<Vec<Sf2GridCell>> {
    Err(AppError::Internal(WORKBOOK_NOT_READABLE.to_string()))
}

/// One worksheet by name.
///
/// A missing sheet is an error, not an empty read. A month whose sheet has been
/// renamed - which is what the pre-split calendar cycle does, leaving
/// `__SF2_HIDDEN_n` - cannot be measured, and saying so is the only safe answer.
#[cfg(target_os = "windows")]
fn worksheet(
    workbook: &crate::sf2::excel::excel_com::com_session::ComObject,
    sheet_name: &str,
) -> Result<crate::sf2::excel::excel_com::com_session::ComObject> {
    use crate::sf2::excel::excel_com::com_session::ComVariant;

    workbook
        .get_object("Worksheets")?
        .get_object_with_args("Item", vec![ComVariant::bstr(sheet_name)])
}

/// Evaluate one formula **on the worksheet being read**.
///
/// Two deliberate choices, both measured on this project's Excel:
///
/// * **Worksheet-scoped, not application-scoped.** `Application.Evaluate`
///   behaves like typing the formula into the *active* cell. On a workbook whose
///   active sheet is a different one, or whose first tab is hidden - which is
///   what the pre-split cycle leaves behind - it refuses the evaluation and
///   returns an error variant. Eleven of a file's twelve month sheets are hidden
///   in exactly that state. `Worksheet.Evaluate` evaluates in the target sheet's
///   own context, so a hidden sheet reads the same as a visible one. The
///   application is kept only as a fallback for an Excel too old to have it.
/// * **Fallible.** `Application.Evaluate`/`Worksheet.Evaluate` do not throw when
///   Excel cannot evaluate a formula; they return a `VT_ERROR` variant.
///   [`ComVariant::to_string_value`](crate::sf2::excel::excel_com::com_session::ComVariant::to_string_value)
///   keeps that an error, and this collapses either failure to `None` so the
///   caller's error message names the row and the sheet.
#[cfg(target_os = "windows")]
fn evaluate_on_sheet(
    sheet: &crate::sf2::excel::excel_com::com_session::ComObject,
    app: &crate::sf2::excel::excel_com::com_session::ComObject,
    formula: &str,
) -> Option<String> {
    use crate::sf2::excel::excel_com::com_session::ComVariant;

    sheet
        .method("Evaluate", vec![ComVariant::bstr(formula)])
        .or_else(|_| app.method("Evaluate", vec![ComVariant::bstr(formula)]))
        .ok()
        .and_then(|answer| answer.to_string_value().ok())
}

/// The formula that flattens one learner row's whole attendance block into a
/// single string, one token per column.
///
/// `TEXTJOIN("<sep>", FALSE, 'SHEET'!F9:AL9)` - one COM round-trip per row
/// rather than one per cell, which is what keeps a 40-learner month from feeling
/// like a hang.
///
/// ## Why not the bit-mask form this replaced
///
/// It used to be `TEXTJOIN("",TRUE,('SHEET'!F9="X")*1, ... )` - 33 boolean
/// products, concatenated into a 33-character bit mask. That formula is what
/// `Application.Evaluate` **refuses**: measured on this project's Excel it
/// answers an error value, at 33 terms and at 8 terms alike, application-scoped
/// and worksheet-scoped alike. The failing ingredient is the `(ref="X")*1`
/// comparison term, not the term count - `COUNTIF` and a plain `TEXTJOIN` over
/// the same range both work.
///
/// Because the answer was an error variant and the old reader rendered error
/// variants as the empty string, every row came back with an empty mask, every
/// month came back with **zero** `X` cells, and zero satisfies
/// `db_count >= workbook_count` for every month. The guard was reporting
/// `Proven` - permission to erase the grid - on a measurement that had never
/// happened. This formula works; [`x_cells_in_row`] is what stops the next COM
/// quirk from doing the same thing again.
#[cfg(target_os = "windows")]
fn row_read_formula(sheet_name: &str, row: u32, block: &[String]) -> String {
    let sheet = quote_sheet_name(sheet_name);
    let first = &block[0];
    let last = &block[block.len() - 1];
    format!("TEXTJOIN(\"{BLOCK_SEPARATOR}\",FALSE,{sheet}!{first}{row}:{last}{row})")
}

/// The `X` cells one flattened row holds, or an error if the row did not come
/// back the right shape.
///
/// **This validation is the fix.** The read used to take whatever string arrived
/// and count its `'1'` characters, so a refused formula - delivered as the empty
/// string - became "no `X` marks", which became `Proven`, which became a cleared
/// grid. A flattened row is only meaningful if it has exactly one token per
/// column: too few means the answer is truncated or empty, too many means the
/// layout is not the DepEd SF2 grid this build expects. Either way the row is
/// **not** a count of zero, and saying otherwise is the bug.
#[cfg(target_os = "windows")]
fn x_cells_in_row(
    sheet_name: &str,
    row: u32,
    answer: &str,
    block: &[String],
) -> Result<Vec<Sf2GridCell>> {
    let separator = BLOCK_SEPARATOR.to_string();
    let tokens: Vec<&str> = answer.split(separator.as_str()).collect();
    if tokens.len() != block.len() {
        return Err(AppError::Internal(format!(
            "read {} of {} cells from '{sheet_name}' row {row}; the month cannot be measured, so \
             nothing will be cleared",
            tokens.len(),
            block.len()
        )));
    }

    Ok(tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.trim().eq_ignore_ascii_case(ABSENT_MARK))
        .map(|(offset, _)| Sf2GridCell {
            sheet_name: sheet_name.to_string(),
            column_letter: block[offset].clone(),
            row_index: row,
        })
        .collect())
}

/// The mark the SF2 workbook uses for an absence.
///
/// Compared case-insensitively, because the same reader is used to check a
/// workbook a teacher has typed into by hand, and a lowercase `x` is the same
/// mark.
#[cfg(target_os = "windows")]
const ABSENT_MARK: &str = "X";

/// Quote a sheet name for a formula, doubling embedded single quotes the way
/// Excel's own formula syntax requires.
#[cfg(target_os = "windows")]
fn quote_sheet_name(sheet_name: &str) -> String {
    format!("'{}'", sheet_name.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_cell(sheet: &str, column: &str, row: u32) -> Sf2GridCell {
        Sf2GridCell {
            sheet_name: sheet.to_string(),
            column_letter: column.to_string(),
            row_index: row,
        }
    }

    #[test]
    fn a_mark_address_splits_into_its_column_and_row() {
        let mark = crate::sf2::logic::Sf2CellMark {
            sheet_name: "JULY 2026".to_string(),
            cell_address: "AL47".to_string(),
            value: "X".to_string(),
        };

        let cell = Sf2GridCell::from_mark(&mark).expect("AL47 splits");

        assert_eq!(cell, grid_cell("JULY 2026", "AL", 47));
    }

    #[test]
    fn an_address_with_no_row_is_not_a_cell() {
        let mark = crate::sf2::logic::Sf2CellMark {
            sheet_name: "JULY 2026".to_string(),
            cell_address: "AL".to_string(),
            value: "X".to_string(),
        };

        assert_eq!(Sf2GridCell::from_mark(&mark), None);
    }

    #[test]
    fn the_attendance_block_is_columns_f_through_al() {
        let block = attendance_block_columns();

        assert_eq!(block.first().map(String::as_str), Some("F"));
        assert_eq!(block.last().map(String::as_str), Some("AL"));
        assert_eq!(block.len(), 33);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn the_row_read_formula_flattens_the_range_not_a_boolean_product() {
        // The regression test for the refused formula. The old shape - 33
        // `(ref="X")*1` products inside a `TEXTJOIN` - is an error value on this
        // project's Excel at any term count, and the empty mask that followed
        // was read as "this month has no X marks".
        let block = attendance_block_columns();
        let formula = row_read_formula("SEPTEMBER 2026", 9, &block);

        assert_eq!(formula, "TEXTJOIN(\"|\",FALSE,'SEPTEMBER 2026'!F9:AL9)");
        assert!(
            !formula.contains("=\"X\""),
            "the comparison term is the part Excel refuses: {formula}"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn a_sheet_name_with_a_quote_is_escaped_for_the_formula() {
        let block = attendance_block_columns();
        let formula = row_read_formula("ROSTER'S COPY", 9, &block);

        assert!(
            formula.contains("'ROSTER''S COPY'!"),
            "an embedded quote must be doubled: {formula}"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn a_whole_row_formula_fits_inside_the_formula_length_ceiling() {
        let block = attendance_block_columns();
        let formula = row_read_formula("SEPTEMBER 2026", 47, &block);

        assert!(
            formula.len() < 8_192,
            "the whole 33-column block must resolve in one formula, got {} chars",
            formula.len()
        );
    }

    // ── the laundering itself ────────────────────────────────────────────
    //
    // These four are the tests that would have caught the defect. They do not
    // need Excel: they exercise what this module does with the string Excel
    // hands back, which is where the error became a zero.

    #[cfg(target_os = "windows")]
    #[test]
    fn a_row_whose_x_marks_are_known_is_read_back_at_those_columns() {
        let block = attendance_block_columns();
        // 33 tokens, X at offsets 0, 2 and 32.
        let mut tokens = vec![""; block.len()];
        tokens[0] = "X";
        tokens[2] = "X";
        tokens[32] = "X";
        let answer = tokens.join("|");

        let cells = x_cells_in_row("SEPTEMBER 2026", 47, &answer, &block).expect("row reads");

        assert_eq!(
            cells,
            vec![
                grid_cell("SEPTEMBER 2026", "F", 47),
                grid_cell("SEPTEMBER 2026", "H", 47),
                grid_cell("SEPTEMBER 2026", "AL", 47),
            ]
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn an_empty_row_is_zero_marks_not_a_failure() {
        // A genuine zero has to survive the validation, or the fix would have
        // traded a false zero for a false refusal and the guard would never be
        // able to clear anything again.
        let block = attendance_block_columns();
        let answer = vec![""; block.len()].join("|");

        let cells = x_cells_in_row("SEPTEMBER 2026", 47, &answer, &block).expect("row reads");

        assert!(cells.is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn a_refused_formula_is_a_failure_and_never_zero_marks() {
        // The exact defect. A formula Excel refuses comes back as an error
        // variant, which the old reader rendered as "". "" is not 33 tokens, so
        // it must be an error - and an error must never be counted as a month
        // with no absences, because a month with no absences is permission to
        // clear.
        let block = attendance_block_columns();

        for answer in ["", "#VALUE!", "#REF!", "#N/A"] {
            assert!(
                x_cells_in_row("SEPTEMBER 2026", 47, answer, &block).is_err(),
                "{answer:?} is not a 33-token row and must not be read as zero marks"
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn a_row_read_that_came_back_the_wrong_width_is_a_failure() {
        // Too few tokens means truncated, too many means this is not the grid
        // this build expects. Neither is a count of zero.
        let block = attendance_block_columns();
        let short = vec![""; block.len() - 1].join("|");
        let long = vec!["X"; block.len() + 1].join("|");

        assert!(x_cells_in_row("SEPTEMBER 2026", 47, &short, &block).is_err());
        assert!(x_cells_in_row("SEPTEMBER 2026", 47, &long, &block).is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn a_lowercase_x_is_the_same_absence_mark() {
        // The workbook is typed into by hand.
        let block = attendance_block_columns();
        let mut tokens = vec![""; block.len()];
        tokens[4] = "x";
        let answer = tokens.join("|");

        let cells = x_cells_in_row("SEPTEMBER 2026", 47, &answer, &block).expect("row reads");

        assert_eq!(cells, vec![grid_cell("SEPTEMBER 2026", "J", 47)]);
    }

    // ── the type-level distinction ───────────────────────────────────────

    #[test]
    fn a_failed_scan_has_no_count_to_compare() {
        let scan = WorkbookMarkScan::failed(WORKBOOK_NOT_READABLE);

        assert_eq!(scan.x_cells(), None);
        assert_eq!(scan.failure_reason(), Some(WORKBOOK_NOT_READABLE));
    }

    #[test]
    fn a_measured_zero_is_distinguishable_from_a_failure() {
        let measured = WorkbookMarkScan::Measured {
            x_cells: Vec::new(),
            cells_scanned: 40 * 33,
        };

        assert_eq!(measured.x_cells(), Some([].as_slice()));
        assert_eq!(measured.failure_reason(), None);
        assert_ne!(
            WorkbookMarkScan::failed("boom").x_cells(),
            measured.x_cells(),
            "measured zero and could not measure must be different values"
        );
    }

    #[test]
    fn a_scan_with_no_mapped_learners_fails_rather_than_reading_nothing() {
        let scan = measure_workbook_marks(
            std::path::Path::new("does-not-matter.xls"),
            &[],
            &[Sf2DateMappingRecord {
                template_id: "tpl".to_string(),
                sheet_name: "SEPTEMBER 2026".to_string(),
                date: "2026-09-01".to_string(),
                column_letter: "F".to_string(),
                column_index: 6,
            }],
        );

        assert_eq!(scan.failure_reason(), Some(guard::NO_MAPPED_LEARNERS));
    }

    #[test]
    fn a_scan_with_no_mapped_dates_fails_rather_than_reading_nothing() {
        let scan = measure_workbook_marks(
            std::path::Path::new("does-not-matter.xls"),
            &[Sf2StudentMappingRecord {
                template_id: "tpl".to_string(),
                student_id: "s1".to_string(),
                workbook_name: "LEARNER".to_string(),
                normalized_name: "LEARNER".to_string(),
                row_index: 8,
                gender_block: None,
            }],
            &[],
        );

        assert_eq!(scan.failure_reason(), Some(guard::NO_MAPPED_DATES));
    }

    #[test]
    fn a_workbook_that_cannot_be_opened_is_a_failure_never_a_count() {
        // The file is not there, so Excel can never be reached. Before the fix
        // this path could only produce a count; now it can only produce a
        // failure.
        let scan = measure_workbook_marks(
            std::path::Path::new("this-file-does-not-exist.xls"),
            &[Sf2StudentMappingRecord {
                template_id: "tpl".to_string(),
                student_id: "s1".to_string(),
                workbook_name: "LEARNER".to_string(),
                normalized_name: "LEARNER".to_string(),
                row_index: 8,
                gender_block: None,
            }],
            &[Sf2DateMappingRecord {
                template_id: "tpl".to_string(),
                sheet_name: "SEPTEMBER 2026".to_string(),
                date: "2026-09-01".to_string(),
                column_letter: "F".to_string(),
                column_index: 6,
            }],
        );

        assert!(
            matches!(scan, WorkbookMarkScan::Failed { .. }),
            "an unreadable workbook must never be reported as a measurement"
        );
        assert_eq!(scan.x_cells(), None);
    }
}

#[cfg(test)]
#[path = "__tests__/measure_tests.rs"]
mod measure_tests;
