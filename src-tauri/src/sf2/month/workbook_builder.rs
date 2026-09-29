//! Build the twelve month worksheets of one SF2 workbook, and read the marks out
//! of a legacy workbook without touching Excel cell by cell (spec §0 A1, D1,
//! D14).
//!
//! Two things live here, and they are deliberately in one file because they are
//! the two halves of the same COM conversation:
//!
//! * [`build_school_year_workbook`] - writes the month worksheets of the one
//!   file the class has, dates each one, writes the header, copies the roster,
//!   and writes the `X` marks from the database. It **verifies before it saves**:
//!   a build where any month came back wrong is closed without saving, so the file
//!   on disk is left exactly as it was and the preserved copy in `_legacy/` is
//!   still the original.
//! * [`read_legacy_months`] - one bulk range read per band of learner rows, for
//!   every month, in a single read-only Excel session.
//!
//! ## Where the marks come from, and where they cannot come from
//!
//! From `events` in the database, and never from a worksheet. The bundled
//! DepEd template ships **36 `X` marks of its own** (`JUNE 2025` = 13,
//! `OCTOBER 2025` = 23) on a roster of names that are not this teacher's
//! students, and the pre-split workbook those marks were verified against is
//! still in `sf2-workbooks/`. Reading a month out of a sheet is therefore the one
//! operation this project must never perform: it would turn the template's
//! fiction into the user's attendance, and from then on every mark count, every
//! guard comparison and every Excel `COUNTIF` would be measuring it.
//!
//! [`read_legacy_months`] exists for the *opposite* purpose - it is how the merge
//! job reads what the user's own workbook holds so it can prove, before writing,
//! that the database holds at least as much (spec §0 A5) - and it only ever
//! matches a worksheet whose name parses to a real month of the school year. A
//! `__SF2_HIDDEN_{n}` sheet has no month in its name and so can never be read as
//! one.
//!
//! ## Why the source is read with a formula, not cell by cell
//!
//! The attendance block is 33 columns wide and up to ~100 learner rows per
//! month. Reading it one `Range(...).Text` at a time is ~3,300 COM round-trips
//! per month and ~40,000 for a school year - minutes of wall clock during which
//! Excel looks hung and the user assumes the app has frozen. So a whole band is
//! handed to Excel as **one formula** and the answer comes back as a single
//! string, the same trick [`crate::backup::x_count`] uses with `COUNTIF`.
//!
//! `TEXTJOIN` is the compact form: one range argument, one call per band. It
//! only exists from Excel 2019, so the `&`-concatenated form is the fallback -
//! a chain of every cell in the band, chunked so the formula stays well inside
//! Excel's formula-length limit. Both are tried and the *shape* of the answer is
//! checked either way, so a formula Excel could not evaluate fails the month
//! loudly instead of quietly yielding a short grid that reads as "no marks".
//!
//! ## Only learner rows are read
//!
//! A band is the learner rows of one gender block, never `first..=last` over the
//! whole sheet: the MALE/FEMALE TOTAL rows sit between them and hold `SUM`
//! formulas, whose evaluated values are numbers. Reading them would count
//! phantom marks, and writing them back would be refused by the formula guard -
//! aborting the month for a row that was never a learner.

use crate::domain::error::{AppError, Result};
use crate::sf2::excel::excel_com::com_session::{
    run_excel_task, with_workbook, ComObject, ComVariant, ExcelSession,
};
use crate::sf2::excel::excel_com::learners::workbook_learners;
use crate::sf2::excel::excel_com::workbook_utils::{
    column_number_to_letter, month_number, year_from_sheet_name,
};
use crate::sf2::excel::excel_com::worksheet::{
    cell_text, set_sf2_cell, set_sf2_formula, set_sf2_mark_force,
};
use crate::sf2::logic::{Sf2CellMark, SF2_ABSENT_MARK};
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord, Sf2WorkbookLearner};
use crate::sf2::month::workbook_sheets::{
    copy_layout_geometry, copy_used_range_over, empty_month_sheet, grow_roster_rows,
    is_month_sheet_of as is_month_sheet_for, make_month_sheets_visible, prepare_month_sheet,
    remove_non_month_form_sheets, sheet_entries, weekday_slots, worksheet_by_name,
    PreparedMonthSheet, SF2_ADVISER_ROW, SF2_DAY_ROW, SF2_FIRST_DAY_COLUMN,
    SF2_FRESH_FEMALE_START_ROW, SF2_ITEM_NUMBER_COLUMN, SF2_LAST_DAY_COLUMN, SF2_NAME_COLUMN,
    SF2_SIGNATURE_ROW,
};
use crate::sf2::month::Sf2MonthDateMapping;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use std::collections::{HashMap, HashSet};
use std::path::Path;

// ── SF2 form geometry ────────────────────────────────────────────────────────
// The DepEd School Form 2 layout the bundled template ships lives in
// [`workbook_sheets`], because that module both empties a sheet and fills one and
// the two cannot disagree about where the roster ends. These are the form's own
// coordinates rather than Excel constants: a cell address is part of the school
// record's format.

/// Male learner slots a fresh bundled template ships with.
const SF2_FRESH_MALE_SLOTS: usize = 21;
/// Female learner slots a fresh bundled template ships with.
const SF2_FRESH_FEMALE_SLOTS: usize = 19;

/// The MALE total row's label, which is how the female block is found.
const SF2_MALE_TOTAL_LABEL: &str = "MALE TOTAL";
/// The FEMALE total row's label.
const SF2_FEMALE_TOTAL_LABEL: &str = "FEMALE TOTAL";
/// The combined total row's label.
const SF2_COMBINED_TOTAL_LABEL: &str = "COMBINED TOTAL";

/// The separator used to flatten a range into one string.
///
/// A pipe cannot appear in a DepEd absence mark, and the token count is checked
/// on the way back in, so a cell that somehow did hold one fails the month
/// instead of quietly shifting every mark after it.
const BLOCK_CELL_SEPARATOR: &str = "|";

/// Ceiling on the cells one `TEXTJOIN` call may flatten.
const MAX_CELLS_PER_READ: usize = 4_000;

/// Ceiling on the characters one concatenation formula may reach.
const MAX_CONCATENATION_FORMULA_LEN: usize = 4_000;

/// Characters one concatenation cell costs: `'SHEET NAME'!F8&"|"&`. Deliberately
/// over-estimated, so the real formula is always shorter than the budget.
const CONCATENATION_CELL_OVERHEAD: usize = 12;

/// One day column of a month file, resolved against the sheet's own weekday
/// header rather than against a hardcoded calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthDaySlot {
    /// 1-based Excel column of the day cell.
    pub column: u32,
    /// 0-based week, counting from the first week the month touches.
    pub week_index: u32,
    /// 0 = Monday .. 4 = Friday.
    pub weekday_index: u32,
}

/// The header block, copied from the legacy workbook's own template row.
#[derive(Debug, Clone, Default)]
pub struct MonthHeader {
    pub school_id: String,
    pub school_name: String,
    pub school_year: String,
    pub report_month: String,
    pub grade_level: String,
    pub section: String,
    pub adviser_name: String,
    pub school_head_name: String,
}

/// One learner row to write onto every one of the twelve worksheets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthLearnerWrite {
    /// The `students.id` this row is. Carried so the build can resolve an
    /// absence's `(student, date)` pair to a cell without a second lookup, and so
    /// a mark is never addressed by name.
    pub student_id: String,
    pub row_index: u32,
    pub name: String,
    /// The `No.` cell value, 1-based within its gender block.
    pub item_number: u32,
    /// `MALE` / `FEMALE`, which is what decides the band this row is read in.
    pub gender_block: Option<String>,
}

/// One cell to copy verbatim into a month sheet's attendance grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthMarkWrite {
    pub row_index: u32,
    /// 1-based Excel column.
    pub column_index: u32,
    pub value: String,
}

/// One absence the database holds, before it has been given a cell.
///
/// Deliberately *not* a cell address. The address is `(student -> row) x (date ->
/// column)`, and both of those are only knowable once the sheet's roster and day
/// grid have been laid out - so the build resolves them, and the same arithmetic
/// that writes the grid resolves the marks. That is what makes it impossible for
/// a mark to land in a column the sheet's own `COUNTIF` formulas do not count.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MonthAbsence {
    pub student_id: String,
    /// `YYYY-MM-DD`.
    pub date: String,
}

/// Everything the build needs to write one month's worksheet.
#[derive(Debug, Clone)]
pub struct MonthBuildRequest {
    /// The `sf2_month_templates.id` this month's grid is recorded against.
    pub template_id: String,
    pub report_month: String,
    pub report_year: i32,
    pub first_school_day: u32,
    pub header: MonthHeader,
    pub learners: Vec<MonthLearnerWrite>,
    /// The `X` marks to write, resolved from [`Self::absences`] against the day
    /// grid and the roster the build has just laid out.
    ///
    /// Never read from a worksheet. The bundled template ships 36 marks of its
    /// own on a sample roster, and reading marks out of any sheet is the one
    /// operation that would turn that fiction into this user's attendance.
    pub absences: Vec<MonthAbsence>,
    /// Where the female block starts, so the sheet is grown to the roster's shape
    /// and the row indices above mean the same thing on all twelve sheets.
    pub source_female_start_row: u32,
}

impl MonthBuildRequest {
    /// How many `X` marks this month's worksheet is expected to hold.
    ///
    /// Derived rather than passed, so it cannot drift from what the build was
    /// actually asked to write - and a month that comes back with a different
    /// number fails the whole build instead of being saved and quietly
    /// mis-measured by every later guard comparison.
    #[must_use]
    pub fn expected_x_count(&self) -> usize {
        self.absences.len()
    }

    /// How many learner rows this month's worksheet is expected to hold.
    #[must_use]
    pub fn expected_learner_rows(&self) -> usize {
        self.learners.len()
    }
}

/// What one month's worksheet ended up holding, and whether it matched.
#[derive(Debug, Clone)]
pub struct MonthBuildReport {
    /// The worksheet the month was written to, named `"{MONTH} {year}"`.
    pub sheet_name: String,
    /// The grid the month was laid out over, ready for
    /// `sf2_month_date_mappings` - each row carrying the `sheet_name` it is
    /// written to, which is what makes a twelve-sheet file addressable.
    pub dates: Vec<Sf2MonthDateMapping>,
    /// How many cells were written into the attendance grid.
    pub written_marks: usize,
    /// How many extra roster rows the sheet had to grow by.
    pub extra_roster_rows: u32,
    /// Non-monthly helper sheets the bundled template ships, kept as they are.
    pub kept_helper_sheets: Vec<String>,
    pub verification: MonthVerification,
}

/// The result of comparing a freshly built month against its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonthVerification {
    Verified,
    Mismatch {
        expected_x: usize,
        found_x: usize,
        expected_learners: usize,
        found_learners: usize,
    },
}

impl MonthVerification {
    /// Did the month come back exactly as the source had it?
    #[must_use]
    pub fn is_verified(&self) -> bool {
        matches!(self, Self::Verified)
    }

    /// One line naming the two numbers that disagreed, for the log and the UI.
    #[must_use]
    pub fn mismatch_reason(&self) -> Option<String> {
        match self {
            Self::Verified => None,
            Self::Mismatch {
                expected_x,
                found_x,
                expected_learners,
                found_learners,
            } => Some(format!(
                "copied {found_x} of {expected_x} X marks and {found_learners} of {expected_learners} learner rows"
            )),
        }
    }
}

/// A contiguous run of learner rows, read as one bulk range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBand {
    pub first_row: u32,
    pub last_row: u32,
    pub first_column: u32,
    pub last_column: u32,
}

impl RowBand {
    #[must_use]
    pub fn row_count(&self) -> u32 {
        self.last_row.saturating_sub(self.first_row) + 1
    }

    #[must_use]
    pub fn column_count(&self) -> u32 {
        self.last_column.saturating_sub(self.first_column) + 1
    }

    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.row_count() as usize * self.column_count() as usize
    }
}

/// How one band is flattened into a single string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReadStrategy {
    /// `TEXTJOIN` over the band - one call, Excel 2019 and later.
    TextJoin,
    /// A `&`-joined chain of every cell - every Excel version ever.
    Concatenate,
}

/// One month of the legacy workbook, read without writing to it.
#[derive(Debug, Clone)]
pub struct LegacyMonthSnapshot {
    /// The legacy sheet the month was read from.
    pub sheet_name: String,
    pub report_month: String,
    pub report_year: i32,
    pub learners: Vec<Sf2WorkbookLearner>,
    /// Every non-empty cell of the learner rows, with its absolute position.
    pub marks: Vec<MonthMarkWrite>,
    /// How many `X` marks the month holds.
    pub x_count: usize,
    /// The day number the sheet prints in each labelled day column, read out of
    /// its own row 6.
    ///
    /// Needed to turn a mark's *column* back into a *date*, which is what makes
    /// §0 A5's "the workbook holds a mark the database cannot produce" check
    /// exact rather than a bare count comparison. Read from the sheet rather than
    /// recomputed from the calendar, because the number in the cell is the day
    /// whoever wrote the sheet meant.
    pub day_by_column: HashMap<u32, u32>,
    /// Where the source's female block starts, so the month file is grown to
    /// the same shape and the row indices above stay valid.
    pub female_start_row: u32,
}

impl LegacyMonthSnapshot {
    /// The `YYYY-MM-DD` a mark in `column` stands for, or `None` when the sheet
    /// prints no day there.
    ///
    /// `None` is the honest answer for the seven unlabelled columns of a merged
    /// weekday pair and for every column past the month's last school day. A
    /// mark there cannot be matched against a date, so §0 A5's check ignores it
    /// rather than inventing one - and a mark in a sub-cell is a duplicate of its
    /// pair's primary anyway, which *is* checked.
    #[must_use]
    pub fn date_in_column(&self, column_index: u32) -> Option<String> {
        let day = self.day_by_column.get(&column_index)?;
        let month = crate::sf2::calendar::sf2_month_number(&self.report_month)?;
        NaiveDate::from_ymd_opt(self.report_year, month, *day)
            .map(|date| date.format("%Y-%m-%d").to_string())
    }
}

/// One month of a bulk read, successful or not.
///
/// A month is a separate outcome rather than a separate session because a
/// failure has to stay *inside* the month: the legacy file may simply have no
/// sheet for one month, and that must not cost the other eleven their read.
#[derive(Debug, Clone)]
pub struct LegacyMonthRead {
    pub report_month: String,
    pub report_year: i32,
    pub snapshot: Option<LegacyMonthSnapshot>,
    /// Why this month could not be read, when it could not.
    pub error: Option<String>,
}

// ── Pure geometry ───────────────────────────────────────────────────────────

/// The learner rows of one month's sheet, as one band per gender block.
///
/// Splitting by gender block is what keeps the MALE/FEMALE TOTAL rows - `SUM`
/// formulas, not marks - out of the copy. A roster with no female learners
/// produces a single band.
#[must_use]
pub fn attendance_bands(learners: &[Sf2WorkbookLearner]) -> Vec<RowBand> {
    bands_from(&learner_rows(
        learners
            .iter()
            .map(|learner| (learner.row_index, learner.gender_block.clone())),
    ))
}

/// The same bands, for a roster that is about to be written rather than read.
#[must_use]
pub fn attendance_bands_for_writes(learners: &[MonthLearnerWrite]) -> Vec<RowBand> {
    bands_from(&learner_rows(
        learners
            .iter()
            .map(|learner| (learner.row_index, learner.gender_block.clone())),
    ))
}

fn learner_rows(rows: impl Iterator<Item = (u32, Option<String>)>) -> Vec<(u32, Option<String>)> {
    rows.collect()
}

fn bands_from(rows: &[(u32, Option<String>)]) -> Vec<RowBand> {
    ["MALE", "FEMALE"]
        .iter()
        .filter_map(|block| {
            let mut block_rows = rows
                .iter()
                .filter(|(_, gender_block)| gender_block.as_deref() == Some(*block))
                .map(|(row_index, _)| *row_index);
            let first_row = block_rows.next()?;
            let last_row = block_rows.fold(first_row, |last, row| last.max(row));
            Some(RowBand {
                first_row,
                last_row,
                first_column: SF2_FIRST_DAY_COLUMN,
                last_column: SF2_LAST_DAY_COLUMN,
            })
        })
        .collect()
}

/// The first row of a roster's female block, or a fresh template's own when the
/// roster has no female learners.
#[must_use]
pub fn female_start_row(learners: &[(u32, Option<String>)]) -> u32 {
    learners
        .iter()
        .filter(|(_, gender_block)| gender_block.as_deref() == Some("FEMALE"))
        .map(|(row_index, _)| *row_index)
        .min()
        .unwrap_or(SF2_FRESH_FEMALE_START_ROW)
}

/// The day number to print in each day column, or `None` for a column the month
/// does not reach.
///
/// A column is left blank when the month has no school day in that
/// (week, weekday) cell - most of a month's first week, and all of its tail. A
/// month needing more days than the form has columns simply has its last days
/// left off the sheet, exactly as the pre-split calendar writer behaved.
#[must_use]
pub fn day_numbers_for_slots(
    report_year: i32,
    report_month: u32,
    first_school_day: u32,
    slots: &[MonthDaySlot],
) -> Vec<(u32, Option<u32>)> {
    let last_day = days_in_month(report_year, report_month);
    let anchor =
        NaiveDate::from_ymd_opt(report_year, report_month, first_school_day).map(monday_anchor_for);

    slots
        .iter()
        .map(|slot| {
            let day = anchor.and_then(|monday_anchor| {
                (first_school_day..=last_day)
                    .filter_map(|day| NaiveDate::from_ymd_opt(report_year, report_month, day))
                    .find(|date| {
                        weekday_index(*date) == Some(slot.weekday_index)
                            && week_number(*date, monday_anchor) == slot.week_index
                    })
            });
            (slot.column, day.map(|date| date.day()))
        })
        .collect()
}

/// The Monday of the week `date` falls in, whether or not `date` is a school day.
///
/// [`day_numbers_for_slots`] counts school weeks *from* this date, so it has to
/// be a Monday even when the first day of the month is not one. A month that
/// opens on a weekend - November 2026 opens on a Sunday, say - has no weekday to
/// step back from, and taking the previous school day first is what keeps that
/// month's grid from coming out **empty**: every one of its day columns would
/// otherwise be blank, and a school term's worth of attendance with nowhere to
/// record it.
fn monday_anchor_for(date: NaiveDate) -> NaiveDate {
    let previous_school_day = match date.weekday() {
        Weekday::Sat => date - Duration::days(1),
        Weekday::Sun => date - Duration::days(2),
        _ => date,
    };
    previous_school_day
        - Duration::days(previous_school_day.weekday().num_days_from_monday().into())
}

/// The school days of a month that the form has no day column for.
///
/// The DepEd SF2 grid is **five weeks of Monday..Friday - 25 labelled day
/// cells** - and the ABSENT/PRESENT block begins in the very next column
/// (`AM`), so a sixth week has nowhere to go. Read off the bundled template's own
/// weekday header (`row 7`, columns `F`..`AL`), the labelled columns are
/// `6, 8, 9, 10, 11 | 12, 14, 15, 16, 17 | 18, 20, 21, 22, 24 | 26, 28, 29, 30,
/// 31 | 32, 33, 35, 36, 37`: columns `7, 13, 19, 23, 27, 34, 38` are the second
/// halves of merged weekday pairs and carry no label, and column `39` is
/// `ABSENT`.
///
/// That is 25 slots for a month that can hold at most **23** Monday-Friday days
/// (a 31-day month starting on a Monday is the maximum), so nothing is ever
/// dropped, and the week index computed against the Monday on or before the first
/// attendance day can never exceed 4. That is why this function returns an empty
/// vector for every real month.
///
/// It exists anyway, and every writer calls it, because "it cannot happen" is the
/// weakest possible guard for the one failure that costs a user a term of marks:
/// a school day that is silently missing from the grid is a day the user cannot
/// record an absence on, and it looks like a holiday nobody took. A non-empty
/// result is a loud log line instead.
#[must_use]
pub fn days_without_a_slot(
    report_year: i32,
    report_month: u32,
    first_school_day: u32,
    slots: &[MonthDaySlot],
) -> Vec<u32> {
    let placed: std::collections::HashSet<u32> =
        day_numbers_for_slots(report_year, report_month, first_school_day, slots)
            .into_iter()
            .filter_map(|(_, day)| day)
            .collect();

    (first_school_day..=days_in_month(report_year, report_month))
        .filter(|day| {
            NaiveDate::from_ymd_opt(report_year, report_month, *day)
                .is_some_and(|date| weekday_index(date).is_some())
        })
        .filter(|day| !placed.contains(day))
        .collect()
}

/// Which week of the form a date falls in, counting from the Monday on or
/// before the first attendance day.
///
/// `u32::MAX` for a date before the anchor, which no slot can match - the anchor
/// is the first attendance day itself or earlier, so this cannot happen in
/// practice, and a wrong day is far worse than no day.
fn week_number(date: NaiveDate, monday_anchor: NaiveDate) -> u32 {
    u32::try_from((date - monday_anchor).num_days() / 7).unwrap_or(u32::MAX)
}

/// The `sf2_month_date_mappings` rows implied by a month sheet's day columns.
///
/// Derived from the same numbers the sheet was written with, so the grid the
/// database records and the grid on the page are the same grid by construction -
/// there is no second read of the workbook that could disagree with the first.
///
/// Each row carries the `sheet_name` it belongs on. Under the one-file
/// twelve-sheet model that is no longer derivable-and-therefore-droppable: it is
/// the column a write path compares against what Excel actually has, and the one
/// that makes "which worksheet is this day on?" answerable by a query.
#[must_use]
pub fn month_date_mappings(
    template_id: &str,
    report_year: i32,
    report_month: u32,
    first_school_day: u32,
    slots: &[MonthDaySlot],
) -> Vec<Sf2MonthDateMapping> {
    let sheet_name =
        crate::sf2::month::workbook_sheets::month_sheet_name(report_month, report_year);
    day_numbers_for_slots(report_year, report_month, first_school_day, slots)
        .into_iter()
        .filter_map(|(column, day)| {
            let day = day?;
            let date = NaiveDate::from_ymd_opt(report_year, report_month, day)?;
            Some(Sf2MonthDateMapping {
                template_id: template_id.to_string(),
                date: date.format("%Y-%m-%d").to_string(),
                column_letter: column_number_to_letter(column as i32),
                column_index: column,
                sheet_name: Some(sheet_name.clone()),
            })
        })
        .collect()
}

/// How many extra roster rows a month file must grow by to hold a source roster.
///
/// Two independent reasons to grow, and both are honoured:
///
/// * the source roster is longer than the form's own capacity (E9), and
/// * the source's female block starts lower than a fresh template's, which
///   means the legacy file was already expanded and its row indices have moved.
///
/// The second reason is not cosmetic: without it, row 30 means a different
/// learner in the two files and every copied X would land on the wrong student.
#[must_use]
pub fn roster_expansion_for(
    male_count: usize,
    female_count: usize,
    source_female_start_row: u32,
) -> (u32, u32) {
    let by_capacity = male_count.saturating_sub(SF2_FRESH_MALE_SLOTS) as u32;
    let by_layout = source_female_start_row.saturating_sub(SF2_FRESH_FEMALE_START_ROW);
    let extra_male = by_capacity.max(by_layout);
    let extra_female = female_count.saturating_sub(SF2_FRESH_FEMALE_SLOTS) as u32;
    (extra_male, extra_female)
}

/// How far the header rows below the roster move when the roster grows.
///
/// `Range.Insert` on an entire row pushes everything below it down, and the
/// adviser and signature blocks sit below the roster. Without this the header
/// would be written over the wrong cells on any expanded workbook.
#[must_use]
pub fn header_row_shift(extra_male: u32, extra_female: u32) -> u32 {
    extra_male + extra_female
}

/// The plain comparison behind a month's verification, with no Excel in it.
///
/// Both counts must match exactly. There is no "close enough": a month that
/// cannot prove it holds the same marks as its source is not written, and that
/// is the whole reason the user can trust the split.
#[must_use]
pub fn verify_month_build(
    expected_x: usize,
    found_x: usize,
    expected_learners: usize,
    found_learners: usize,
) -> MonthVerification {
    if expected_x == found_x && expected_learners == found_learners {
        return MonthVerification::Verified;
    }
    MonthVerification::Mismatch {
        expected_x,
        found_x,
        expected_learners,
        found_learners,
    }
}

/// How many rows one formula may flatten, for a strategy and a sheet name.
///
/// The concatenation form is the reason this exists: its formula carries a fully
/// qualified reference per cell, so a long sheet name eats the budget and a
/// wide band has to be split into more, shorter reads.
#[must_use]
pub fn rows_per_chunk(band: &RowBand, sheet_name: &str, strategy: BlockReadStrategy) -> u32 {
    let rows = band.row_count().max(1) as usize;
    let columns = band.column_count().max(1) as usize;
    match strategy {
        BlockReadStrategy::TextJoin => (MAX_CELLS_PER_READ / columns).clamp(1, rows) as u32,
        BlockReadStrategy::Concatenate => {
            let per_row = (sheet_name.len() + CONCATENATION_CELL_OVERHEAD) * columns;
            (MAX_CONCATENATION_FORMULA_LEN / per_row.max(1)).clamp(1, rows) as u32
        }
    }
}

/// The formula that flattens one chunk of a band into a single string.
#[must_use]
pub fn build_block_read_formula(
    sheet_name: &str,
    band: &RowBand,
    strategy: BlockReadStrategy,
) -> String {
    let quoted = quote_sheet_name(sheet_name);
    match strategy {
        BlockReadStrategy::TextJoin => format!(
            "TEXTJOIN(\"{BLOCK_CELL_SEPARATOR}\",FALSE,{quoted}!{})",
            band_address(band)
        ),
        BlockReadStrategy::Concatenate => {
            // The separator goes *between* cells, never in front of the first
            // one: a leading separator would add an empty token and shift every
            // mark after it by one column.
            let mut formula = String::new();
            for row in band.first_row..=band.last_row {
                for column in band.first_column..=band.last_column {
                    if !formula.is_empty() {
                        formula.push_str("&\"|\"&");
                    }
                    formula.push_str(&quoted);
                    formula.push('!');
                    formula.push_str(&column_number_to_letter(column as i32));
                    formula.push_str(&row.to_string());
                }
            }
            formula
        }
    }
}

/// Split a flattened band back into rows of cells.
///
/// The token count is the check that the read actually worked. A formula Excel
/// could not evaluate comes back as an error variant carrying no string value,
/// and an error must never be mistaken for "this month has no marks".
pub fn parse_block_tokens(
    sheet_name: &str,
    flattened: &str,
    band: &RowBand,
) -> Result<Vec<Vec<String>>> {
    let tokens = flattened.split(BLOCK_CELL_SEPARATOR).collect::<Vec<_>>();
    let expected = band.cell_count();
    if tokens.len() != expected {
        return Err(AppError::Internal(format!(
            "read {} of {expected} cells from '{sheet_name}' rows {}..={}; \
             the sheet layout is not the DepEd SF2 grid this build expects",
            tokens.len(),
            band.first_row,
            band.last_row,
        )));
    }

    let column_count = band.column_count() as usize;
    Ok(tokens
        .chunks(column_count)
        .map(|chunk| chunk.iter().map(|cell| cell.trim().to_string()).collect())
        .collect())
}

/// How many `X` marks a read band holds.
///
/// The same function is applied to the source and to the built month, over the
/// **whole** band rather than column by column. A merged `F:G` day cell reads as
/// `X` in both columns on both files, so a per-column comparison would flag
/// every merged mark as a duplicate; and comparing only primary columns would
/// miss an `X` that only a sub-cell carries. Counting both the same way is the
/// only comparison that cannot pass a month which lost a mark.
#[must_use]
pub fn count_absent_marks(rows: &[Vec<String>]) -> usize {
    rows.iter()
        .flatten()
        .filter(|cell| cell.eq_ignore_ascii_case(SF2_ABSENT_MARK))
        .count()
}

/// The non-empty cells of a read band, as writes against absolute positions.
///
/// Only non-empty cells are written. A present student is a *blank* cell, and
/// the month file is a fresh template whose grid is already blank - writing the
/// blanks would mean ~1,300 COM calls per month to reproduce emptiness, and
/// would risk clearing a neighbouring merged cell on the way.
#[must_use]
pub fn marks_from_band(band: &RowBand, rows: &[Vec<String>]) -> Vec<MonthMarkWrite> {
    let mut marks = Vec::new();
    for (row_offset, row) in rows.iter().enumerate() {
        for (column_offset, cell) in row.iter().enumerate() {
            if cell.is_empty() {
                continue;
            }
            marks.push(MonthMarkWrite {
                row_index: band.first_row + row_offset as u32,
                column_index: band.first_column + column_offset as u32,
                value: cell.clone(),
            });
        }
    }
    marks
}

/// The day columns that carry a weekday label - the writable ones.
///
/// The other columns of a day cell are the second half of a merged pair. They
/// hold no label of their own, and a write to one lands on their pair's primary
/// cell, which is why they are skipped on the way out.
#[must_use]
pub fn primary_day_columns(slots: &[MonthDaySlot]) -> Vec<u32> {
    slots.iter().map(|slot| slot.column).collect()
}

// ── Building the twelve month worksheets ────────────────────────────────────

/// One month of the single-file build.
///
/// The workbook is **saved only if every month verifies**. A month that comes
/// back with the wrong number of marks, or the wrong number of learners, fails
/// the whole build: the file is closed without saving, the file on disk is left
/// exactly as it was, and the caller records which month needs attention. There
/// is no half-written workbook in which September has been rebuilt and October
/// still holds a template's sample roster.
#[derive(Debug, Clone)]
pub struct MonthSheetBuild {
    /// What to write into this month's worksheet.
    pub request: MonthBuildRequest,
    /// Delete every SF2-form worksheet in the file that is not one of the months
    /// being built.
    ///
    /// `true` for the twelve-month build, which is what removes a leftover
    /// `__SF2_HIDDEN_{n}` from the retired hide/rename/clear cycle and the
    /// bundled template's own sample sheets. `false` when only one month is being
    /// added to a workbook that already has its other months - the
    /// "create this month" path - where deleting the other eleven would be the
    /// data loss this whole project exists to prevent.
    pub remove_stale_sheets: bool,
}

/// What the build of one file ended up holding, and whether it may be saved.
#[derive(Debug, Clone)]
pub struct SchoolYearBuildReport {
    /// One entry per month built, in the order they were built.
    pub months: Vec<MonthBuildReport>,
    /// Worksheets removed because they were not one of the built months. Always
    /// empty when the build did not ask for it.
    pub removed_sheets: Vec<String>,
    /// Non-monthly helper sheets the bundled template ships, kept as they are.
    pub kept_helper_sheets: Vec<String>,
    /// Did every month come back exactly as it was written?
    pub verification: MonthVerification,
}

impl SchoolYearBuildReport {
    /// The worksheet names this build wrote, in the order it wrote them.
    #[must_use]
    pub fn sheet_names(&self) -> Vec<String> {
        self.months
            .iter()
            .map(|month| month.sheet_name.clone())
            .collect()
    }

    /// Total `X` marks written across every month of the file.
    #[must_use]
    pub fn total_marks(&self) -> usize {
        self.months.iter().map(|month| month.written_marks).sum()
    }
}

/// Build the month worksheets of one workbook, and save it only if all verify.
///
/// `path` must already hold a workbook with at least two School Form 2
/// worksheets, or one to copy from plus one to copy before - see
/// [`workbook_sheets::prepare_month_sheet`], which is where that is enforced.
///
/// The order is fixed and it is the order the safety depends on:
///
/// 1. **Create every month's worksheet**, each a fresh copy of the bundled
///    template's own SF2 form. Nothing is created before its predecessor exists,
///    so the file is never left with no form sheet to copy from.
/// 2. **Delete** every SF2-form worksheet that is not one of them - the template's
///    `JUNE 2025` .. `OCTOBER 2025` sample sheets, their 36 sample `X` marks, and
///    any `__SF2_HIDDEN_{n}` a previous build left behind. Worksheets that are
///    not SF2 forms are left alone.
/// 3. **Make all twelve visible.** There is no hide path anywhere in this
///    module: a month is selected by reading the database, not by uncovering a
///    tab.
/// 4. **Populate each one**: grow the roster, empty the sheet, write the day
///    numbers across the merged weekday pairs, write the header block, copy the
///    roster, write the `X` marks, and write the ABSENT/PRESENT and TOTAL
///    formulas that make Excel count them.
/// 5. **Read every month back** and compare against what was written.
/// 6. **Save**, or close without saving.
pub fn build_school_year_workbook(
    path: &Path,
    builds: &[MonthSheetBuild],
) -> Result<SchoolYearBuildReport> {
    if builds.is_empty() {
        return Err(AppError::InvalidInput(
            "a workbook build was asked for with no months in it".to_string(),
        ));
    }
    let path = path.to_path_buf();
    let builds = builds.to_vec();
    run_excel_task(move || {
        let mut excel = ExcelSession::new()?;
        let outcome = build_in_session(&excel, &path, &builds);
        let quit = excel.quit();
        match (outcome, quit) {
            (Ok(report), Ok(())) => Ok(report),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    })
}

fn build_in_session(
    excel: &ExcelSession,
    path: &Path,
    builds: &[MonthSheetBuild],
) -> Result<SchoolYearBuildReport> {
    let workbook = excel.open_workbook(path, false)?;
    let outcome = build_open_workbook(excel, &workbook, builds);
    // `Close(false)` discards every in-memory change, which is exactly the
    // rollback this needs for a build that did not verify: the file on disk is
    // still whatever it was, and the preserved copy in `_legacy/` is still the
    // original.
    let save = matches!(&outcome, Ok(report) if report.verification.is_verified());
    let close = workbook.method("Close", vec![ComVariant::bool(save)]);
    match outcome {
        Ok(report) => {
            close?;
            Ok(report)
        }
        Err(error) => {
            let _ = close;
            Err(error)
        }
    }
}

fn build_open_workbook(
    excel: &ExcelSession,
    workbook: &ComObject,
    builds: &[MonthSheetBuild],
) -> Result<SchoolYearBuildReport> {
    // The one worksheet every month sheet is copied from. Read before step 1,
    // because step 1 adds sheets and step 2 removes the ones that are not months -
    // and the donor is one of the sheets step 2 removes.
    let donor = donor_form_sheet(workbook)?;

    // Step 1: every worksheet exists before anything is removed.
    let mut sheets = Vec::with_capacity(builds.len());
    for build in builds {
        let report_month_number = month_number(&build.request.report_month);
        if report_month_number == 0 {
            return Err(AppError::InvalidInput(format!(
                "`{}` is not a month this workbook can hold",
                build.request.report_month
            )));
        }
        let prepared =
            prepare_month_sheet(workbook, report_month_number, build.request.report_year)?;
        sheets.push((prepared, report_month_number, build));
    }

    let wanted = sheets
        .iter()
        .map(|(sheet, _, _)| Ok(sheet.name.clone()))
        .collect::<Result<Vec<_>>>()?;

    // Step 2: all twelve visible. No sheet is ever hidden by this module.
    let prepared = wanted
        .iter()
        .map(|name| PreparedMonthSheet {
            name: name.clone(),
            male_total_row: 0,
            female_total_row: 0,
            combined_total_row: 0,
            last_roster_row: 0,
        })
        .collect::<Vec<_>>();
    make_month_sheets_visible(workbook, &prepared)?;

    // Steps 3 and 4, per month: populate, then read back and compare.
    let mut reports = Vec::with_capacity(builds.len());
    for (index, (_sheet, report_month_number, build)) in sheets.iter().enumerate() {
        let sheet = worksheet_by_name(workbook, &wanted[index])?;
        reports.push(populate_month_sheet(
            excel,
            workbook,
            &donor,
            &sheet,
            *report_month_number,
            &build.request,
        )?);
    }

    // Step 5: everything that is not one of the twelve goes, but only when the
    // caller asked for it. Worksheets that are not SF2 forms - `COMPLETE DAYS`,
    // and anything a school's own workbook carries - are never touched.
    //
    // **This runs last**, after all twelve months are written, because the donor
    // the month sheets were copied from is itself one of the sheets being removed.
    // Removing it first would leave every later month copying from a worksheet
    // that no longer exists - which fails as "object required", long after the
    // damage of a half-built file.
    let remove_stale = builds
        .first()
        .is_some_and(|build| build.remove_stale_sheets);
    let removed_sheets = if remove_stale {
        remove_non_month_form_sheets(workbook, &wanted)?
    } else {
        Vec::new()
    };

    let kept_helper_sheets = helper_sheet_names(workbook)?;
    let verification = combine_verifications(reports.iter().map(|month| &month.verification));

    Ok(SchoolYearBuildReport {
        months: reports,
        removed_sheets,
        kept_helper_sheets,
        verification,
    })
}

/// The one worksheet every month sheet is copied from.
///
/// Chosen by *form*, not by name: a previous build may have renamed it
/// `__SF2_HIDDEN_1`, and a month sheet the previous merge wrote is also a form
/// sheet but is a month this build is about to rewrite. So the first worksheet
/// whose `A1` carries the form's own title wins, and its position in the file is
/// not consulted.
fn donor_form_sheet(workbook: &ComObject) -> Result<ComObject> {
    for entry in sheet_entries(workbook)? {
        if entry.is_sf2_form {
            return worksheet_by_name(workbook, &entry.name);
        }
    }
    Err(AppError::Internal(
        "the workbook has no School Form 2 worksheet, so the twelve month worksheets cannot be \
         built from it"
            .to_string(),
    ))
}

/// Combine the per-month verdicts into the file's verdict.
///
/// `Mismatch` wins over `Verified` in every pairing: a file is only saved when
/// *every* month matched, and this is the function that decides that. The
/// mismatching month's own numbers are carried through so the caller can name
/// the month rather than just saying "something did not match".
fn combine_verifications<'a>(
    verifications: impl Iterator<Item = &'a MonthVerification>,
) -> MonthVerification {
    let mut expected_x = 0usize;
    let mut found_x = 0usize;
    let mut expected_learners = 0usize;
    let mut found_learners = 0usize;
    let mut verified = true;

    for verification in verifications {
        match verification {
            MonthVerification::Verified => {}
            MonthVerification::Mismatch {
                expected_x: e,
                found_x: f,
                expected_learners: el,
                found_learners: fl,
            } => {
                verified = false;
                expected_x = expected_x.saturating_add(*e);
                found_x = found_x.saturating_add(*f);
                expected_learners = expected_learners.saturating_add(*el);
                found_learners = found_learners.saturating_add(*fl);
            }
        }
    }

    if verified {
        MonthVerification::Verified
    } else {
        MonthVerification::Mismatch {
            expected_x,
            found_x,
            expected_learners,
            found_learners,
        }
    }
}

/// Empty one month worksheet and write everything into it.
fn populate_month_sheet(
    excel: &ExcelSession,
    workbook: &ComObject,
    donor: &ComObject,
    sheet: &ComObject,
    report_month_number: u32,
    request: &MonthBuildRequest,
) -> Result<MonthBuildReport> {
    // The form first, so the roster rows being inserted below shift the pasted
    // MALE/FEMALE TOTAL rows and their formulas with them. Growing a blank sheet
    // and pasting afterwards would leave the two in an order that only happens to
    // come out right.
    copy_used_range_over(donor, sheet)?;

    // Grow before clearing: the clear's row numbers are the post-insertion ones.
    // The roster is the same on all twelve sheets, so one expansion decision
    // covers the whole file and a row index means the same learner everywhere.
    let female_start = request
        .source_female_start_row
        .max(SF2_FRESH_FEMALE_START_ROW);
    let male_count = request
        .learners
        .iter()
        .filter(|learner| learner.row_index < female_start)
        .count();
    let female_count = request.learners.len().saturating_sub(male_count);
    let (extra_male, extra_female) = roster_expansion_for(male_count, female_count, female_start);
    grow_roster_rows(sheet, extra_male, extra_female)?;
    let row_shift = header_row_shift(extra_male, extra_female);
    let (male_total_row, female_total_row, combined_total_row) =
        bundled_total_rows(male_count, female_count);

    copy_layout_geometry(donor, sheet)?;
    empty_month_sheet(donor, sheet, male_total_row, female_total_row)?;

    let slots = weekday_slots(sheet)?;
    if slots.is_empty() {
        return Err(AppError::Internal(
            "the SF2 month sheet has no weekday header, so its days cannot be laid out".to_string(),
        ));
    }
    write_day_numbers(sheet, request, report_month_number, &slots)?;
    write_month_header(sheet, request, row_shift)?;
    write_learner_rows(sheet, &request.learners)?;
    write_total_labels(sheet, male_total_row, female_total_row, combined_total_row)?;

    let writable_columns = primary_day_columns(&slots);
    let dates = month_date_mappings(
        &request.template_id,
        request.report_year,
        report_month_number,
        request.first_school_day,
        &slots,
    );
    let marks = resolve_marks(request, &dates, &writable_columns);
    let mut written_marks = 0usize;
    for mark in &marks {
        set_sf2_cell(
            sheet,
            mark.row_index as i32,
            mark.column_index as i32,
            &mark.value,
            true,
        )?;
        written_marks += 1;
    }

    // The ABSENT / PRESENT and TOTAL formulas that make Excel count what was just
    // written. Without them the form shows the marks but every total is blank,
    // and `empty_month_sheet` has just cleared the ones the template shipped.
    let (formula_marks, static_marks) = month_summary_formulas(
        request,
        &dates,
        male_total_row,
        female_total_row,
        combined_total_row,
    );
    for formula in &formula_marks {
        set_sf2_formula(sheet, &formula.cell_address, &formula.value)?;
    }
    for value in &static_marks {
        set_sf2_mark_force(sheet, &value.cell_address, &value.value)?;
    }

    let target_sheet_name = sheet.get_string("Name")?;
    let bands = attendance_bands_for_writes(&request.learners);
    let (found_x, found_learners) = verify_open_workbook(
        excel,
        workbook,
        &target_sheet_name,
        &bands,
        &request.learners,
    )?;
    let verification = verify_month_build(
        request.expected_x_count(),
        found_x,
        request.expected_learner_rows(),
        found_learners,
    );

    Ok(MonthBuildReport {
        sheet_name: target_sheet_name,
        dates,
        written_marks,
        extra_roster_rows: extra_male + extra_female,
        kept_helper_sheets: Vec::new(),
        verification,
    })
}

/// The cells a month's absences land in, given the grid and roster just written.
///
/// An absence the roster does not know, or whose date the grid has no column for,
/// is **dropped and logged** rather than guessed at. Both cases are the shape of
/// the bug this project exists to fix: a mark in a column the `COUNTIF` does not
/// count, or on a row that is not a learner, is a mark Excel shows as absent and
/// the database cannot reproduce - which is exactly what a later guard
/// comparison would then refuse to clear.
#[must_use]
pub fn resolve_marks(
    request: &MonthBuildRequest,
    dates: &[Sf2MonthDateMapping],
    writable_columns: &[u32],
) -> Vec<MonthMarkWrite> {
    let row_by_student = request
        .learners
        .iter()
        .map(|learner| (learner.student_id.as_str(), learner.row_index))
        .collect::<HashMap<_, _>>();
    let column_by_date = dates
        .iter()
        .map(|date| (date.date.as_str(), date.column_index))
        .collect::<HashMap<_, _>>();

    let mut marks = Vec::with_capacity(request.absences.len());
    let mut unmapped_students = 0usize;
    let mut unmapped_dates = 0usize;
    let mut seen = HashSet::new();
    for absence in &request.absences {
        let Some(row_index) = row_by_student.get(absence.student_id.as_str()).copied() else {
            unmapped_students += 1;
            continue;
        };
        let Some(column_index) = column_by_date.get(absence.date.as_str()).copied() else {
            unmapped_dates += 1;
            continue;
        };
        if !writable_columns.contains(&column_index) {
            continue;
        }
        if !seen.insert((row_index, column_index)) {
            continue;
        }
        marks.push(MonthMarkWrite {
            row_index,
            column_index,
            value: SF2_ABSENT_MARK.to_string(),
        });
    }

    if unmapped_students > 0 || unmapped_dates > 0 {
        log::warn!(
            "SF2 month {} {}: {} absence(s) name a learner the roster does not hold and {} fall \
             on a day the grid has no column for. They were not written, and the build will not \
             save: a mark the grid cannot count is a mark Excel shows and the database cannot \
             reproduce.",
            request.report_month,
            request.report_year,
            unmapped_students,
            unmapped_dates
        );
    }
    marks.sort_by_key(|mark| (mark.row_index, mark.column_index));
    marks
}

/// The ABSENT / PRESENT and TOTAL formulas for one month's worksheet.
///
/// Delegates to the two generators the write path already uses, so a formula
/// written by the build and a formula written by a later sync land on the same
/// cells with the same text.
fn month_summary_formulas(
    request: &MonthBuildRequest,
    dates: &[Sf2MonthDateMapping],
    male_total_row: u32,
    female_total_row: u32,
    combined_total_row: u32,
) -> (Vec<Sf2CellMark>, Vec<Sf2CellMark>) {
    let sheet_name = crate::sf2::month::workbook_sheets::month_sheet_name(
        month_number(&request.report_month),
        request.report_year,
    );
    let date_records = dates
        .iter()
        .map(|date| Sf2DateMappingRecord {
            template_id: date.template_id.clone(),
            sheet_name: date.resolved_sheet_name(),
            date: date.date.clone(),
            column_letter: date.column_letter.clone(),
            column_index: date.column_index,
        })
        .collect::<Vec<_>>();
    let student_records = request
        .learners
        .iter()
        .map(|learner| Sf2StudentMappingRecord {
            template_id: request.template_id.clone(),
            student_id: String::new(),
            workbook_name: learner.name.clone(),
            normalized_name: learner.name.clone(),
            row_index: learner.row_index,
            gender_block: learner.gender_block.clone(),
        })
        .collect::<Vec<_>>();

    let male_count = student_records
        .iter()
        .filter(|learner| learner.gender_block.as_deref() == Some("MALE"))
        .count();
    let female_count = student_records.len().saturating_sub(male_count);

    let mut formula_marks = crate::sf2::attendance::attendance_marks::total_formula_marks(
        male_count,
        female_count,
        male_total_row,
        female_total_row,
        combined_total_row,
        &date_records,
    );
    let (absent_present, static_marks) =
        crate::sf2::attendance::attendance_marks::learner_absent_present_formula_marks(
            &student_records,
            male_count,
            female_count,
            date_records.len(),
            male_total_row,
            female_total_row,
            combined_total_row,
            &[sheet_name.as_str()],
        );
    formula_marks.extend(absent_present);
    (formula_marks, static_marks)
}

/// The MALE / FEMALE / Combined TOTAL rows of a roster of this size.
///
/// Delegates to the one place the SF2 layout arithmetic lives
/// ([`crate::sf2::roster::roster_parser::bundled_template_total_rows`]) so a
/// formula written here and a formula written by the roster sync cannot land on
/// different rows.
fn bundled_total_rows(male_count: usize, female_count: usize) -> (u32, u32, u32) {
    crate::sf2::roster::roster_parser::bundled_template_total_rows(male_count, female_count)
}

/// The non-monthly worksheets still in the workbook, for the build report.
fn helper_sheet_names(workbook: &ComObject) -> Result<Vec<String>> {
    Ok(sheet_entries(workbook)?
        .into_iter()
        .filter(|entry| !entry.is_sf2_form)
        .map(|entry| entry.name)
        .collect())
}

/// Write this month's day numbers into the sheet's day row.
fn write_day_numbers(
    sheet: &ComObject,
    request: &MonthBuildRequest,
    report_month_number: u32,
    slots: &[MonthDaySlot],
) -> Result<()> {
    let last_day = days_in_month(request.report_year, report_month_number);
    if request.first_school_day < 1 || request.first_school_day > last_day {
        return Err(AppError::InvalidInput(format!(
            "the first attendance day of {} is not between 1 and {last_day}",
            request.report_month
        )));
    }
    // Every labelled column is written, blank included: a month with no school
    // day in a slot must not inherit the previous occupant's day number, and the
    // cell writer lands on the primary cell of each merged weekday pair.
    for (column, day) in day_numbers_for_slots(
        request.report_year,
        report_month_number,
        request.first_school_day,
        slots,
    ) {
        let value = day.map_or_else(String::new, |day| day.to_string());
        set_sf2_cell(sheet, SF2_DAY_ROW, column as i32, &value, true)?;
    }

    // The form has 25 day columns and a month needs at most 23, so this is empty
    // for every real month. It is checked rather than assumed: a day the grid
    // cannot hold is a day nobody can record an absence on.
    let dropped = days_without_a_slot(
        request.report_year,
        report_month_number,
        request.first_school_day,
        slots,
    );
    if !dropped.is_empty() {
        log::warn!(
            "SF2 month {} {} has {} school day(s) the DepEd form has no column for: {:?}. \
             These days cannot hold an X and cannot be recorded.",
            request.report_month,
            request.report_year,
            dropped.len(),
            dropped
        );
    }
    Ok(())
}

/// Write the header block, from the legacy workbook's own template row.
fn write_month_header(
    sheet: &ComObject,
    request: &MonthBuildRequest,
    row_shift: u32,
) -> Result<()> {
    let header = &request.header;
    let shift = |row: i32| row + row_shift as i32;
    set_sf2_cell(sheet, 3, 6, &header.school_id, true)?;
    set_sf2_cell(sheet, 3, 13, &header.school_year, true)?;
    set_sf2_cell(sheet, 3, 27, &header.report_month, true)?;
    set_sf2_cell(sheet, 4, 6, &header.school_name, true)?;
    set_sf2_cell(sheet, 4, 27, &header.grade_level, true)?;
    set_sf2_cell(sheet, 4, 39, &header.section, true)?;
    set_sf2_cell(
        sheet,
        shift(SF2_ADVISER_ROW),
        40,
        &header.adviser_name,
        true,
    )?;
    set_sf2_cell(
        sheet,
        shift(SF2_ADVISER_ROW),
        26,
        &header.adviser_name,
        true,
    )?;
    set_sf2_cell(
        sheet,
        shift(SF2_SIGNATURE_ROW),
        40,
        &header.school_head_name,
        true,
    )?;
    Ok(())
}

/// Label the three total rows the form's roster is divided by.
///
/// [`empty_month_sheet`] clears the roster block, and that block *includes* the
/// MALE TOTAL, FEMALE TOTAL and Combined TOTAL rows - so the labels go with it.
/// Nothing puts them back unless this does, and a sheet without them is a sheet
/// nothing can read: `workbook_learners` finds the female block by the
/// `FEMALE ... TOTAL` label, `analyze_workbook` finds its day columns the same
/// way, and the guard's bands come from both. A rebuilt month whose totals are
/// unlabelled reads as one block of males running through the female rows, which
/// puts a female learner's mark in a band nothing counts.
///
/// So this is not cosmetic. The labels are how the form says where one block ends
/// and the next begins.
fn write_total_labels(
    sheet: &ComObject,
    male_total_row: u32,
    female_total_row: u32,
    combined_total_row: u32,
) -> Result<()> {
    for (row, label) in [
        (male_total_row, SF2_MALE_TOTAL_LABEL),
        (female_total_row, SF2_FEMALE_TOTAL_LABEL),
        (combined_total_row, SF2_COMBINED_TOTAL_LABEL),
    ] {
        set_sf2_cell(sheet, row as i32, SF2_NAME_COLUMN, label, true)?;
    }
    Ok(())
}

/// Copy the roster into the learner rows: the `No.` cell and the name.
fn write_learner_rows(sheet: &ComObject, learners: &[MonthLearnerWrite]) -> Result<()> {
    for learner in learners {
        set_sf2_cell(
            sheet,
            learner.row_index as i32,
            SF2_ITEM_NUMBER_COLUMN,
            &learner.item_number.to_string(),
            true,
        )?;
        set_sf2_cell(
            sheet,
            learner.row_index as i32,
            SF2_NAME_COLUMN,
            learner.name.trim(),
            true,
        )?;
    }
    Ok(())
}

/// Read the built month back and count what it actually holds.
///
/// The read happens on the still-open workbook, before the save, so a mismatch
/// costs nothing: the file on disk is untouched.
///
/// ## Why `COUNTIF` and not the bulk `TEXTJOIN` read
///
/// [`read_band_from_workbook`] exists to bring a band's **cells** back, because
/// the pre-split comparison needs to know which cell each mark is in. Verification
/// only needs two numbers, and `Application.Evaluate` answers both in one call
/// each:
///
/// * `COUNTIF('SHEET'!F8:AL24,"X")` - the marks on the sheet.
/// * `COUNTA('SHEET'!C8:C49)` - the learner rows that have a name.
///
/// It also gets the **merged-cell** case right, which the cell-by-cell read
/// cannot. The DepEd grid's first day cell is `F{r}:G{r}`, so a mark there is
/// *one* cell holding a value; `COUNTIF` counts it once, and the plain read
/// counted it twice - which would have failed every month whose first day
/// carried an absence, for a month that was entirely correct.
fn verify_open_workbook(
    excel: &ExcelSession,
    workbook: &ComObject,
    sheet_name: &str,
    bands: &[RowBand],
    learners: &[MonthLearnerWrite],
) -> Result<(usize, usize)> {
    let quoted = quote_sheet_name(sheet_name);
    let mut x_count = 0usize;
    for band in bands {
        x_count += evaluate_count(
            excel,
            &format!(
                "COUNTIF({quoted}!{},\"{SF2_ABSENT_MARK}\")",
                band_address(band)
            ),
        )?;
    }

    // Counted band by band rather than as one rectangle from the first learner row
    // to the last: the MALE TOTAL row sits between the two blocks and carries a
    // label, so a single `COUNTA` over the whole span counts a total row as a
    // learner and the build fails for a sheet that is exactly right.
    let mut found_learners = 0usize;
    for band in bands {
        found_learners += evaluate_count(
            excel,
            &format!(
                "COUNTA({quoted}!{}{}:{}{})",
                column_number_to_letter(SF2_NAME_COLUMN),
                band.first_row,
                column_number_to_letter(SF2_NAME_COLUMN),
                band.last_row
            ),
        )?;
    }
    let _ = (workbook, learners);
    Ok((x_count, found_learners))
}

/// One Excel count, as a Rust `usize`.
///
/// `Evaluate` hands a number back, and a number is a variant this layer reads
/// without ambiguity - unlike the string results of the bulk read, which is one
/// more reason verification uses this and not that.
fn evaluate_count(excel: &ExcelSession, formula: &str) -> Result<usize> {
    let evaluated = excel
        .app
        .method("Evaluate", vec![ComVariant::bstr(formula)])?
        .to_i32()
        .map_err(|error| {
            AppError::Internal(format!("Excel could not evaluate `{formula}`: {error}"))
        })?;
    Ok(usize::try_from(evaluated).unwrap_or_default())
}

/// Read one month file's day-column slots, without writing to it.
///
/// The "create this month" path needs the same grid the build computes, and the
/// grid is a property of the sheet's own weekday header - not of the calendar,
/// and not of a constant this file could hardcode. So the header is read out of
/// the file that was just written, opened **read-only** and closed without
/// saving. An `Err` is not fatal to the caller: a created month with no date
/// mappings is the state every write path already refuses to act on, so falling
/// back to it is safe.
pub fn read_day_slots(path: &Path) -> Result<Vec<MonthDaySlot>> {
    let path = path.to_path_buf();
    run_excel_task(move || {
        with_workbook(&path, true, false, |_excel, workbook| {
            for entry in sheet_entries(workbook)? {
                if !entry.is_sf2_form {
                    continue;
                }
                let sheet = worksheet_by_name(workbook, &entry.name)?;
                return weekday_slots(&sheet);
            }
            Err(AppError::Internal(format!(
                "the workbook at {} has no School Form 2 sheet, so its day columns cannot be read",
                path.display()
            )))
        })
    })
}

// ── Reading the legacy workbook ─────────────────────────────────────────────

/// Read every month of a legacy 12-tab workbook, in one read-only Excel session.
///
/// The workbook is opened read-only and closed without saving, so reading the
/// original can never modify it - which is the promise that lets the split keep
/// the file as the fallback authority for any month it cannot prove.
///
/// Only a failure to open or read the workbook at all fails the whole call. A
/// month the legacy file has no sheet for comes back as its own failed
/// [`LegacyMonthRead`], so the caller can abandon that month alone and leave the
/// other eleven untouched.
pub fn read_legacy_months(path: &Path, months: &[(String, i32)]) -> Result<Vec<LegacyMonthRead>> {
    let path = path.to_path_buf();
    let months = months.to_vec();
    run_excel_task(move || {
        with_workbook(&path, true, false, |excel, workbook| {
            let reads = months
                .iter()
                .map(|(report_month, report_year)| {
                    let read = read_legacy_month(excel, workbook, report_month, *report_year);
                    match read {
                        Ok(snapshot) => LegacyMonthRead {
                            report_month: report_month.to_uppercase(),
                            report_year: *report_year,
                            snapshot: Some(snapshot),
                            error: None,
                        },
                        Err(error) => {
                            log::warn!(
                                "workbook split: cannot read {report_month} {report_year} from the \
                                 original workbook: {error}"
                            );
                            LegacyMonthRead {
                                report_month: report_month.to_uppercase(),
                                report_year: *report_year,
                                snapshot: None,
                                error: Some(error.to_string()),
                            }
                        }
                    }
                })
                .collect::<Vec<_>>();
            Ok(reads)
        })
    })
}

fn read_legacy_month(
    excel: &ExcelSession,
    workbook: &ComObject,
    report_month: &str,
    report_year: i32,
) -> Result<LegacyMonthSnapshot> {
    let month = month_number(report_month);
    let sheets = workbook.get_object("Worksheets")?;
    let sheet_count = sheets.get_i32("Count")?;
    let mut candidates = Vec::new();
    for index in 1..=sheet_count {
        let sheet = sheets.get_object_with_args("Item", vec![ComVariant::i4(index)])?;
        let name = sheet.get_string("Name")?;
        if is_month_sheet_for(&name, month) {
            candidates.push((name, sheet));
        }
    }
    // Prefer the sheet whose year is the one being split, so a leftover renamed
    // tab from an earlier school year cannot be read as this month.
    let (sheet_name, sheet) = candidates
        .into_iter()
        .min_by_key(|(name, _)| (year_from_sheet_name(name) != report_year, name.clone()))
        .ok_or_else(|| {
            AppError::InvalidInput(format!(
                "the original workbook has no {} sheet to split from",
                report_month.to_uppercase()
            ))
        })?;

    let learners = workbook_learners(&sheet)?;
    let bands = attendance_bands(&learners);
    let mut marks = Vec::new();
    let mut x_count = 0usize;
    for band in &bands {
        let rows = read_band_from_workbook(excel, &sheet_name, *band)?;
        x_count += count_absent_marks(&rows);
        marks.extend(marks_from_band(band, &rows));
    }

    Ok(LegacyMonthSnapshot {
        sheet_name,
        report_month: report_month.to_string(),
        report_year,
        day_by_column: legacy_day_by_column(&sheet)?,
        female_start_row: source_female_start_row(&learners),
        learners,
        marks,
        x_count,
    })
}

/// The day number each labelled day column of a sheet prints, read from row 6.
///
/// Only the columns the sheet's own weekday header labels are returned, so a
/// merged pair's unlabelled sub-column is absent - which is what keeps a mark
/// there from being read as a second, unmatchable mark.
fn legacy_day_by_column(sheet: &ComObject) -> Result<HashMap<u32, u32>> {
    let slots = weekday_slots(sheet)?;
    let mut days = HashMap::with_capacity(slots.len());
    for slot in &slots {
        let text = cell_text(sheet, SF2_DAY_ROW, slot.column as i32)?;
        if let Ok(day) = text.trim().parse::<u32>() {
            if (1..=31).contains(&day) {
                days.insert(slot.column, day);
            }
        }
    }
    Ok(days)
}

/// The first row of a source roster's female block, from the legacy read.
#[must_use]
pub fn source_female_start_row(learners: &[Sf2WorkbookLearner]) -> u32 {
    female_start_row(&learner_rows(
        learners
            .iter()
            .map(|learner| (learner.row_index, learner.gender_block.clone())),
    ))
}

// ── Bulk range reads ────────────────────────────────────────────────────────

/// Read one band of a sheet with a single Excel evaluation per chunk.
///
/// The formula is evaluated against the *active* workbook, which is the one this
/// session opened immediately before - the same assumption
/// [`crate::backup::x_count`] makes, and safe here because only one workbook is
/// ever open at a time in these paths.
///
/// `TEXTJOIN` is tried first because it is one call for a whole band. If this
/// Excel is too old to have it - or the answer does not come back as a full
/// grid - the `&`-concatenation form is used instead, which works on every
/// version. Only a failure of *both* is an error, and it is an error for the
/// month rather than a silent empty read.
fn read_band_from_workbook(
    excel: &ExcelSession,
    sheet_name: &str,
    band: RowBand,
) -> Result<Vec<Vec<String>>> {
    match read_band_with(excel, sheet_name, band, BlockReadStrategy::TextJoin) {
        Ok(rows) => Ok(rows),
        Err(textjoin_error) => {
            log::debug!(
                "TEXTJOIN read of {sheet_name} rows {}..={} failed ({textjoin_error}); \
                 retrying with the concatenation form",
                band.first_row,
                band.last_row
            );
            read_band_with(excel, sheet_name, band, BlockReadStrategy::Concatenate)
        }
    }
}

fn read_band_with(
    excel: &ExcelSession,
    sheet_name: &str,
    band: RowBand,
    strategy: BlockReadStrategy,
) -> Result<Vec<Vec<String>>> {
    let chunk_rows = rows_per_chunk(&band, sheet_name, strategy);
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut first_row = band.first_row;
    while first_row <= band.last_row {
        let chunk = RowBand {
            first_row,
            last_row: (first_row + chunk_rows - 1).min(band.last_row),
            first_column: band.first_column,
            last_column: band.last_column,
        };
        let formula = build_block_read_formula(sheet_name, &chunk, strategy);
        // `to_string_value` is fallible so an Excel error value cannot arrive
        // here as the empty string and be read, by `parse_block_tokens`' token
        // count, as a band that genuinely holds no marks. Excel does not throw
        // when it cannot evaluate a formula - it hands back an error *variant* -
        // so the failure is named here, where the strategy that produced it is
        // still known, and the caller falls back to the other one.
        let evaluated = excel
            .app
            .method("Evaluate", vec![ComVariant::bstr(formula.as_str())])?
            .to_string_value()
            .map_err(|error| {
                log::warn!(
                    "reading `{sheet_name}` rows {}..={} as one block with {strategy:?} did not \
                     evaluate: {error}",
                    chunk.first_row,
                    chunk.last_row
                );
                error
            })?;
        rows.extend(parse_block_tokens(sheet_name, &evaluated, &chunk)?);
        first_row = chunk.last_row + 1;
    }
    Ok(rows)
}

// ── Small helpers ───────────────────────────────────────────────────────────

/// Quote a sheet name for a formula, doubling embedded single quotes.
fn quote_sheet_name(sheet_name: &str) -> String {
    format!("'{}'", sheet_name.replace('\'', "''"))
}

fn band_address(band: &RowBand) -> String {
    format!(
        "{}{}:{}{}",
        column_number_to_letter(band.first_column as i32),
        band.first_row,
        column_number_to_letter(band.last_column as i32),
        band.last_row
    )
}

fn weekday_index(date: NaiveDate) -> Option<u32> {
    match date.weekday() {
        Weekday::Mon => Some(0),
        Weekday::Tue => Some(1),
        Weekday::Wed => Some(2),
        Weekday::Thu => Some(3),
        Weekday::Fri => Some(4),
        Weekday::Sat | Weekday::Sun => None,
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|first| first.pred_opt())
        .map_or(0, |last| last.day())
}

#[cfg(test)]
#[path = "__tests__/workbook_builder_tests.rs"]
mod tests;
