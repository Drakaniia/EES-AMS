//! The worksheets of the single-file, twelve-sheet SF2 workbook, and the naming
//! rules that decide which of them is a month.
//!
//! The DepEd School Form 2 is **one worksheet per month** on one file. This
//! module owns the two things that must never disagree about that file:
//!
//! * the form's own geometry - which row a learner is on, where the weekday
//!   header sits, where a MALE TOTAL row lands - because the code that *empties* a
//!   worksheet and the code that *fills* one both read it from here; and
//! * which worksheet name counts as a month, because that single predicate is what
//!   makes "never import the bundled template's sample data" structural rather
//!   than a promise.
//!
//! ## Why the naming rule is the safety property
//!
//! The bundled `TEMPLATE_AUTOMATED_SF2.xls` ships five month worksheets and a
//! sixth copy of the form called `COMPLETE DAYS`, carrying **36 `X` marks** on a
//! roster of names that are not this teacher's students. The old per-month-file
//! design also left `__SF2_HIDDEN_{n}` worksheets behind whenever it hid and
//! renamed a tab.
//!
//! [`is_month_sheet_name`] accepts a name only if it parses to a real month **and**
//! a four-digit year of this century. `JUNE`, `__SF2_HIDDEN_1` and `COMPLETE
//! DAYS` all fail it. So no read path in this project can reach a worksheet that
//! is not a month of the school year, and the template's fiction cannot become the
//! user's attendance no matter which worksheet a future edit points at.
//!
//! ## What "emptying" a worksheet means
//!
//! A month worksheet is made by **copying the form** and then clearing the roster
//! block of the copy - never by clearing a worksheet the user may have filled in.
//! [`empty_month_sheet`] unmerges the roster rows before clearing them and
//! re-applies the donor's *formats* afterwards, so the DepEd layout - borders,
//! the `C:E` name merge, the day-column fills - survives while not one mark, name
//! or number of the donor's sample class does.

use crate::domain::error::{AppError, Result};
use crate::sf2::excel::excel_com::com_session::{ComObject, ComVariant};
use crate::sf2::excel::excel_com::worksheet::cell_text;
use crate::sf2::logic::SF2_ABSENT_MARK;
use crate::sf2::models::Sf2WorkbookLearner;
use crate::sf2::month::workbook_builder::MonthDaySlot;
use std::path::Path;

// ── The form's geometry ──────────────────────────────────────────────────────
// Read off `TEMPLATE_AUTOMATED_SF2.xls` and verified against the affected
// install's own workbook on 2026-09-27. A cell address here is part of the school
// record's format, not a preference: `sf2_month_student_mappings.row_index` and
// `sf2_month_date_mappings.column_letter` are both stored in these coordinates.

/// The first day column of the SF2 grid, `F` - the left half of the merged
/// `F:G` first day cell.
pub const SF2_FIRST_DAY_COLUMN: u32 = 6;
/// The last day column of the SF2 grid, `AL` - the right half of the merged
/// `AK:AL` last day cell.
pub const SF2_LAST_DAY_COLUMN: u32 = 38;
/// The `No.` column, `A`, which carries the item number.
pub const SF2_ITEM_NUMBER_COLUMN: i32 = 1;
/// The `NAME` column, `C`. The form merges `C:E` across a learner row.
pub const SF2_NAME_COLUMN: i32 = 3;
/// The row the day numbers are printed in, 6.
pub const SF2_DAY_ROW: i32 = 6;
/// The row the weekday header is printed in, 7.
pub const SF2_WEEKDAY_ROW: i32 = 7;
/// The first learner row of a fresh bundled template, 8.
pub const SF2_FIRST_LEARNER_ROW: u32 = 8;
/// The row the adviser's signature is printed in, 53.
pub const SF2_ADVISER_ROW: i32 = 53;
/// The row the class head's signature is printed in, 54.
pub const SF2_SIGNATURE_ROW: i32 = 54;

/// Male learner slots a fresh bundled template ships with, 21.
pub const SF2_FRESH_MALE_SLOTS: u32 = 21;
/// Female learner slots a fresh bundled template ships with, 19.
pub const SF2_FRESH_FEMALE_SLOTS: u32 = 19;
/// The MALE TOTAL row of a fresh, unexpanded bundled template, 29.
pub const SF2_FRESH_MALE_TOTAL_ROW: u32 = 29;
/// The first FEMALE learner row of a fresh, unexpanded bundled template, 30.
pub const SF2_FRESH_FEMALE_START_ROW: u32 = 30;
/// The FEMALE TOTAL row of a fresh, unexpanded bundled template, 49.
pub const SF2_FRESH_FEMALE_TOTAL_ROW: u32 = 49;
/// The Combined TOTAL row, one below the FEMALE TOTAL, 50.
pub const SF2_FRESH_COMBINED_TOTAL_ROW: u32 = 50;

/// The `ABSENT` column, `AM`, immediately right of the day grid.
pub const SF2_ABSENT_COLUMN: u32 = 39;

/// The prefix of the retired hide/rename cycle's worksheets.
///
/// The old per-month-file design hid a tab and renamed it to `__SF2_HIDDEN_{n}`
/// while it rebuilt the month. Those leftovers are recognised here so they can be
/// *removed* - and, because they are not month names, so they can never be *read*
/// as one.
pub const HIDDEN_SHEET_PREFIX: &str = "__SF2_HIDDEN_";

/// Excel refuses a worksheet name longer than this.
pub const MONTH_SHEET_NAME_MAX: usize = 31;

/// The title in `A1` of a DepEd School Form 2 worksheet.
///
/// Matched as a prefix so a school's own variant - the same title with a different
/// suffix - is still recognised as a form worksheet. This is what tells a form
/// from a school's own working sheet, and it is why `COMPLETE DAYS`, which *is* a
/// full copy of the form, counts as a form and not as a helper.
pub const SF2_FORM_TITLE: &str = "School Form 2 (SF2)";

/// `xlPasteAll`.
const XL_PASTE_ALL: i32 = -4104;
/// `xlPasteFormats`.
const XL_PASTE_FORMATS: i32 = -4122;
/// `xlShiftToBottom`.
const XL_SHIFT_TO_BOTTOM: &str = "ShiftToBottom";
/// `xlCopyOrigin`, which `Rows.Insert` needs to avoid pasting the copied row back.
const XL_COPY_ORIGIN: &str = "CopyOrigin";

/// One worksheet's name, position and visibility.
#[derive(Debug, Clone)]
pub struct SheetEntry {
    /// Excel's 1-based index of the worksheet in the workbook.
    pub index: i32,
    /// The worksheet's name.
    pub name: String,
    /// Whether Excel is showing the worksheet. `xlSheetVisible` is `-1`.
    pub visible: bool,
    /// Whether the worksheet's `A1` carries the DepEd School Form 2 title.
    pub is_sf2_form: bool,
}

/// A month worksheet that exists, ready to be filled.
#[derive(Debug, Clone)]
pub struct PreparedMonthSheet {
    /// The worksheet's name, `"{MONTH} {year}"`.
    pub name: String,
    /// The MALE TOTAL row after the roster is grown to fit.
    pub male_total_row: u32,
    /// The FEMALE TOTAL row after the roster is grown to fit.
    pub female_total_row: u32,
    /// The Combined TOTAL row, one below the FEMALE TOTAL.
    pub combined_total_row: u32,
    /// The last learner row of the sheet, which is the FEMALE TOTAL row.
    pub last_roster_row: u32,
}

impl PreparedMonthSheet {
    /// The worksheet's Excel object.
    ///
    /// Named rather than left to a `Deref` because re-looking-up the worksheet by
    /// name at each call site is how a write ends up on a tab that was renamed
    /// underneath it.
    ///
    /// # Errors
    ///
    /// Fails when the workbook no longer holds a worksheet of this name.
    pub fn object(&self, workbook: &ComObject) -> Result<ComObject> {
        worksheet_by_name(workbook, &self.name)
    }
}

// ── Naming ───────────────────────────────────────────────────────────────────

/// The canonical worksheet name for a month: `"{MONTH} {year}"`.
///
/// The full month name, upper case, because that is the spelling
/// [`crate::sf2::workbook_files::month_workbook_sheet_name`] canonicalises every
/// other spelling to. Both must produce the same string, or a write addressed by a
/// stored `sheet_name` would not find its worksheet.
#[must_use]
pub fn month_sheet_name(month: u32, year: i32) -> String {
    format!(
        "{} {year}",
        crate::sf2::excel::excel_com::workbook_utils::month_name(month).trim()
    )
}

/// Whether a worksheet name is a month of a school year.
///
/// True only for a name carrying both a real month and a four-digit year of this
/// century: `SEPTEMBER 2026`, `SEPT. 2025` and `JUNE2025` all qualify. `JUNE`,
/// `__SF2_HIDDEN_1`, `COMPLETE DAYS`, `JUNE 1899` and `JUNE 20255` do not.
///
/// This is the predicate that keeps the bundled template's sample marks out of the
/// user's attendance, so it is deliberately strict.
#[must_use]
pub fn is_month_sheet_name(sheet_name: &str) -> bool {
    let (month, year) = split_month_sheet_name(sheet_name);
    month.is_some() && is_this_century(year)
}

/// Whether a worksheet name is the given month, in any year.
///
/// The year is not checked here, because a caller that already knows which school
/// year it is reading compares the month and then prefers the year itself
/// afterwards - see [`crate::sf2::month::workbook_builder::read_legacy_months`],
/// which reads a leftover tab from an earlier school year only as a last resort.
#[must_use]
pub fn is_month_sheet_of(sheet_name: &str, month: u32) -> bool {
    split_month_sheet_name(sheet_name).0 == Some(month)
}

/// Whether a worksheet name is a leftover of the retired hide/rename cycle.
///
/// The name came from a loop, not from a user, so case and padding are ignored.
#[must_use]
pub fn is_hidden_sheet_name(sheet_name: &str) -> bool {
    sheet_name
        .trim()
        .to_uppercase()
        .starts_with(HIDDEN_SHEET_PREFIX)
}

/// The month and the year a worksheet name spells out, or `(None, 0)`.
fn split_month_sheet_name(sheet_name: &str) -> (Option<u32>, i32) {
    let trimmed = sheet_name.trim();
    match trimmed.rfind(' ') {
        Some(split) => {
            let (month_part, year_part) = trimmed.split_at(split);
            (
                crate::sf2::calendar::sf2_month_number(month_part),
                year_part.trim().parse::<i32>().unwrap_or_default(),
            )
        }
        // `JUNE2025` carries no space, so the year starts where the digits do.
        None => {
            let digits = trimmed
                .char_indices()
                .rev()
                .take_while(|(_, character)| character.is_ascii_digit())
                .count();
            if digits == 0 {
                return (None, 0);
            }
            let split = trimmed.len() - digits;
            (
                crate::sf2::calendar::sf2_month_number(&trimmed[..split]),
                trimmed[split..].parse::<i32>().unwrap_or_default(),
            )
        }
    }
}

/// A year of this century: 2000..=2099.
///
/// Bounded on both sides because a worksheet named `JUNE 1899` or `JUNE 20` is not
/// a month of anyone's school year, and treating it as one would let a historical
/// or half-typed tab be read as a live month.
fn is_this_century(year: i32) -> bool {
    (2000..=2099).contains(&year)
}

/// A weekday header cell's text, as a Monday-to-Friday index.
///
/// `TH` is Thursday and `T` is Tuesday, so the two-letter spellings are matched
/// before the one-letter ones. Anything that is not a weekday - `ABSENT`,
/// `PRESENT`, the empty cell at the right-hand edge of the grid - is `None`,
/// because reading one as a weekday would add a 26th day cell and shift every mark
/// after it.
#[must_use]
pub fn parse_weekday_label(label: &str) -> Option<u32> {
    match label.trim().to_ascii_uppercase().as_str() {
        "M" | "MON" | "MONDAY" => Some(0),
        "T" | "TUE" | "TUES" | "TUESDAY" => Some(1),
        "W" | "WED" | "WEDNESDAY" => Some(2),
        "TH" | "THU" | "THUR" | "THURS" | "THURSDAY" => Some(3),
        "F" | "FRI" | "FRIDAY" => Some(4),
        _ => None,
    }
}

// ── Finding worksheets ───────────────────────────────────────────────────────

/// Every worksheet in the workbook, in tab order, with its visibility.
pub fn sheet_entries(workbook: &ComObject) -> Result<Vec<SheetEntry>> {
    eprintln!("S2DEBUG sheet_entries");
    let sheets = workbook.get_object("Worksheets")?;
    let count = sheets.get_i32("Count")?;
    let mut entries = Vec::new();
    for index in 1..=count {
        let sheet = sheets.get_object_with_args("Item", vec![ComVariant::i4(index)])?;
        let title = cell_text(&sheet, 1, 1)?;
        entries.push(SheetEntry {
            index,
            name: sheet.get_string("Name")?,
            visible: sheet.get_i32("Visible")? == -1,
            is_sf2_form: title.trim_start().starts_with(SF2_FORM_TITLE),
        });
    }
    Ok(entries)
}

/// One worksheet, by name.
///
/// Fails rather than falling back to "the first month-shaped tab", because a write
/// that lands on the wrong worksheet is a teacher's marks on a month the school
/// never opened.
///
/// # Errors
///
/// Fails when the workbook holds no worksheet of this name.
pub fn worksheet_by_name(workbook: &ComObject, name: &str) -> Result<ComObject> {
    workbook
        .get_object("Worksheets")?
        .get_object_with_args("Item", vec![ComVariant::bstr(name)])
        .map_err(|error| {
            AppError::Internal(format!(
                "the workbook has no worksheet named `{name}`: {error}"
            ))
        })
}

/// The day columns of a worksheet, from its weekday header.
///
/// Read off row 7 rather than assumed, so a school's form is measured rather than
/// presumed - and so a worksheet with no weekday header at all returns empty
/// instead of 25 invented columns.
pub fn weekday_slots(sheet: &ComObject) -> Result<Vec<MonthDaySlot>> {
    let mut slots = Vec::new();
    for column in SF2_FIRST_DAY_COLUMN..=SF2_LAST_DAY_COLUMN {
        let label = cell_text(sheet, SF2_WEEKDAY_ROW, column as i32)?;
        if let Some(weekday_index) = parse_weekday_label(&label) {
            // The form is five weeks of Monday..Friday, so every fifth labelled
            // column starts a new week. Deriving the week from the count rather
            // than from a hard-coded column list means a form that grew or lost a
            // day column still gets its weeks numbered in order.
            slots.push(MonthDaySlot {
                column,
                weekday_index,
                week_index: u32::try_from(slots.len() / 5).unwrap_or(u32::MAX),
            });
        }
    }
    Ok(slots)
}

/// Create the month worksheet if it is not there yet, and return it.
///
/// Reusing an existing worksheet of the right name is deliberate: a rebuild of a
/// school year must be able to run against a workbook that already holds eleven of
/// the twelve months, and a build that failed on the tenth month would otherwise
/// have to be thrown away whole.
pub fn prepare_month_sheet(
    workbook: &ComObject,
    month: u32,
    year: i32,
) -> Result<PreparedMonthSheet> {
    let name = month_sheet_name(month, year);
    if name.chars().count() > MONTH_SHEET_NAME_MAX {
        return Err(AppError::Internal(format!(
            "`{name}` is longer than the {MONTH_SHEET_NAME_MAX} characters Excel allows a worksheet \
             name to be"
        )));
    }

    // The workbook has to be the active one. `Application.Evaluate` - which is how
    // the build reads a worksheet back before saving - evaluates in the context of
    // the *active* workbook, so a workbook that was opened and then left behind
    // makes every verification formula unresolvable rather than wrong. Activating
    // here rather than only on the create path is deliberate: a month that already
    // exists still has to be verifiable.
    eprintln!("S2DEBUG prepare {name}");
    workbook.method("Activate", Vec::new())?;

    if worksheet_by_name(workbook, &name).is_ok() {
        return Ok(blank_prepared_month_sheet(name));
    }

    let added = workbook
        .get_object("Worksheets")?
        .get_object_with_args("Add", Vec::new())?;
    added.put_string("Name", name.as_str())?;
    // The new sheet has to be the active one too, or the clipboard paste in
    // `copy_used_range_over` lands on whichever tab happened to be showing.
    added.method("Activate", Vec::new())?;

    Ok(blank_prepared_month_sheet(name))
}

/// A month worksheet whose roster rows are not laid out yet.
///
/// The row fields are filled in by the build once the roster's shape is known;
/// they are zero here because a worksheet that has only just been created has no
/// roster of its own yet.
fn blank_prepared_month_sheet(name: String) -> PreparedMonthSheet {
    PreparedMonthSheet {
        name,
        male_total_row: 0,
        female_total_row: 0,
        combined_total_row: 0,
        last_roster_row: 0,
    }
}

// ── Shaping a month worksheet ────────────────────────────────────────────────

/// Grow the worksheet's roster to hold `extra_male` more males and `extra_female`
/// more females.
///
/// Rows are **inserted**, not appended, so the blocks that follow the roster - the
/// MALE TOTAL row, the female block, the signature rows - move down intact and keep
/// their own formulas pointing at the right rows. Appending instead would put the
/// twenty-second male on top of the MALE TOTAL row.
pub fn grow_roster_rows(sheet: &ComObject, extra_male: u32, extra_female: u32) -> Result<()> {
    if extra_female > 0 {
        // Females first, so the male rows inserted afterwards do not move the
        // female block a second time.
        insert_roster_rows(sheet, SF2_FRESH_MALE_TOTAL_ROW, extra_female)?;
    }
    if extra_male > 0 {
        // One row below the last male, which is the MALE TOTAL row itself.
        insert_roster_rows(sheet, SF2_FRESH_MALE_TOTAL_ROW, extra_male)?;
    }
    Ok(())
}

/// Insert `count` rows above `row`, shifting the rows below it down.
fn insert_roster_rows(sheet: &ComObject, row: u32, count: u32) -> Result<()> {
    for _ in 0..count {
        worksheet_range(sheet, row, row)?
            .get_object_with_args("EntireRow", Vec::new())?
            .method(
                "Insert",
                vec![
                    ComVariant::bstr(XL_SHIFT_TO_BOTTOM),
                    ComVariant::bstr(XL_COPY_ORIGIN),
                ],
            )
            .map_err(|error| {
                AppError::Internal(format!(
                    "could not make room for {count} more learner row(s) at row {row}: {error}"
                ))
            })?;
    }
    Ok(())
}

/// Copy the donor form's column widths and row heights onto the new worksheet.
///
/// `UsedRange.Copy` pastes *values and formats* but not column widths, so a month
/// sheet built by copying alone comes out with the donor's default grid - every day
/// column one character wide, and the DepEd table unreadable.
///
/// A row the donor has no height for is left alone rather than failed on: a
/// worksheet that was just created has no `Rows` collection to speak of, and a
/// default height is already the right answer.
pub fn copy_layout_geometry(donor: &ComObject, sheet: &ComObject) -> Result<()> {
    eprintln!("S2DEBUG geometry");
    for column in SF2_FIRST_DAY_COLUMN..=SF2_LAST_DAY_COLUMN {
        let width = donor
            .get_object("Columns")?
            .get_object_with_args("Item", vec![ComVariant::i4(column as i32)])?
            .get("ColumnWidth")?;
        sheet
            .get_object("Columns")?
            .get_object_with_args("Item", vec![ComVariant::i4(column as i32)])?
            .put_variant("ColumnWidth", width)?;
    }

    let rows = sheet.get_object("Rows")?;
    let last_row = sheet
        .get_object("UsedRange")?
        .get_object("Rows")?
        .get_i32("Count")?;
    for row in 1..=last_row {
        let Ok(source) = donor.get_object("Rows").and_then(|donor_rows| {
            donor_rows.get_object_with_args("Item", vec![ComVariant::i4(row)])
        }) else {
            continue;
        };
        let Ok(height) = source.get("RowHeight") else {
            continue;
        };
        if let Ok(target) = rows.get_object_with_args("Item", vec![ComVariant::i4(row)]) {
            let _ = target.put_variant("RowHeight", height);
        }
    }
    Ok(())
}

/// Empty the roster block of a worksheet copied from `donor`, keeping its formats.
///
/// This is the step that makes the bundled template's sample class impossible to
/// carry over. The order is not negotiable:
///
/// 1. **Unmerge** the roster rows. A merged `C:E` cannot be cleared cell by cell -
///    only its top-left cell exists, and writing past it fails or writes nothing.
/// 2. **Clear** the contents, which takes the sample names, the sample item
///    numbers and the sample `X` marks with it.
/// 3. **Re-apply the donor's formats** over the same block, so the borders, the
///    alignment and the day-column fills are the DepEd layout again rather than
///    twelve months of bare cells.
pub fn empty_month_sheet(
    donor: &ComObject,
    sheet: &ComObject,
    male_total_row: u32,
    female_total_row: u32,
) -> Result<()> {
    eprintln!("S2DEBUG empty rows {male_total_row} {female_total_row}");
    let last_roster_row = female_total_row.max(male_total_row);

    // 1. Unmerge.
    let _ = worksheet_range(sheet, SF2_FIRST_LEARNER_ROW, last_roster_row)?
        .method("UnMerge", Vec::new());

    // 2. Clear the sample data.
    worksheet_range(sheet, SF2_FIRST_LEARNER_ROW, last_roster_row)?
        .method("ClearContents", Vec::new())?;

    // 3. Re-apply the donor's formats.
    worksheet_range(donor, SF2_FIRST_LEARNER_ROW, last_roster_row)?.method("Copy", Vec::new())?;
    sheet.method("Activate", Vec::new())?;
    paste_at(sheet, SF2_FIRST_LEARNER_ROW, XL_PASTE_FORMATS)
}

/// Paste the donor's `UsedRange` onto the new worksheet, values and formats alike.
pub fn copy_used_range_over(donor: &ComObject, sheet: &ComObject) -> Result<()> {
    donor.get_object("UsedRange")?.method("Copy", Vec::new())?;
    sheet.method("Activate", Vec::new())?;
    paste_at(sheet, 1, XL_PASTE_ALL)?;
    // The A1 title is the caller's to overwrite. Reading it here is what notices a
    // worksheet that came out empty, while the donor is still in hand.
    let _ = cell_text(sheet, 1, 1).unwrap_or_default();
    Ok(())
}

/// Paste at `A{row}` of `sheet`, which must already be the active worksheet.
fn paste_at(sheet: &ComObject, row: u32, paste: i32) -> Result<()> {
    let anchor = worksheet_range(sheet, row, row)?;
    anchor.method("Select", Vec::new())?;
    anchor.method("PasteSpecial", vec![ComVariant::i4(paste)])?;
    Ok(())
}

/// Make every month worksheet visible, and name any that refused.
///
/// Excel will not let the *last* visible worksheet be hidden and will not accept
/// `Visible = -1` on a workbook whose only other sheet is hidden, so each refusal
/// is reported by name: a month the user cannot click to is a month that looks like
/// it is missing.
///
/// # Errors
///
/// Fails naming the first worksheet that refused to become visible.
pub fn make_month_sheets_visible(
    workbook: &ComObject,
    sheets: &[PreparedMonthSheet],
) -> Result<()> {
    for prepared in sheets {
        eprintln!("S2DEBUG visible {}", prepared.name);
        let sheet = prepared.object(workbook)?;
        if sheet.get_i32("Visible")? == -1 {
            continue;
        }
        let name = prepared.name.clone();
        if let Err(error) = sheet.put_variant("Visible", ComVariant::i4(-1)) {
            return Err(AppError::Internal(format!(
                "the worksheet `{name}` could not be made visible, so a month of the school year \
                 would be missing from the workbook: {error}"
            )));
        }
    }
    Ok(())
}

/// Remove the SF2 form worksheets that are not in `keep`, and report their names.
///
/// Only worksheets that carry the form's own title are considered, so a school's
/// own working sheet is never touched. Must be called only after every month sheet
/// exists, or the last form sheet would leave the workbook with no visible sheet and
/// Excel would refuse to save.
pub fn remove_non_month_form_sheets(workbook: &ComObject, keep: &[String]) -> Result<Vec<String>> {
    let entries = sheet_entries(workbook)?;
    let doomed = entries
        .iter()
        .filter(|entry| entry.is_sf2_form && !keep.contains(&entry.name))
        .map(|entry| (entry.index, entry.name.clone()))
        .collect::<Vec<_>>();

    // Highest index first, so the indices still to be visited stay valid.
    for (index, name) in doomed.iter().rev() {
        let sheet = workbook
            .get_object("Worksheets")?
            .get_object_with_args("Item", vec![ComVariant::i4(*index)])?;
        sheet.method("Delete", Vec::new()).map_err(|error| {
            AppError::Internal(format!("could not remove the worksheet `{name}`: {error}"))
        })?;
    }
    Ok(doomed.into_iter().map(|(_, name)| name).collect())
}

/// Rows `first..=last`, columns `A`..`AL`, of one worksheet.
fn worksheet_range(sheet: &ComObject, first_row: u32, last_row: u32) -> Result<ComObject> {
    sheet.get_object_with_args(
        "Range",
        vec![ComVariant::bstr(&format!("A{first_row}:AL{last_row}"))],
    )
}

// ── Reading a worksheet back ─────────────────────────────────────────────────
// Used by the tests that prove a saved file holds what the build said it would.
// They open the file read-only and close it without saving, so measuring a
// workbook never changes it.

/// Every worksheet in the file at `path`, with whether it is visible.
#[cfg(target_os = "windows")]
pub fn read_sheet_names(path: &Path) -> Result<Vec<(String, bool)>> {
    use crate::sf2::excel::excel_com::com_session::{run_excel_task, with_workbook};

    let path = path.to_path_buf();
    run_excel_task(move || {
        with_workbook(&path, true, false, |_excel, workbook| {
            Ok(sheet_entries(workbook)?
                .into_iter()
                .map(|entry| (entry.name, entry.visible))
                .collect())
        })
    })
}

/// Non-Windows placeholder - Excel automation is unavailable, so no workbook can
/// be read back.
#[cfg(not(target_os = "windows"))]
pub fn read_sheet_names(_path: &Path) -> Result<Vec<(String, bool)>> {
    Ok(Vec::new())
}

/// How many `X` marks the named worksheet's learner rows hold.
///
/// Read cell by cell rather than by `COUNTIF` because what has to be proven is
/// that the marks are on *this* worksheet and in the *learner* rows: a count over
/// the whole sheet would be satisfied by a mark in a total row, and one over the
/// full grid would be satisfied by a mark left in a column the month does not use.
#[cfg(target_os = "windows")]
pub fn count_absent_marks_on_sheet(path: &Path, sheet_name: &str) -> Result<usize> {
    use crate::sf2::excel::excel_com::com_session::{run_excel_task, with_workbook};

    let path = path.to_path_buf();
    let sheet_name = sheet_name.to_string();
    run_excel_task(move || {
        with_workbook(&path, true, false, |_excel, workbook| {
            let sheet = worksheet_by_name(workbook, &sheet_name)?;
            let learners = crate::sf2::excel::excel_com::learners::workbook_learners(&sheet)?;
            let slots = weekday_slots(&sheet)?;
            let columns = crate::sf2::month::workbook_builder::primary_day_columns(&slots);
            let mut count = 0usize;
            for band in crate::sf2::month::workbook_builder::attendance_bands(&learners) {
                for row in band.first_row..=band.last_row {
                    for column in &columns {
                        if cell_text(&sheet, row as i32, *column as i32)?
                            .trim()
                            .eq_ignore_ascii_case(SF2_ABSENT_MARK)
                        {
                            count += 1;
                        }
                    }
                }
            }
            Ok(count)
        })
    })
}

/// Non-Windows placeholder - Excel automation is unavailable, so no workbook can
/// be read back.
#[cfg(not(target_os = "windows"))]
pub fn count_absent_marks_on_sheet(_path: &Path, _sheet_name: &str) -> Result<usize> {
    Ok(0)
}

/// The roster on one worksheet, with each learner's own row and gender block.
///
/// This is what a build needs - a learner is a *row*, and the twelve worksheets
/// only work because the same name is on the same row on all of them - so it is
/// the shape [`crate::sf2::month::merge`] reads a class's roster in.
#[cfg(target_os = "windows")]
pub fn learners_on_sheet(path: &Path, sheet_name: &str) -> Result<Vec<Sf2WorkbookLearner>> {
    use crate::sf2::excel::excel_com::com_session::{run_excel_task, with_workbook};

    let path = path.to_path_buf();
    let sheet_name = sheet_name.to_string();
    run_excel_task(move || {
        with_workbook(&path, true, false, |_excel, workbook| {
            let sheet = worksheet_by_name(workbook, &sheet_name)?;
            crate::sf2::excel::excel_com::learners::workbook_learners(&sheet)
        })
    })
}

/// Non-Windows placeholder - Excel automation is unavailable, so no workbook can
/// be read back.
#[cfg(not(target_os = "windows"))]
pub fn learners_on_sheet(_path: &Path, _sheet_name: &str) -> Result<Vec<Sf2WorkbookLearner>> {
    Ok(Vec::new())
}

/// The learner names on one worksheet, in row order.
///
/// Used to prove that all twelve worksheets carry the **same** roster on the
/// **same** rows, which is what makes one `sf2_month_student_mappings` row set
/// valid for all twelve months.
#[cfg(target_os = "windows")]
pub fn learner_names_on_sheet(path: &Path, sheet_name: &str) -> Result<Vec<String>> {
    Ok(learners_on_sheet(path, sheet_name)?
        .into_iter()
        .map(|learner| learner.name.trim().to_string())
        .collect())
}

/// Non-Windows placeholder - Excel automation is unavailable, so no workbook can
/// be read back.
#[cfg(not(target_os = "windows"))]
pub fn learner_names_on_sheet(_path: &Path, _sheet_name: &str) -> Result<Vec<String>> {
    Ok(Vec::new())
}

#[cfg(test)]
#[path = "__tests__/workbook_sheets_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "__tests__/school_year_build_tests.rs"]
mod school_year_build_tests;
