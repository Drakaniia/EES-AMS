//! The shape of the mark diagnostic, and the rules that turn measurements into a
//! verdict.
//!
//! Everything here is data. The one behavioural rule - what a month's status is
//! allowed to claim - lives in [`month_for`], because that is where the
//! difference between "the workbook holds no marks" and "the workbook could not
//! be read" is decided, and getting it wrong is the one failure this whole
//! module exists to prevent: **a false zero.**
//!
//! ## Why `Option` is everywhere
//!
//! [`MarkCounts`] and [`MonthMarkComparison`] hold `Option` on every measured
//! number rather than defaulting to `0`. A count of zero is a claim - "I looked
//! and there is nothing" - and this module is only allowed to make that claim
//! when it actually looked. Every other state is a refusal:
//!
//! | state | meaning |
//! |---|---|
//! | `NoSheet` | the workbook has no worksheet for this month |
//! | `NoMappings` | the worksheet exists but nothing on it can be resolved to a day or a learner |
//! | `ExcelUnavailable` | Excel could not be reached, or the user has the file open |
//! | `WorkbookMissing` | the file the database points at is not on disk |
//!
//! Any of those is [`MarkSourceStatus::Incomparable`], and an incomparable month
//! makes the whole verdict incomparable. The `.xls` is the only known copy of
//! this user's marks (spec §0 A5); answering "the database is complete" for a
//! month nobody managed to read is how that copy gets cleared.

use serde::{Deserialize, Serialize};

/// One absence the database holds: which child, on which local day.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbsentRecord {
    pub student_id: String,
    pub class_id: Option<String>,
    /// `YYYY-MM-DD`, already local.
    pub date: String,
}

/// One learner row of a workbook, as the database believes it to be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterRow {
    pub student_id: String,
    pub workbook_name: String,
    pub row_index: u32,
}

/// One learner row of a workbook, as the database believes it to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MappingSource {
    /// `sf2_month_student_mappings` / `sf2_month_date_mappings` (spec §6.2).
    PerMonthTables,
    /// `sf2_student_mappings` / `sf2_date_mappings` - the pre-split model.
    LegacyTables,
    /// Neither table has a row for this month. Nothing can be scoped.
    None,
}

impl MappingSource {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PerMonthTables => "perMonthTables",
            Self::LegacyTables => "legacyTables",
            Self::None => "none",
        }
    }
}

impl std::fmt::Display for MappingSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// How a month's roster rows were decided.
///
/// The three are not interchangeable, and the difference decides whether a
/// difference found in a month means anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RosterResolution {
    /// The database's own row mappings, used because the sheet is on the
    /// workbook the database's template row points at.
    DatabaseRowMappings,
    /// The worksheet's own `NAME` column matched against the database's
    /// students, used because the sheet is on some *other* file. A row index
    /// from one workbook means nothing in another, so a name match is the only
    /// honest way to line them up.
    WorkbookNameMatch,
    /// Nothing could place a learner on a row.
    Unresolved,
}

impl RosterResolution {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DatabaseRowMappings => "databaseRowMappings",
            Self::WorkbookNameMatch => "workbookNameMatch",
            Self::Unresolved => "unresolved",
        }
    }
}

impl std::fmt::Display for RosterResolution {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One cell of the attendance block, in the vocabulary the guard and the writer
/// share.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkCell {
    /// The child's name, as the workbook spells it.
    pub student_name: String,
    /// `YYYY-MM-DD`, or a sentence saying why the cell has no date.
    pub date: String,
    /// The worksheet the cell is on.
    pub sheet_name: String,
    /// The A1 address, e.g. `AL47`.
    pub cell_address: String,
}

/// Could this month be measured at all?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkSourceStatus {
    /// The database holds at least what the workbook shows, on the same cells.
    Comparable,
    /// The database has no template row and no file for this month.
    WorkbookMissing,
    /// Excel could not be reached, or the user has the workbook open.
    ExcelUnavailable,
    /// The workbook has no worksheet for this month.
    NoSheet,
    /// A worksheet exists, but no cell on it can be resolved to a day or a
    /// learner, so nothing on it can be compared.
    NoMappings,
}

impl MarkSourceStatus {
    /// Is this month's measurement usable?
    #[must_use]
    pub fn is_comparable(self) -> bool {
        matches!(self, Self::Comparable)
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Comparable => "Comparable",
            Self::WorkbookMissing => "WorkbookMissing",
            Self::ExcelUnavailable => "ExcelUnavailable",
            Self::NoSheet => "NoSheet",
            Self::NoMappings => "NoMappings",
        }
    }
}

impl std::fmt::Display for MarkSourceStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The measured numbers for one month.
///
/// Every count is an `Option`, and `None` means *not measured*, never *zero*.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkCounts {
    /// `X` cells the workbook holds, in scope: the mapped learner rows and the
    /// mapped day columns of this month's sheet.
    pub workbook_x_count: Option<usize>,
    /// `absent` events the database holds whose local date falls in this month,
    /// for the class this workbook is for. Not scoped to the grid, so it can
    /// exceed [`Self::db_mapped_absent_count`].
    pub db_absent_count: Option<usize>,
    /// The subset of [`Self::db_absent_count`] the grid can actually hold: an
    /// absence on a day the month has no column for, or for a learner with no
    /// roster row, is in the first count and not in this one.
    pub db_mapped_absent_count: Option<usize>,
    /// Learner rows x day columns actually inspected. `None` when nothing was.
    pub cells_scanned: Option<usize>,
    /// What
    /// [`measure_workbook_marks`](crate::sf2::guard::evaluate::measure_workbook_marks),
    /// the bulk range read the destructive-sync guard clears the grid on the
    /// strength of, returned for the same month and the same scope.
    ///
    /// Reported beside [`Self::workbook_x_count`] rather than instead of it,
    /// because on this project's Excel the two disagree: the guard's row-mask
    /// formula is a 33-term `TEXTJOIN`, and `Application.Evaluate` answers that
    /// with an error variant, so the guard reports zero X cells for a month that
    /// demonstrably has them. That is a false zero on the safety-critical path,
    /// and a diagnostic that hid it would be hiding the one fact worth knowing.
    pub guard_x_count: Option<usize>,
    /// Do the guard's count and this diagnostic's count agree?
    ///
    /// `false` means the guard cannot currently see the marks the workbook
    /// holds, so a write path evaluating it would be cleared to proceed on a
    /// count of zero.
    pub guard_agrees: Option<bool>,
}

impl MarkCounts {
    /// What the diagnostic found for a month it could not read.
    #[must_use]
    pub fn unmeasured() -> Self {
        Self::default()
    }
}

/// One month of the comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthMarkComparison {
    /// Canonical uppercase month name, e.g. `SEPTEMBER`.
    pub report_month: String,
    pub report_year: i32,
    pub counts: MarkCounts,
    pub cells_only_in_workbook: Vec<MarkCell>,
    pub cells_only_in_database: Vec<MarkCell>,
    pub source_status: MarkSourceStatus,
    /// Why the status is what it is, in a sentence a teacher can read.
    pub reason: String,
    /// The worksheet this month was measured on, when there was one.
    pub sheet_name: Option<String>,
    /// The file that worksheet is on. Two months measured on the same file is
    /// the normal case under spec §0 A1; two different files is worth seeing.
    pub workbook_path: Option<String>,
    pub mapping_source: MappingSource,
    /// How the roster rows used for this month were decided.
    pub roster_resolution: RosterResolution,
    pub roster_rows: usize,
    pub day_columns: usize,
}

impl Default for MonthMarkComparison {
    fn default() -> Self {
        Self {
            report_month: String::new(),
            report_year: 0,
            counts: MarkCounts::unmeasured(),
            cells_only_in_workbook: Vec::new(),
            cells_only_in_database: Vec::new(),
            source_status: MarkSourceStatus::NoSheet,
            reason: String::new(),
            sheet_name: None,
            workbook_path: None,
            mapping_source: MappingSource::None,
            roster_resolution: RosterResolution::Unresolved,
            roster_rows: 0,
            day_columns: 0,
        }
    }
}

impl MonthMarkComparison {
    /// A month that could not be measured, and why.
    #[must_use]
    pub fn unmeasured(
        report_month: &str,
        report_year: i32,
        status: MarkSourceStatus,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            report_month: report_month.to_string(),
            report_year,
            source_status: status,
            reason: reason.into(),
            ..Self::default()
        }
    }
}

/// What the twelve months add up to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkVerdict {
    /// The workbook holds X marks the database has no record of. **The `.xls` is
    /// the copy that must be preserved**; nothing may be written until the
    /// difference has been imported (spec §0 A5).
    WorkbookIsSourceOfTruth,
    /// Every month was measurable and the database held at least every mark the
    /// workbook showed, on the same cells.
    DatabaseIsSourceOfTruth,
    /// At least one month could not be measured. Which one, and why, is in
    /// [`Sf2MarkDiagnostic::incomparable_months`]. No conclusion is drawn.
    Incomparable,
}

impl MarkVerdict {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorkbookIsSourceOfTruth => "WorkbookIsSourceOfTruth",
            Self::DatabaseIsSourceOfTruth => "DatabaseIsSourceOfTruth",
            Self::Incomparable => "Incomparable",
        }
    }

    /// May a write path act on this verdict?
    ///
    /// `false` for everything except [`Self::DatabaseIsSourceOfTruth`], and
    /// even that one only clears the *comparison* - the destructive-sync guard
    /// has to run per month before anything is actually cleared (spec §9.1).
    #[must_use]
    pub fn permits_write(self) -> bool {
        matches!(self, Self::DatabaseIsSourceOfTruth)
    }
}

impl std::fmt::Display for MarkVerdict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One worksheet the diagnostic found but could not place in a month.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnplacedSheet {
    pub workbook_path: String,
    pub sheet_name: String,
    pub visible: bool,
    /// `X` cells on the sheet, over the whole day-column block.
    pub unrowed_x_count: usize,
    /// Why the sheet could not be compared against a month.
    pub reason: String,
    /// Every `X` on the sheet, named by the sheet's own roster, with the cell
    /// address. Empty when the sheet holds none.
    pub marks: Vec<MarkCell>,
}

/// One `.xls` in the workbook directory, and what was found on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbookFileReport {
    pub path: String,
    /// Is this the file the database's template row points at?
    pub is_referenced_by_database: bool,
    pub sheet_count: usize,
    pub month_sheets_measured: usize,
    /// Total `X` cells over every sheet in the file, day-column block only.
    pub total_x_count: usize,
    /// Set when the file could not be opened at all.
    pub read_error: Option<String>,
}

/// Row counts of one table, or `None` when the table does not exist.
///
/// `None` is not `0`: a database at schema v18 has no per-month tables, and
/// reporting those as empty would say "the backfill copied nothing" when the
/// truth is "the backfill cannot have run yet".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRowCount {
    pub table: String,
    pub exists: bool,
    pub rows: Option<i64>,
}

/// The legacy and per-month mapping tables, as they actually stand.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingTableState {
    pub legacy: Vec<TableRowCount>,
    pub per_month: Vec<TableRowCount>,
    /// The legacy `sf2_date_mappings` grouped by the sheet it names.
    pub legacy_date_mapping_sheets: Vec<SheetDayGridSummary>,
    /// The per-month `sf2_month_date_mappings` grouped by the month it covers.
    pub month_date_mapping_grids: Vec<SheetDayGridSummary>,
}

/// One stored grid, summarised by what it claims to cover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetDayGridSummary {
    /// The worksheet, for the legacy table. Empty for the per-month table,
    /// which has no `sheet_name` column (spec §0 A4).
    pub sheet_name: String,
    /// `YYYY-MM`, derived from the mapping rows' own dates.
    pub year_month: String,
    pub first_date: String,
    pub last_date: String,
    pub day_columns: usize,
}

/// The whole answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2MarkDiagnostic {
    /// Unix seconds, when the diagnostic ran.
    pub generated_at: i64,
    pub database_path: String,
    /// `PRAGMA user_version` of that database.
    pub schema_version: Option<i32>,
    /// The workbook file the database's template row points at.
    pub workbook_path: Option<String>,
    /// The directory every `.xls` was looked for in.
    pub workbook_dir: String,
    pub active_class_id: Option<String>,
    pub school_year: Option<String>,
    /// The month the database's own template row names.
    pub stored_report_month: Option<String>,
    /// Where the roster and grid the comparison used came from.
    pub mapping_source: MappingSource,
    pub months: Vec<MonthMarkComparison>,
    pub verdict: MarkVerdict,
    /// One sentence, in the vocabulary above, saying what the verdict rests on.
    pub verdict_reason: String,
    /// `"{MONTH} {year}: {status} - {reason}"` for every month that could not
    /// be measured. Never empty when the verdict is
    /// [`MarkVerdict::Incomparable`] and there was at least one month to try.
    pub incomparable_months: Vec<String>,
    /// Worksheets that hold `X` cells and cannot be tied to a month. This is the
    /// list that keeps a "the database is complete" answer honest.
    pub unplaced_sheets: Vec<UnplacedSheet>,
    pub workbooks: Vec<WorkbookFileReport>,
    pub tables: MappingTableState,
    /// `events` rows per event type, so a total can be checked against the
    /// per-month figures.
    pub event_counts: Vec<EventTypeCount>,
    /// Total `absent` events in the database.
    pub total_absent_events: i64,
    /// Absences that belong to no class this diagnostic knows about - either
    /// recorded against a different `class_id`, or against none. Nothing will
    /// ever place these on a grid, so they are counted separately rather than
    /// inflating a month's number.
    pub absent_events_without_class: i64,
    /// The baseline recorded in the app's `db-fingerprint.json` sidecar, when
    /// one could be read. Its absent count is what the next update compares
    /// against (spec §9.4), so a fall is worth seeing next to the live count.
    pub recorded_fingerprint: Option<crate::backup::fingerprint::DbFingerprint>,
    /// Set when the live absent count is *lower* than the recorded one:
    /// attendance records went missing between versions.
    pub absent_decreased_since_fingerprint: bool,
    /// The §9.4 decrease notice, when there is one to show.
    pub fingerprint_decrease_notice: Option<String>,
    /// `false` when any measured month had
    /// [`MarkCounts::guard_agrees`] as `false`: the destructive-sync guard's own
    /// read disagrees with a direct read of the same cells.
    ///
    /// `true` is the only value that leaves the guard's behaviour unquestioned.
    /// `None` means no month was measured, so nothing was compared.
    pub guard_scan_matches_direct_read: Option<bool>,
}

/// `events` rows for one event type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTypeCount {
    pub event_type: String,
    pub rows: i64,
}

/// What the twelve months say.
///
/// Three rules, in this order, and the order is the point:
///
/// 1. **Any month the workbook holds marks the database lacks wins.** Not
///    because it is the most likely answer - because it is the one that must
///    never be lost to a nicer-sounding verdict.
/// 2. `DatabaseIsSourceOfTruth` requires **all** months to be
///    [`MarkSourceStatus::Comparable`]. A school year with eleven months that
///    could not be read is not a school year the database has been shown to
///    cover.
/// 3. Anything else is [`MarkVerdict::Incomparable`], and the caller is told
///    which months and why. No guess, no default-to-safe.
#[must_use]
pub fn verdict_for(months: &[MonthMarkComparison]) -> MarkVerdict {
    if months
        .iter()
        .any(|month| !month.cells_only_in_workbook.is_empty())
    {
        return MarkVerdict::WorkbookIsSourceOfTruth;
    }
    if months.is_empty() {
        return MarkVerdict::Incomparable;
    }
    if months
        .iter()
        .all(|month| month.source_status.is_comparable())
    {
        return MarkVerdict::DatabaseIsSourceOfTruth;
    }
    MarkVerdict::Incomparable
}

/// The sentence that goes with a [`MarkVerdict`].
#[must_use]
pub fn verdict_reason(
    verdict: MarkVerdict,
    months: &[MonthMarkComparison],
    unplaced_x_cells: usize,
) -> String {
    match verdict {
        MarkVerdict::WorkbookIsSourceOfTruth => {
            let missing: usize = months
                .iter()
                .map(|month| month.cells_only_in_workbook.len())
                .sum();
            format!(
                "{missing} X mark(s) in the workbook have no record in the database. The .xls is \
                 the only copy of those marks: import them before anything is written."
            )
        }
        MarkVerdict::DatabaseIsSourceOfTruth => format!(
            "All {} months were measurable and the database holds every mark the workbook shows, \
             on the same cells.",
            months.len()
        ),
        MarkVerdict::Incomparable => {
            let unmeasured: Vec<String> = months
                .iter()
                .filter(|month| !month.source_status.is_comparable())
                .map(|month| {
                    format!(
                        "{} {} ({})",
                        month.report_month,
                        month.report_year,
                        month.source_status.as_str()
                    )
                })
                .collect();
            let mut reason = if unmeasured.is_empty() {
                "No month could be measured.".to_string()
            } else {
                format!(
                    "{} of {} month(s) could not be measured: {}. No conclusion is drawn.",
                    unmeasured.len(),
                    months.len(),
                    unmeasured.join(", ")
                )
            };
            if unplaced_x_cells > 0 {
                reason.push_str(&format!(
                    " A further {unplaced_x_cells} X cell(s) sit on worksheet(s) that cannot be \
                     tied to any month, so they were not compared either."
                ));
            }
            reason
        }
    }
}

/// `"{MONTH} {year}: {status} - {reason}"`, for the months a reader must be
/// told about.
#[must_use]
pub fn incomparable_summary(month: &MonthMarkComparison) -> String {
    format!(
        "{} {}: {} - {}",
        month.report_month,
        month.report_year,
        month.source_status.as_str(),
        month.reason
    )
}
