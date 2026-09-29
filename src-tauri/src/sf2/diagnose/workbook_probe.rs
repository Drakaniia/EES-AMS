//! The one read-only pass over a workbook that the diagnostic needs: what
//! worksheets exist, what day grid each one carries, which cells hold `X`, and
//! who each learner row is.
//!
//! ## Read-only, and structurally so
//!
//! [`with_workbook`] is called with `read_only = true` and `save_on_close =
//! false`, and nothing in this file reaches a `put_*` or a writing `method`. The
//! only Excel calls below are `get_*`, `Range`, and `Application.Evaluate`, so
//! there is no branch that *can* write.
//!
//! Every call goes through [`run_excel_task`], the single place the process-wide
//! gate in [`crate::sf2::excel::excel_lock`] is taken. A diagnostic that opened
//! its own COM session outside that gate would be the concurrent session the
//! gate exists to prevent.
//!
//! ## Reading a range, and what `Application.Evaluate` will not do
//!
//! `Range.Value` would be the obvious way to read a block of cells, and it is
//! unavailable: the COM layer here hands back a `VARIANT`, and a range read
//! arrives as a `SAFEARRAY` this module cannot marshal. So a range is read by
//! asking Excel to *flatten* it into one string with a formula, which is the
//! technique the rest of the SF2 module uses.
//!
//! Measured on this project's Excel, `Application.Evaluate` will not evaluate a
//! `TEXTJOIN` over more than a handful of rows of a 33-column block - a
//! four-row read returns 132 tokens, a five-row read returns an error variant
//! and no text at all. The limit is not the result length and not the cell
//! count, and it does not have to be understood to be worked around: the reader
//! tries the whole band in one call, falls back to one call per row, and
//! **validates the token count either way**. A read that came back short is an
//! error, never an empty grid - the failure mode this module must not have is a
//! worksheet that reads as "no marks" because Excel refused to flatten it.
//!
//! ## The separator
//!
//! Cells are joined with a character that cannot occur in the data, because a
//! separator that *can* occur shifts every cell after it by a token and the grid
//! stops lining up with the sheet. `|` is safe in the attendance block - the
//! cells there are `X`, a day number, or a formula result - and is what
//! [`block_formula`] produces, so the day grid is read with the same builder the
//! legacy-month reader uses. It is *not* safe in the `NAME` column: the DepEd
//! form's own subtotal rows read `<=== MALE | TOTAL Per Day ===>`. The name
//! column is therefore joined with [`NAME_SEPARATOR`], a control character no
//! cell can hold.
//!
//! ## Hidden sheets are read too
//!
//! [`analyze_workbook`](crate::sf2::excel::analyze_workbook) - and therefore
//! [`crate::backup::x_count`] - skip any worksheet whose `Visible` is not
//! `xlSheetVisible`. On a file the pre-split calendar cycle has been through,
//! that is eleven of the twelve months. The diagnostic reads every sheet
//! whatever its visibility, and says which ones it could not place.

use super::model::RosterRow;
use crate::domain::error::{AppError, Result};
use crate::sf2::excel::excel_com::com_session::{
    run_excel_task, with_workbook, ComObject, ComVariant, ExcelSession,
};
use crate::sf2::excel::excel_com::workbook_utils::{
    column_number_to_letter, month_number, year_from_sheet_name,
};
use crate::sf2::excel::excel_com::worksheet::cell_text;
use crate::sf2::logic::{is_learner_name, SF2_ABSENT_MARK};
use crate::sf2::month::workbook_builder::RowBand;
use std::path::{Path, PathBuf};

/// `xlSheetVisible`. A worksheet whose `Visible` is anything else is hidden.
const EXCEL_SHEET_VISIBLE: i32 = -1;

/// The row the DepEd form keeps its day numbers in.
const DAY_ROW: u32 = 6;

/// First day column of the attendance grid (`F`).
const FIRST_DAY_COLUMN: u32 = 6;

/// Last day column of the attendance grid (`AL`).
const LAST_DAY_COLUMN: u32 = 38;

/// The `NAME` column, merged across `C:E`.
const NAME_COLUMN: u32 = 3;

/// Last row of the block read. The roster ends well before this and the form's
/// adviser/signature block starts at 76, so nothing an `X` could legitimately
/// be in is left out.
const LAST_BLOCK_ROW: u32 = 80;

/// The `report_month` header cell: column `AA`, row 3.
const REPORT_MONTH_ROW: i32 = 3;
const REPORT_MONTH_COLUMN: i32 = 27;

/// The separator the attendance block is joined with, matching the builder in
/// `month::workbook_builder`. Safe here: the block holds `X`, day numbers and
/// formula results, and no DepEd absence mark contains a pipe.
const BLOCK_SEPARATOR: char = '|';

/// The separator the `NAME` column is joined with.
///
/// A control character, because the form's own subtotal labels contain a pipe
/// (`<=== MALE | TOTAL Per Day ===>`) and a pipe in the data would shift every
/// name after that row onto the wrong roster row.
const NAME_SEPARATOR: char = '\u{1}';

/// One `X` cell, addressed the way Excel addresses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawMark {
    /// 1-based Excel row.
    pub row_index: u32,
    /// 1-based Excel column.
    pub column_index: u32,
}

impl RawMark {
    /// The A1 address, e.g. `AL47`.
    #[must_use]
    pub fn address(&self) -> String {
        format!(
            "{}{}",
            column_number_to_letter(self.column_index as i32),
            self.row_index
        )
    }
}

/// A worksheet, as the workbook has it, before any month has been assigned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSheet {
    pub sheet_name: String,
    pub visible: bool,
    /// The month the sheet's **name** says it is, when the name says one.
    pub month_from_name: Option<u32>,
    /// The year the sheet's name says, when it says one.
    pub year_from_name: Option<i32>,
    /// The sheet's own `report_month` header cell, verbatim.
    ///
    /// Reported, never trusted. `configure_sf2_calendar` rewrites this cell on
    /// the one sheet it makes visible and leaves the rest holding whatever the
    /// bundled template shipped, so on a file that has been through that cycle
    /// every hidden sheet can claim to be the same month. Trusting it would
    /// read four sample-data worksheets as September and count them twice.
    pub header_month_label: String,
    /// `(column_index, day_number)` for every day column that carries a day.
    pub day_numbers: Vec<(u32, u32)>,
    /// Every `X` cell in the block, in row-then-column order.
    pub marks: Vec<RawMark>,
    /// The worksheet's own learner names, by row.
    pub roster_names: Vec<RosterRow>,
}

impl RawSheet {
    /// How many `X` cells the worksheet holds, over every row and column of the
    /// block. The roster-free count: what is on the sheet, regardless of whether
    /// the database can place any of it.
    #[must_use]
    pub fn mark_count(&self) -> usize {
        self.marks.len()
    }

    /// The name of the learner in `row_index`, when this sheet names one.
    #[must_use]
    pub fn name_at(&self, row_index: u32) -> Option<&str> {
        self.roster_names
            .iter()
            .find(|row| row.row_index == row_index)
            .map(|row| row.workbook_name.as_str())
    }
}

/// A workbook, as the diagnostic found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawWorkbook {
    pub path: PathBuf,
    pub sheets: Vec<RawSheet>,
}

impl RawWorkbook {
    /// The first sheet that names `month`, by name then by tab order.
    ///
    /// Name first, so a workbook that keeps all twelve months visible resolves
    /// each month to its own sheet rather than to whichever comes first.
    #[must_use]
    pub fn sheet_for_month(&self, month: u32) -> Option<&RawSheet> {
        self.sheets
            .iter()
            .find(|sheet| sheet.month_from_name == Some(month))
    }
}

/// Read every worksheet of `path`, in one read-only session.
pub fn probe_workbook(path: &Path) -> Result<RawWorkbook> {
    let path_label = path.display().to_string();
    let opened = path.to_path_buf();
    let sheets = run_excel_task(move || {
        with_workbook(&opened, true, false, |excel, workbook| {
            probe_in_session(excel, workbook)
        })
    })
    .map_err(|error| {
        AppError::Internal(format!(
            "could not read the workbook at {path_label}: {error}"
        ))
    })?;

    Ok(RawWorkbook {
        path: path.to_path_buf(),
        sheets,
    })
}

/// The `Worksheets` walk, inside the open read-only workbook.
fn probe_in_session(excel: &ExcelSession, workbook: &ComObject) -> Result<Vec<RawSheet>> {
    let worksheets = workbook.get_object("Worksheets")?;
    let sheet_count = worksheets.get_i32("Count")?;

    let mut sheets = Vec::with_capacity(sheet_count.max(0) as usize);
    for index in 1..=sheet_count {
        let sheet = worksheets.get_object_with_args("Item", vec![ComVariant::i4(index)])?;
        let sheet_name = sheet.get_string("Name")?;
        let block = read_band(
            &sheet,
            excel,
            &sheet_name,
            attendance_block(),
            BLOCK_SEPARATOR,
        )?;
        let names = read_band(&sheet, excel, &sheet_name, name_band(), NAME_SEPARATOR)?;

        sheets.push(RawSheet {
            visible: sheet.get_i32("Visible")? == EXCEL_SHEET_VISIBLE,
            header_month_label: cell_text(&sheet, REPORT_MONTH_ROW, REPORT_MONTH_COLUMN)?
                .trim()
                .to_string(),
            day_numbers: day_numbers(&block),
            marks: marks_in(&block),
            roster_names: roster_names(&names),
            year_from_name: positive_year(year_from_sheet_name(&sheet_name)),
            month_from_name: positive_month(month_number(&sheet_name)),
            sheet_name,
        });
    }
    Ok(sheets)
}

/// The whole attendance block of a worksheet: rows 1..=80 by columns `F`..`AL`.
#[must_use]
pub fn attendance_block() -> RowBand {
    RowBand {
        first_row: 1,
        last_row: LAST_BLOCK_ROW,
        first_column: FIRST_DAY_COLUMN,
        last_column: LAST_DAY_COLUMN,
    }
}

/// The `NAME` column, rows 1..=80.
///
/// Its own band because [`attendance_block`] starts at `F`: the name column is
/// not inside it, and reading a learner name out of the first cell of a day
/// column would label every learner with a day number.
#[must_use]
pub fn name_band() -> RowBand {
    RowBand {
        first_row: 1,
        last_row: LAST_BLOCK_ROW,
        first_column: NAME_COLUMN,
        last_column: NAME_COLUMN,
    }
}

/// A band, as a grid of trimmed cell text. Row `r` is `grid[r - first_row]`, and
/// within it column `c` is `grid[...][c - first_column]`.
type Grid = Vec<Vec<String>>;

/// Read a band of one sheet, as a grid.
///
/// Two strategies, both validated: the whole band in one Excel evaluation, and
/// one evaluation per row. The first is one COM round trip and is what a modern
/// Excel gives; the second is what it falls back to, and on this project's
/// Excel it is the one that actually completes. Neither is trusted without a
/// token count.
fn read_band(
    sheet: &ComObject,
    excel: &ExcelSession,
    sheet_name: &str,
    band: RowBand,
    separator: char,
) -> Result<Grid> {
    if let Some(grid) = read_band_in_one_call(sheet, excel, sheet_name, band, separator) {
        return Ok(grid);
    }
    read_band_row_by_row(sheet, excel, sheet_name, band, separator)
}

/// One `TEXTJOIN` over the whole band, or `None` when Excel would not do it.
fn read_band_in_one_call(
    sheet: &ComObject,
    excel: &ExcelSession,
    sheet_name: &str,
    band: RowBand,
    separator: char,
) -> Option<Grid> {
    let formula = format!(
        "TEXTJOIN(\"{separator}\",FALSE,{}!{})",
        quote_sheet_name(sheet_name),
        band_address(band)
    );
    let flattened = evaluate_flattened(sheet, excel, &formula)?;
    split_grid(&flattened, band, separator)
}

/// One `TEXTJOIN` per row.
///
/// A single row of the 33-column block is the largest read measured to work
/// through `Application.Evaluate` on this project's Excel: four rows come back
/// whole, five do not, and neither does a 33-term `&` chain. One call per row
/// costs COM round trips and buys a read that actually completes - which is the
/// whole trade, because a read that comes back empty is a month that looks
/// empty.
///
/// `TEXTJOIN` rather than the `&` form, because the `&` chain fails at 33 terms
/// where `TEXTJOIN` over the same 33 cells succeeds.
fn read_band_row_by_row(
    sheet: &ComObject,
    excel: &ExcelSession,
    sheet_name: &str,
    band: RowBand,
    separator: char,
) -> Result<Grid> {
    let quoted = quote_sheet_name(sheet_name);
    let mut grid = Vec::with_capacity(band.row_count() as usize);
    for row in band.first_row..=band.last_row {
        let formula = format!(
            "TEXTJOIN(\"{separator}\",FALSE,{quoted}!{})",
            row_address(band, row)
        );
        let flattened = evaluate_flattened(sheet, excel, &formula).ok_or_else(|| {
            AppError::Internal(format!(
                "Excel would not read row {row} of '{sheet_name}'; the sheet is not readable and \
                 its marks cannot be counted"
            ))
        })?;
        let tokens: Vec<String> = flattened
            .split(separator)
            .map(|token| token.trim().to_string())
            .collect();
        if tokens.len() != band.column_count() as usize {
            return Err(AppError::Internal(format!(
                "read {} of {} cells from '{sheet_name}' row {row}; the sheet layout is not the \
                 DepEd SF2 grid this build expects",
                tokens.len(),
                band.column_count()
            )));
        }
        grid.push(tokens);
    }
    Ok(grid)
}

/// Evaluate one flattening formula, `None` when Excel answers with anything
/// that is not text - an error variant included.
///
/// The formula is evaluated **on the worksheet**, not on the application, and
/// that is not a detail. `Application.Evaluate` behaves like typing the formula
/// into the *active* cell, and on a workbook whose first tab is hidden -
/// `__SF2_HIDDEN_1`, which is what the pre-split calendar cycle leaves behind -
/// that context refuses the evaluation outright and returns an error variant.
/// Eleven of a file's twelve month sheets are hidden in exactly that state.
/// `Worksheet.Evaluate` evaluates in the target sheet's own context, so a hidden
/// sheet reads the same as a visible one. The application is kept as a fallback
/// for an Excel old enough not to have it.
///
/// An error variant is `None`, not `Some("")`. That is the whole reason this
/// module validates every read by token count, and the reason the guard's own
/// scanner validates too: a formula Excel refused must never be able to reach a
/// caller as an empty answer, because an empty answer is a month that reads as
/// holding no absences.
fn evaluate_flattened(sheet: &ComObject, excel: &ExcelSession, formula: &str) -> Option<String> {
    let answer = sheet
        .method("Evaluate", vec![ComVariant::bstr(formula)])
        .or_else(|_| {
            excel
                .app
                .method("Evaluate", vec![ComVariant::bstr(formula)])
        })
        .ok()?;
    // `to_string_value` is fallible precisely so this line cannot turn an Excel
    // error value into the empty string.
    answer.to_string_value().ok()
}

/// Split a flattened band back into a grid, refusing a short or long answer.
fn split_grid(flattened: &str, band: RowBand, separator: char) -> Option<Grid> {
    let tokens: Vec<String> = flattened
        .split(separator)
        .map(|token| token.trim().to_string())
        .collect();
    if tokens.len() != band.cell_count() {
        return None;
    }
    Some(
        tokens
            .chunks(band.column_count() as usize)
            .map(<[String]>::to_vec)
            .collect(),
    )
}

/// The worksheet's own day-number row, as `(column_index, day)` pairs.
///
/// Blank, non-numeric and out-of-range cells are not day columns and are
/// skipped - the same acceptance `analyze_workbook` applies, so the two reads
/// cannot disagree about which columns hold days.
fn day_numbers(grid: &Grid) -> Vec<(u32, u32)> {
    let block = attendance_block();
    let Some(row) = grid.get(DAY_ROW.saturating_sub(block.first_row) as usize) else {
        return Vec::new();
    };
    // The range is parenthesised on purpose: `a..=b.method()` parses as
    // `a..=(b.method())`, which silently turns the whole expression into a
    // range and drops the iterator chain.
    (block.first_column..=block.last_column)
        .zip(row.iter())
        .filter_map(|(column, text)| {
            let day = text.trim().parse::<u32>().ok()?;
            ((1..=31).contains(&day)).then_some((column, day))
        })
        .collect()
}

/// Every `X` in the block.
fn marks_in(grid: &Grid) -> Vec<RawMark> {
    let block = attendance_block();
    let mut marks = Vec::new();
    for (row_offset, row) in grid.iter().enumerate() {
        for (column_offset, text) in row.iter().enumerate() {
            if text.trim().eq_ignore_ascii_case(SF2_ABSENT_MARK) {
                marks.push(RawMark {
                    row_index: block.first_row + row_offset as u32,
                    column_index: block.first_column + column_offset as u32,
                });
            }
        }
    }
    marks
}

/// The worksheet's own learner names, by row.
fn roster_names(names: &Grid) -> Vec<RosterRow> {
    names
        .iter()
        .enumerate()
        .filter_map(|(row_offset, row)| {
            let name = row.first()?.trim();
            is_learner_name(name).then(|| RosterRow {
                student_id: String::new(),
                workbook_name: name.to_string(),
                row_index: NAME_BAND_FIRST_ROW + row_offset as u32,
            })
        })
        .collect()
}

/// The first row of the name band, restated so the row arithmetic above reads
/// without a second lookup.
const NAME_BAND_FIRST_ROW: u32 = 1;

/// Quote a sheet name for a formula, doubling embedded single quotes.
fn quote_sheet_name(sheet_name: &str) -> String {
    format!("'{}'", sheet_name.replace('\'', "''"))
}

/// `F1:AL80`, the range part of a formula.
fn band_address(band: RowBand) -> String {
    format!(
        "{}{}:{}{}",
        column_number_to_letter(band.first_column as i32),
        band.first_row,
        column_number_to_letter(band.last_column as i32),
        band.last_row
    )
}

/// `F1:AL1`, one row of a band.
fn row_address(band: RowBand, row: u32) -> String {
    format!(
        "{}{}:{}{}",
        column_number_to_letter(band.first_column as i32),
        row,
        column_number_to_letter(band.last_column as i32),
        row
    )
}

/// A year a sheet's name actually carries. `0` means it carries none.
fn positive_year(year: i32) -> Option<i32> {
    (year > 0).then_some(year)
}

/// A month a sheet's name actually names. `0` means it names none - which is the
/// case for a worksheet the pre-split calendar cycle renamed to
/// `__SF2_HIDDEN_n`, and the reason its marks cannot be placed in a month.
fn positive_month(month: u32) -> Option<u32> {
    (month > 0).then_some(month)
}
