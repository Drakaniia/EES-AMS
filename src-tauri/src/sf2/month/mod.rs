//! The SF2 month-workbook model (spec §0 A1, A4, D16).
//!
//! **One `.xls` file holding twelve month worksheets**, all visible, each with its
//! own day-number grid and its own X marks (spec §0 A1). The earlier D2/D4 model,
//! twelve files with one worksheet each, was a mis-transcription of the request
//! and is gone; see the amendment at the top of the spec for why, and
//! [`schema_v24`] for the one piece of schema that reversal needed.
//!
//! * [`Sf2MonthTemplateRepo`] - twelve rows for a school year, all pointing at
//!   the one file. A row identifies a month's *grid*, not a file.
//! * [`Sf2MonthDateRepo`] - one month's day-number grid, with the `sheet_name`
//!   that says which of the twelve worksheets each day lives on.
//! * [`Sf2MonthStudentRepo`] - a month's roster, plus the DepEd learner ID that
//!   makes a roster reshuffle survivable.
//! * [`first_school_day`] - the pure D16 derivation and the school-year
//!   year-assignment rule.
//! * [`workbook_builder`] - writes the twelve worksheets into one file and reads
//!   a legacy workbook's marks with one bulk range read per row band.
//! * [`workbook_sheets`] - the worksheet layer: which sheet a month is on, and
//!   how a sheet is emptied of anything the bundled template shipped in it.
//! * [`merge`] - the job that turns one legacy file into the twelve-sheet
//!   workbook, and the "re-run" command.
//! * [`schema_v24`] - the `sheet_name` column and its backfill.

pub mod date_repo;
pub mod first_school_day;
pub mod merge;
pub mod schema_v24;
pub mod student_repo;
pub mod template_repo;
pub mod workbook_builder;
pub mod workbook_sheets;

pub use date_repo::Sf2MonthDateRepo;
pub use first_school_day::{
    derive_first_school_day, effective_first_school_day, is_school_day,
    needs_school_start_date_prompt, report_year_for_school_month, SCHOOL_START_DATE_PROMPT,
    SCHOOL_YEAR_START_MONTH,
};
pub use merge::{
    is_merge_complete, merge_summary_message, needs_attention_labels, verified_count,
    MergeMonthStatus, MergeOutcome, SPLIT_MONTH_COUNT,
};
pub use student_repo::{
    deped_learner_id_from_cells, match_roster_learner, Sf2LearnerMatch, Sf2LearnerMatchKind,
    Sf2MonthStudentRepo,
};
pub use template_repo::Sf2MonthTemplateRepo;
pub use workbook_builder::{
    build_school_year_workbook, day_numbers_for_slots, days_without_a_slot, month_date_mappings,
    read_day_slots, read_legacy_months, MonthBuildRequest, MonthDaySlot, MonthSheetBuild,
    SchoolYearBuildReport,
};
pub use workbook_sheets::{
    is_hidden_sheet_name, is_month_sheet_name, is_month_sheet_of, month_sheet_name, sheet_entries,
    worksheet_by_name, HIDDEN_SHEET_PREFIX, MONTH_SHEET_NAME_MAX,
};

use crate::sf2::calendar::sf2_month_name;
use crate::sf2::calendar::sf2_month_number;
use serde::{Deserialize, Serialize};

/// The twelve months a school year is made of.
///
/// This is the set, not the running order: a school year reads
/// SEPTEMBER -> AUGUST, and each month's calendar year follows from
/// [`first_school_day::report_year_for_school_month`]. Sort by
/// [`Sf2MonthTemplate::school_year_order_key`] to get the running order.
pub const SF2_SCHOOL_YEAR_MONTHS: [u32; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

/// `first_school_day` value meaning "not derived yet".
///
/// A month file that has not been dated yet stores this rather than a guess.
/// It is a real day number that can never occur, so it is distinguishable from
/// any derived or overridden value without an extra column, and the split job
/// replaces it with a derived day.
pub const FIRST_SCHOOL_DAY_UNDETERMINED: u32 = 0;

/// Reject a month workbook analysis that produced no calendar dates before it
/// can delete the mappings the database already holds.
///
/// Same guard, same wording as the pre-split repository: committing an empty
/// date-mapping set deletes every day column for the month, which leaves the
/// reports grid empty and the next workbook write with nothing to write back.
/// The degenerate analysis is a symptom, never an instruction.
pub const EMPTY_DATE_ANALYSIS_MESSAGE: &str =
    "The SF2 workbook produced no calendar dates. The existing mappings were left untouched.";

/// Same guard for a roster that came back empty: committing it would unmap
/// every learner in the month file.
pub const EMPTY_ROSTER_ANALYSIS_MESSAGE: &str =
    "The SF2 workbook produced no learners. The existing mappings were left untouched.";

/// One month workbook file, as stored in `sf2_month_templates`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2MonthTemplate {
    pub id: String,
    pub active_class_id: String,
    /// The school year label, e.g. `2026-2027`.
    pub school_year: String,
    /// Canonical uppercase month name, e.g. `SEPTEMBER`.
    pub report_month: String,
    /// The calendar year this month falls in, which is *not* always the first
    /// year of the school year - see [`first_school_day::report_year_for_school_month`].
    pub report_year: i32,
    pub source_path: String,
    pub source_hash: String,
    pub school_id: Option<String>,
    pub school_name: Option<String>,
    pub grade_level: Option<String>,
    pub section: Option<String>,
    pub adviser_name: Option<String>,
    pub school_head_name: Option<String>,
    /// The effective first attendance day: derived, or the user's override.
    /// [`FIRST_SCHOOL_DAY_UNDETERMINED`] while the month is undated.
    pub first_school_day: u32,
    /// `Some` when `first_school_day` was typed by the user. Re-derivation
    /// only writes while this is `None`, so an override cannot be lost.
    pub first_school_day_override: Option<u32>,
    pub imported_at: i64,
    pub last_synced_at: Option<i64>,
    /// X marks last counted in the file. Meaningless unless
    /// `workbook_scanned_at` is also set - a file that was never scanned is
    /// `Unmeasured`, not empty (spec §9.1, edge case E4).
    pub workbook_x_count: i64,
    pub workbook_scanned_at: Option<i64>,
}

impl Sf2MonthTemplate {
    /// Has this month been dated yet?
    #[must_use]
    pub fn is_first_school_day_known(&self) -> bool {
        self.first_school_day != FIRST_SCHOOL_DAY_UNDETERMINED
    }

    /// Was `first_school_day` typed by the user rather than derived?
    #[must_use]
    pub fn is_first_school_day_overridden(&self) -> bool {
        self.first_school_day_override.is_some()
    }

    /// Has the file behind this row ever been scanned for X marks?
    #[must_use]
    pub fn is_workbook_measured(&self) -> bool {
        self.workbook_scanned_at.is_some()
    }

    /// Sort key that reads a school year the way it runs:
    /// SEPTEMBER -> DECEMBER of the start year, then JANUARY -> AUGUST of the
    /// next. Alphabetical month order would be misleading.
    #[must_use]
    pub fn school_year_order_key(&self) -> (i32, u32) {
        (
            self.report_year,
            sf2_month_number(&self.report_month).unwrap_or_default(),
        )
    }
}

/// One day column of one month, as stored in `sf2_month_date_mappings`.
///
/// ## The sheet is a real column again
///
/// v21 dropped `sheet_name` because the workbook was one file per month, so the
/// worksheet was derivable from the month row. Under §0 A1 the file holds
/// twelve worksheets, so the derivation is still true but is no longer the only
/// place the answer lives - and a write path has to be able to read the stored
/// sheet back and compare it against what Excel has (spec §0 A4, and
/// [`schema_v24`] for the migration that puts the column back).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2MonthDateMapping {
    pub template_id: String,
    /// `YYYY-MM-DD`.
    pub date: String,
    pub column_letter: String,
    pub column_index: u32,
    /// The worksheet this day is written to, e.g. `SEPTEMBER 2026`.
    ///
    /// Nullable in the database and optional here, because a row whose `template_id`
    /// has no month row is an orphan and the migration refuses to invent a name
    /// for it. [`Self::resolved_sheet_name`] is what every reader uses instead of
    /// the field directly: it falls back to the name the date itself implies,
    /// which is the same string, so a `None` here costs a branch and not
    /// correctness.
    pub sheet_name: Option<String>,
}

impl Sf2MonthDateMapping {
    /// The worksheet this day belongs on, derived from the date when the stored
    /// column is absent.
    ///
    /// A date is a full `YYYY-MM-DD`, so its month and year are always readable
    /// from it. That makes the derivation total, which is what lets the read path
    /// trust the column and lets the migration leave a row it cannot name alone
    /// instead of writing a guess.
    #[must_use]
    pub fn resolved_sheet_name(&self) -> String {
        if let Some(name) = self
            .sheet_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            return name.to_string();
        }
        sheet_name_from_date(&self.date).unwrap_or_default()
    }

    /// [`Self::resolved_sheet_name`], borrowed where a borrow is possible.
    #[must_use]
    pub fn sheet_name_or<'a>(&'a self, fallback: &'a str) -> &'a str {
        self.sheet_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(fallback)
    }
}

/// The worksheet name a `YYYY-MM-DD` date implies: `SEPTEMBER 2026`.
///
/// `None` for anything that is not an ISO date, which is the honest answer for
/// a row the migration could not name either.
#[must_use]
pub fn sheet_name_from_date(date: &str) -> Option<String> {
    let date = date.trim();
    if date.len() != 10 || !date.is_ascii() {
        return None;
    }
    let bytes = date.as_bytes();
    let digits = |range: std::ops::Range<usize>| {
        range.into_iter().all(|index| bytes[index].is_ascii_digit())
    };
    if !digits(0..4) || bytes[4] != b'-' || !digits(5..7) || bytes[7] != b'-' || !digits(8..10) {
        return None;
    }
    let month: u32 = date[5..7].parse().ok()?;
    let year: i32 = date[0..4].parse().ok()?;
    let name = sf2_month_name(month);
    if name.is_empty() {
        return None;
    }
    Some(format!("{name} {year}"))
}

/// One learner row of one month file, as stored in
/// `sf2_month_student_mappings`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2MonthStudentMapping {
    pub template_id: String,
    pub student_id: String,
    pub workbook_name: String,
    pub normalized_name: String,
    pub row_index: u32,
    pub gender_block: Option<String>,
    /// The DepEd learner ID read from the workbook, when the workbook had one
    /// to give (spec §6.3). `None` for a workbook whose learner-ID cell is the
    /// merged "No." cell - see [`deped_learner_id_from_cells`].
    pub sf2_learner_id: Option<String>,
}

/// One row of the Settings -> Month workbooks list: the stored month plus what
/// the file on disk actually looks like right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2MonthPreview {
    /// Canonical uppercase month name, e.g. `SEPTEMBER`.
    pub month: String,
    pub report_year: i32,
    pub school_year: String,
    /// The file name this month resolves to, e.g. `SF2-SEPTEMBER-2026.xls`.
    pub file_name: String,
    /// Is that file present? A `true` here with a missing file is edge case E4:
    /// every write path has to refuse and the guard stays `Unmeasured`.
    pub file_exists: bool,
    pub has_template: bool,
    pub first_school_day: u32,
    pub first_school_day_overridden: bool,
    pub workbook_x_count: i64,
    pub workbook_scanned_at: Option<i64>,
    pub last_synced_at: Option<i64>,
    pub learner_count: usize,
    pub mapped_date_count: usize,
}

impl Sf2MonthPreview {
    /// Build the list row for a month that has a stored template row.
    #[must_use]
    pub fn from_template(
        template: &Sf2MonthTemplate,
        file_name: &str,
        file_exists: bool,
        learner_count: usize,
        mapped_date_count: usize,
    ) -> Self {
        Self {
            month: template.report_month.clone(),
            report_year: template.report_year,
            school_year: template.school_year.clone(),
            file_name: file_name.to_string(),
            file_exists,
            has_template: true,
            first_school_day: template.first_school_day,
            first_school_day_overridden: template.is_first_school_day_overridden(),
            workbook_x_count: template.workbook_x_count,
            workbook_scanned_at: template.workbook_scanned_at,
            last_synced_at: template.last_synced_at,
            learner_count,
            mapped_date_count,
        }
    }

    /// Build the list row for a month that has no stored row yet - the normal
    /// state for every month of a school year before the split has run.
    #[must_use]
    pub fn without_template(
        month: &str,
        report_year: i32,
        school_year: &str,
        file_name: &str,
        file_exists: bool,
    ) -> Self {
        Self {
            month: month.to_string(),
            report_year,
            school_year: school_year.to_string(),
            file_name: file_name.to_string(),
            file_exists,
            has_template: false,
            first_school_day: FIRST_SCHOOL_DAY_UNDETERMINED,
            first_school_day_overridden: false,
            workbook_x_count: 0,
            workbook_scanned_at: None,
            last_synced_at: None,
            learner_count: 0,
            mapped_date_count: 0,
        }
    }
}
