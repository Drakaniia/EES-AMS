//! The instant month switch (spec D9, D10, §7).
//!
//! Everything the Reports grid needs for one month, read from SQLite in one
//! pass. No Excel, no COM, no mutation, no progress events. This is the only
//! thing a month switch is allowed to call.
//!
//! ## Why this can be a read at all
//!
//! A month file holds exactly one worksheet, so `sf2_month_date_mappings`
//! dropped `sheet_name` (spec §6.2): the sheet is derivable from
//! `sf2_month_templates.report_month` + `report_year`, and the month is
//! derivable from the row the caller asked for. There is no sheet to resolve
//! and no "which month is this class on?" question, so there is nothing to
//! write. The thousand COM round-trips `set_report_month` used to spend
//! renaming eleven hidden tabs are gone because the eleven tabs are gone.
//!
//! ## The June / September year wrap
//!
//! Two year-assignment rules exist in this crate and they disagree:
//!
//! * `sf2_report_year` (`crate::sf2::calendar`) wraps at **June** and belongs to
//!   the legacy one-workbook-per-class model.
//! * [`report_year_for_school_month`] wraps at **September** and belongs to the
//!   per-month model, because a Philippine school year is SEPTEMBER -> AUGUST.
//!
//! For months 1-7 and 9-12 the two agree. They differ only for AUGUST, where
//! the legacy rule says the *start* year and the school-year rule says the
//! *following* year - and `SF2-AUGUST-2026.xls` and `SF2-AUGUST-2027.xls` are
//! different files, so "they usually agree" is not good enough.
//!
//! The reconciliation is that **this path never recomputes the year for a month
//! that has a row.** [`month_preview`] reads `report_year` off
//! `sf2_month_templates`, a stored column written under the September rule - by
//! `migrate_to_v22.sql` for backfilled installs and by the split for new ones.
//! The pure September function is used only for a month with no row yet, where
//! there is no stored answer to disagree with. The legacy June rule is not
//! consulted at all: it only describes a workbook the split retires, and
//! letting it near this path is exactly how a month file would end up filed
//! under the wrong year.
//!
//! ## Why this module does not compute the day-number grid
//!
//! §6.2 wants every month's grid populated eagerly, at workbook-creation time, so
//! that no month ever needs a COM round-trip to become usable. That is right, and
//! it belongs to `month::workbook_builder`: the day-number columns are merged
//! weekday pairs whose positions are read out of row 7 of the real sheet
//! (`excel::excel_com::calendar::sf2_weekday_slots`), and the bundled template's
//! layout is not something to re-derive from arithmetic in the app. A month that
//! has not been written yet reports [`Sf2MonthGridPreview::grid_empty`] instead,
//! which is the state every write path already refuses to act on.
//!
//! ## Where the mappings come from, and why that matters
//!
//! A grid cell is editable because a *mapping* exists for it: a date mapping puts
//! the day in a workbook column, a student mapping puts the learner in a workbook
//! row. A cell with no mapping is disabled, and an unmapped student row cannot
//! show an X at all - the preview builder hard-codes those cells to `Present`.
//!
//! So a read that resolves mappings from tables which happen to be empty does not
//! render an empty grid, it renders a grid that *lies*: the database holds the
//! absences, the grid insists nobody was ever marked absent, and every cell is
//! disabled so the teacher cannot correct it. That is the reported bug, and it is
//! a read bug, not a data bug.
//!
//! The per-month tables are the answer going forward, but they are only populated
//! for months that have been created or split, and the split is an on-demand
//! one-time job. [`resolve_legacy_mappings`] therefore reads the pre-split tables
//! for whatever the per-month tables cannot answer: the roster, which is
//! class-wide, and the day-number grid, which is **scoped to the requested month
//! as a closed date range**. The scoping is the load-bearing part - see that
//! function's docs. Nothing on this path writes, and the pre-split tables are
//! never deleted, because on the affected install they are still where some of
//! the data lives.

use crate::domain::error::{AppError, Result};
use crate::domain::models::{AttendanceType, Student};
use crate::infrastructure::database::{
    ClassRepository, DbPool, EventRepository, StudentRepository,
};
use crate::sf2::calendar::{last_day_of_month, sf2_month_name, sf2_month_number};
use crate::sf2::models::{
    Sf2ExportReadiness, Sf2PreviewAbsence, Sf2PreviewDate, Sf2PreviewStudentRow,
    Sf2StudentMappingRecord, Sf2TemplateRecord, Sf2TemplateSummary,
};
use crate::sf2::month::first_school_day::{
    current_year, derive_first_school_day, report_year_for_school_month,
};
use crate::sf2::month::{
    Sf2MonthDateMapping, Sf2MonthDateRepo, Sf2MonthStudentMapping, Sf2MonthStudentRepo,
    Sf2MonthTemplate, Sf2MonthTemplateRepo, FIRST_SCHOOL_DAY_UNDETERMINED,
};
use crate::sf2::preview;
use crate::sf2::repository::Sf2Repository;
use crate::sf2::workbook_files::{
    hash_bytes, month_workbook_sheet_name, school_year_month_files, BUNDLED_TEMPLATE_BYTES,
};
use chrono::{Datelike, NaiveDate};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const SCHOOL_CALENDAR_SETTINGS_SQL: &str =
    include_str!("../sql/month_school_calendar_settings.sql");
const LATEST_SCHOOL_YEAR_SQL: &str = include_str!("../sql/month_latest_school_year.sql");

/// Shown when a month is asked for that no name can resolve. A constant so the
/// command, the tests and the frontend cannot disagree on the wording.
pub const UNKNOWN_MONTH_MESSAGE: &str = "Report month must be a valid month name";

/// Shown when a month has no school days at all (spec edge case E2: April, May,
/// summer). A file is never created for such a month, because there is no day
/// in it for the file to record.
pub const NO_SCHOOL_DAYS_MESSAGE: &str =
    "This month has no school days, so no SF2 workbook is created for it.";

/// Everything the Reports page needs to paint one month (spec §7.1).
///
/// The grid half - `dates`, `students`, `absent_list`, the counts, the issues -
/// is the shape `get_sf2_export_preview` already returns, so the grid, the
/// sidebar and the absent list read it unchanged. The month half says which
/// file the grid belongs to, which is the part a per-month model can answer and
/// a single workbook with a mutable `report_month` could not.
///
/// No `PartialEq`: the grid half is made of the shared `Sf2Preview*` models,
/// which derive `Debug`/`Clone`/`Serialize` only. Comparing two previews is not
/// a thing anything needs to do - a month switch replaces one, it does not test
/// whether it changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2MonthGridPreview {
    /// Canonical uppercase month name, e.g. `SEPTEMBER`.
    pub month: String,
    /// The calendar year this month falls in. Read off the month row, never
    /// recomputed - see the module docs on the year wrap.
    pub report_year: i32,
    /// The school year label, e.g. `2026-2027`.
    pub school_year: String,
    pub class_id: String,
    pub class_name: String,
    /// The month's one worksheet, `"{MONTH} {year}"`. Empty when the month has
    /// no row, because there is then no file and no sheet to name.
    pub sheet_name: String,
    /// `SF2-SEPTEMBER-2026.xls`.
    pub file_name: String,
    /// A stored row whose file is gone is edge case E4: every write path has to
    /// refuse and the guard stays `Unmeasured`. The read still succeeds, so the
    /// grid can show what the database holds while the file is restored.
    pub file_exists: bool,
    pub has_template: bool,
    /// [`FIRST_SCHOOL_DAY_UNDETERMINED`] while the month is undated.
    pub first_school_day: u32,
    /// False for a month that ends before classes started, and for a month whose
    /// start date has not been entered yet. Never a guessed day.
    pub has_school_days: bool,
    /// No day columns are mapped, so nothing may be written to this month yet.
    pub grid_empty: bool,
    /// At least one of the two mapping sets came from the pre-split
    /// `sf2_date_mappings` / `sf2_student_mappings` rather than from this month's
    /// own per-month row.
    ///
    /// This is the normal state on an install whose work have not been split yet,
    /// and it is a *read* state only: nothing here is written, and the tables the
    /// mappings came from are never modified. It is surfaced so the Reports page
    /// can say where the grid is being drawn from instead of implying the month
    /// has no data - which is what a teacher reads when their marks are on screen
    /// and the sidebar says the workbook is missing.
    pub uses_legacy_mappings: bool,
    /// X marks last counted in the file, and when they were counted. This is the
    /// guard's own comparison, surfaced so the Reports sidebar can show it as
    /// information rather than as a recovery prompt (spec §12.2). Meaningless
    /// unless `workbook_scanned_at` is also set: a file nobody has counted is
    /// *unmeasured*, not empty, and nothing may be cleared on its word.
    pub workbook_x_count: i64,
    pub workbook_scanned_at: Option<i64>,
    /// When attendance was last written to this month's file.
    pub last_synced_at: Option<i64>,
    /// The month's workbook identity, in the shape the Reports page has always
    /// read for a template. Every month worksheet of a class carries the same
    /// school, grade, section and adviser, so the sidebar's identity panel is fed
    /// from here and never has to reach for a different month's row.
    pub template: Option<Sf2TemplateSummary>,
    /// Every Monday-Friday of the month, in date order.
    pub dates: Vec<Sf2PreviewDate>,
    pub students: Vec<Sf2PreviewStudentRow>,
    pub absent_list: Vec<Sf2PreviewAbsence>,
    pub mapped_students: usize,
    pub mapped_dates: usize,
    pub present_count: usize,
    pub absence_count: usize,
    pub unmapped_student_count: usize,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

/// Every Monday-Friday of the month, in date order.
///
/// A day the month's grid has a column for carries that column; a day it does
/// not is still listed, with a blank column, because the grid shows every
/// weekday the school had whether or not the workbook got as far as writing a
/// number there. This mirrors the pre-split `expand_to_all_weekdays` so the
/// rendered grid is identical whichever read produced it.
///
/// The worksheet each day belongs on is its **own** `sheet_name`, not the sheet
/// of the month being read. Under §0 A1 there are twelve worksheets in one file
/// and a column letter is not an address without a sheet, so this is the value a
/// click resolves against. A day with no stored name falls back to the month
/// being read, which is the same string for a healthy install.
#[must_use]
pub fn expand_to_month_weekdays(
    report_year: i32,
    month_number: u32,
    sheet_name: &str,
    mappings: &[Sf2MonthDateMapping],
) -> Vec<Sf2PreviewDate> {
    let mapping_by_date: HashMap<&str, &Sf2MonthDateMapping> =
        mappings.iter().map(|m| (m.date.as_str(), m)).collect();
    let last_day = last_day_of_month(report_year, month_number);
    let mut dates = Vec::with_capacity(last_day as usize);

    for day in 1..=last_day {
        let Some(date) = NaiveDate::from_ymd_opt(report_year, month_number, day) else {
            continue;
        };
        if date.weekday().number_from_monday() > 5 {
            continue;
        }
        let date_str = date.format("%Y-%m-%d").to_string();
        let (column_letter, column_index, day_sheet) = match mapping_by_date.get(date_str.as_str())
        {
            Some(mapping) => {
                let resolved = mapping.resolved_sheet_name();
                let sheet = if resolved.is_empty() {
                    sheet_name.to_string()
                } else {
                    resolved
                };
                (mapping.column_letter.clone(), mapping.column_index, sheet)
            }
            None => (String::new(), 0, sheet_name.to_string()),
        };
        dates.push(Sf2PreviewDate {
            date: date_str,
            sheet_name: day_sheet,
            column_letter,
            column_index,
        });
    }

    dates
}

/// The one workbook on disk, and whether it is there.
///
/// ## How it is resolved, in order
///
/// 1. The month's own stored row. After the merge every month names the same
///    file, so this is the answer for all twelve.
/// 2. The pre-split `sf2_templates` row for the class. That is the file the
///    merge rebuilds, and it is what an install has before the merge has run.
/// 3. The name the merge would use, derived from the pre-split row's identity.
///
/// A retired per-month file (`SF2-SEPTEMBER-2026.xls`) is **not** consulted. It
/// is not where a month is recorded any more, and answering "is this month's
/// workbook there?" from one would tell a teacher their marks are in a file the
/// app no longer writes to.
fn resolve_workbook_on_disk(
    pool: &DbPool,
    workbook_dir: &Path,
    class_id: &str,
    template: &Option<Sf2MonthTemplate>,
) -> Result<WorkbookOnDisk2> {
    if let Some(row) = template.as_ref() {
        let path = PathBuf::from(&row.source_path);
        return Ok(WorkbookOnDisk2 {
            file_name: path.file_name().map_or_else(
                || row.source_path.clone(),
                |name| name.to_string_lossy().to_string(),
            ),
            exists: path.is_file(),
        });
    }
    if let Some(legacy) = Sf2Repository::new(pool.clone()).latest_template_for_class(class_id)? {
        let path = PathBuf::from(&legacy.source_path);
        let file_name = path.file_name().map_or_else(
            || legacy.source_path.clone(),
            |name| name.to_string_lossy().to_string(),
        );
        return Ok(WorkbookOnDisk2 {
            file_name,
            exists: path.is_file(),
        });
    }
    // No identity on record at all, so there is no workbook name to report and no
    // workbook to find. Not the existence of the *directory*: an empty directory
    // is not a workbook, and reporting one as the workbook would make every D5
    // fallback believe the month it wants is already on disk.
    let _ = workbook_dir;
    Ok(WorkbookOnDisk2 {
        file_name: String::new(),
        exists: false,
    })
}

/// The one workbook's name, and whether it is on disk.
struct WorkbookOnDisk2 {
    file_name: String,
    exists: bool,
}

/// Which month the app opens on launch, and whether the app may offer to create
/// it (spec D5, acceptance #12, edge cases E1 and E2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2LaunchMonth {
    /// The month to open, already resolved through the D5 fallback.
    pub month: String,
    pub report_year: i32,
    pub school_year: String,
    pub class_id: String,
    pub file_name: String,
    pub file_exists: bool,
    pub has_template: bool,
    /// Does the resolved month contain any school day at all? False for April
    /// and May and for summer (edge case E2), and false while
    /// `school_start_date` is unset, because then no month can be dated. Never
    /// a guess in either direction.
    pub has_school_days: bool,
    /// Today's calendar month, before the fallback. The toast names this one:
    /// *"Showing MONTH. Create TODAY'S MONTH to switch."*
    pub today_month: String,
    pub today_report_year: i32,
    /// Today's month has no file and the fallback was used, so the user is being
    /// shown a month they did not ask for and is told which one to create.
    pub fell_back: bool,
    /// A create may be offered for the month the app opened on: it has school days
    /// and no file. False for a month with no school days (E2), and false for a
    /// month whose file is already there, which is not a situation to create
    /// anything into.
    pub can_create: bool,
    /// Whether a create may be offered for **today's** month, which is a
    /// different question from [`Self::can_create`] whenever the D5 fallback ran.
    ///
    /// This is the E1 case: the app is showing MAY because JUNE has no file, and
    /// the thing to offer is "Create JUNE to switch". Asking only
    /// [`Self::can_create`] would answer about MAY - the month already on record,
    /// where a create is the wrong suggestion.
    pub today_can_create: bool,
    /// True while `school_start_date` has not been entered (edge case E3). The
    /// caller prompts once; it is never a reason to guess a day.
    pub needs_school_start_date: bool,
    pub issues: Vec<String>,
}

/// The two v22 settings the per-month model reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchoolCalendarSettings {
    pub school_start_date: Option<String>,
    pub last_report_month: Option<String>,
}

/// Read `school_start_date` and `last_report_month`.
///
/// Both stay `None` when unset. A missing settings row is not an error: it reads
/// as two `None`s, which is the "we do not know yet" state the callers above it
/// are already written to handle.
pub fn school_calendar_settings(pool: &DbPool) -> Result<SchoolCalendarSettings> {
    let conn = pool.get()?;
    let settings = conn
        .query_row(SCHOOL_CALENDAR_SETTINGS_SQL, [], |row| {
            Ok(SchoolCalendarSettings {
                school_start_date: row.get(0)?,
                last_report_month: row.get(1)?,
            })
        })
        .optional()?;
    Ok(settings.unwrap_or_default())
}

/// The newest school year a class actually has month rows for.
///
/// `None` for a class with no rows at all; the caller then falls back to the
/// legacy settings label, and after that to a year derived from the clock.
fn latest_school_year(pool: &DbPool, class_id: &str) -> Result<Option<String>> {
    let conn = pool.get()?;
    conn.query_row(LATEST_SCHOOL_YEAR_SQL, [class_id], |row| {
        row.get::<_, String>(0)
    })
    .optional()
    .map_err(Into::into)
}

/// The class a month read is for: the one asked for, else the only one on record.
fn resolve_class_id(pool: &DbPool, class_id: Option<&str>) -> Result<String> {
    if let Some(class_id) = class_id.map(str::trim).filter(|id| !id.is_empty()) {
        return Ok(class_id.to_string());
    }
    ClassRepository::new(pool.clone())
        .list()?
        .into_iter()
        .next()
        .map(|class| class.id)
        .ok_or_else(|| {
            AppError::InvalidInput("No class is set up yet. Add a class first.".to_string())
        })
}

/// The canonical uppercase name of a month and its number, or an error naming
/// what was wrong. `Sept.`, `sept` and `SEPTEMBER` all resolve to `SEPTEMBER`.
fn canonical_month(report_month: &str) -> Result<(String, u32)> {
    let month_number = sf2_month_number(report_month)
        .ok_or_else(|| AppError::InvalidInput(UNKNOWN_MONTH_MESSAGE.to_string()))?;
    Ok((sf2_month_name(month_number).to_string(), month_number))
}

/// The school year a read is for: the one asked for, else the newest on record,
/// else the legacy settings label, else a year derived from the clock.
fn resolve_school_year(pool: &DbPool, class_id: &str, school_year: Option<&str>) -> Result<String> {
    if let Some(school_year) = school_year.map(str::trim).filter(|year| !year.is_empty()) {
        return Ok(school_year.to_string());
    }
    if let Some(school_year) = latest_school_year(pool, class_id)? {
        return Ok(school_year);
    }
    let legacy = crate::infrastructure::database::SettingsRepository::new(pool.clone())
        .get()?
        .school_year
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    if !legacy.is_empty() {
        return Ok(legacy);
    }
    let year = current_year();
    Ok(format!("{year}-{year}"))
}

/// Read one month from SQL. The whole month switch (spec §7.1).
///
/// `workbook_dir` is consulted for exactly one question - "does this month's
/// file exist?" - answered by a single `stat`. Nothing here opens a workbook,
/// writes a workbook, or emits an `sf2-progress` event.
pub fn month_preview(
    pool: &DbPool,
    workbook_dir: &Path,
    class_id: Option<&str>,
    school_year: Option<&str>,
    report_month: &str,
) -> Result<Sf2MonthGridPreview> {
    let (month, month_number) = canonical_month(report_month)?;
    let class_id = resolve_class_id(pool, class_id)?;
    let school_year = resolve_school_year(pool, &class_id, school_year)?;
    let school_start_date = school_calendar_settings(pool)?
        .school_start_date
        .as_deref()
        .and_then(parse_date);

    let template_repo = Sf2MonthTemplateRepo::new(pool.clone());
    let template = template_repo.find(&class_id, &school_year, &month)?;

    // The stored year wins. See the module docs: recomputing it here is how a
    // month file and a legacy template row would come to disagree about AUGUST.
    let report_year = match template.as_ref() {
        Some(template) => template.report_year,
        None => report_year_for_school_month(&school_year, month_number, current_year()),
    };

    // §0 A1: the file is **one** workbook with twelve month worksheets. So the
    // file name and its existence are the same answer for every month, and what
    // differs per month is the worksheet. A month whose worksheet is missing is
    // edge case E4 for that month alone; a missing *file* is the whole class.
    let workbook = resolve_workbook_on_disk(pool, workbook_dir, &class_id, &template)?;
    let file_name = workbook.file_name.clone();
    let file_exists = workbook.exists;
    let sheet_name = match template.as_ref() {
        Some(_) => month_workbook_sheet_name(&month, report_year),
        None => String::new(),
    };

    // The month tables answer on their own once a month is split, so the pre-split
    // tables are only read when they cannot - which keeps a fully populated month
    // at exactly the query count it costs today (spec D9, acceptance #8).
    //
    // The legacy template row is carried out of the same call because it is needed
    // for the same condition: a month with no row of its own has to be described
    // by something, and under the §0 A1 layout the pre-split row describes the same
    // single file.
    let (mappings, roster, legacy_template, first_school_day) = match template.as_ref() {
        Some(template) => {
            let month_dates = Sf2MonthDateRepo::new(pool.clone()).for_template(&template.id)?;
            let month_roster = Sf2MonthStudentRepo::new(pool.clone()).for_template(&template.id)?;
            if !month_dates.is_empty() && !month_roster.is_empty() {
                (month_dates, month_roster, None, template.first_school_day)
            } else {
                let legacy = resolve_legacy_mappings(
                    pool,
                    &class_id,
                    month_dates,
                    month_roster,
                    report_year,
                    month_number,
                )?;
                (
                    legacy.dates,
                    legacy.roster,
                    legacy.legacy_template,
                    template.first_school_day,
                )
            }
        }
        None => {
            let legacy = resolve_legacy_mappings(
                pool,
                &class_id,
                Vec::new(),
                Vec::new(),
                report_year,
                month_number,
            )?;
            (
                legacy.dates,
                legacy.roster,
                legacy.legacy_template,
                FIRST_SCHOOL_DAY_UNDETERMINED,
            )
        }
    };
    let uses_legacy_mappings = legacy_template.is_some();

    // Each date carries the worksheet it is written to, taken from its own
    // `sheet_name` column rather than from the month being read. They are the
    // same string for every day of a month, so a day whose stored name disagrees
    // is logged and the month wins - but the column is what a write path reads
    // back, so a mismatch is worth saying out loud.
    let dates = expand_to_month_weekdays(report_year, month_number, &sheet_name, &mappings);
    let class_students = StudentRepository::new(pool.clone()).list_by_class(Some(&class_id))?;
    let class_name = ClassRepository::new(pool.clone())
        .get(&class_id)?
        .map(|class| class.name)
        .unwrap_or_default();

    let events = match (dates.first(), dates.last()) {
        (Some(first), Some(last)) => EventRepository::new(pool.clone())
            .list_for_class_and_date_range(&class_id, &first.date, &last.date)?,
        _ => Vec::new(),
    };

    let issues = month_issues(
        &month,
        template.is_some(),
        &file_exists,
        &mappings,
        &roster,
        &class_students,
        uses_legacy_mappings,
    );
    let readiness = Sf2ExportReadiness {
        template: None,
        mapped_students: roster.len(),
        mapped_dates: mappings.len(),
        can_export: issues.is_empty(),
        issues,
        warnings: Vec::new(),
    };

    // Reuse the preview builder rather than reimplementing cell status: an X in
    // the grid has to mean the same thing whichever read produced the grid, and a
    // second implementation of `preview_cell_status` is how that stops holding.
    let shim = identity_record(
        template.as_ref(),
        legacy_template.as_ref(),
        &class_id,
        &school_year,
        &month,
    );
    let preview = preview::export_preview(
        &shim,
        &roster_as_legacy_mappings(&roster),
        &dates,
        &class_name,
        &class_students,
        &events,
        readiness,
    )?;

    let mut warnings = preview.warnings;
    if uses_legacy_mappings {
        // Said once, in the warnings the sidebar already renders, so a teacher
        // whose grid is being served from the pre-split tables knows their marks
        // are on screen and from where - rather than concluding from a blank
        // identity panel that they are gone.
        warnings.push(format!(
            "{month} is drawn from the SF2 mappings recorded before per-month workbooks existed. \
             Your marks are shown and editable; creating this month's workbook moves it onto the \
             per-month tables."
        ));
    }

    let template_summary = template
        .as_ref()
        .map(template_summary_from_month)
        .or_else(|| {
            legacy_template
                .as_ref()
                .map(|legacy| template_summary_from_legacy(legacy, &month))
        });

    Ok(Sf2MonthGridPreview {
        month,
        report_year,
        school_year,
        class_id,
        class_name,
        sheet_name,
        file_name,
        file_exists,
        has_template: template.is_some(),
        first_school_day,
        has_school_days: derive_first_school_day(school_start_date, month_number, report_year)
            .is_some(),
        grid_empty: mappings.is_empty(),
        workbook_x_count: template.as_ref().map_or(0, |row| row.workbook_x_count),
        workbook_scanned_at: template.as_ref().and_then(|row| row.workbook_scanned_at),
        last_synced_at: template.as_ref().and_then(|row| row.last_synced_at),
        template: template_summary,
        uses_legacy_mappings,
        dates: preview.dates,
        students: preview.students,
        absent_list: preview.absent_list,
        mapped_students: preview.mapped_students,
        mapped_dates: preview.mapped_dates,
        present_count: preview.present_count,
        absence_count: preview.absence_count,
        unmapped_student_count: preview.unmapped_student_count,
        issues: preview.issues,
        warnings,
    })
}

/// The mappings one month is drawn from when its own tables cannot answer, and
/// where they came from.
struct LegacyMappings {
    /// Day-number columns for the requested month and no other.
    dates: Vec<Sf2MonthDateMapping>,
    /// The class roster. One shared roster, not twelve (spec §0 A4) - the legacy
    /// `sf2_student_mappings` is already keyed that way, which is why reusing it
    /// is the lower-risk half of this fallback.
    roster: Vec<Sf2MonthStudentMapping>,
    /// The pre-split template row, so a month with no row of its own can still be
    /// described. `None` when the class has no pre-split row at all.
    legacy_template: Option<Sf2TemplateRecord>,
}

/// Serve a month from the pre-split tables, for the mappings its own tables
/// cannot supply.
///
/// ## Why this exists
///
/// The per-month tables are only populated for months that have been created or
/// split, and the split is a one-time job that runs on demand. On an install
/// where it has never run, those tables are empty for eleven or twelve of the
/// twelve months, and the grid's cells are all disabled because they are derived
/// from mappings that do not exist. The teacher's marks are in the database the
/// whole time - in the pre-split tables - so the grid was reporting an empty
/// database rather than an empty month.
///
/// ## What it will not do
///
/// It writes nothing. The pre-split tables are read and never modified, and they
/// are not deleted: on the affected install they are the only place some of the
/// data lives.
///
/// ## The two halves are scoped differently, on purpose
///
/// The roster is class-wide, because there is one class and one roster
/// (`PRIMARY KEY(template_id, student_id)` over a shared row set in the legacy
/// table, and spec §0 A4 keeps the per-month model that way too). It has no month in
/// it, so there is nothing to narrow.
///
/// The day-number grid *is* per month, and the legacy table is keyed by a full
/// `YYYY-MM-DD` with no month of its own. So the month is applied as a closed
/// date range over the requested year. That is the whole point of this function:
/// a September read must be physically unable to receive October's columns, and
/// filtering in Rust after reading the whole table is exactly the version of that
/// bug where the grid claims days it has no mapping for.
///
/// A legacy row from a *different* year is therefore not shown either. It belongs
/// to a different school year's grid, and re-anchoring it onto this one is a
/// guess about which of two stored answers the teacher meant.
fn resolve_legacy_mappings(
    pool: &DbPool,
    class_id: &str,
    month_dates: Vec<Sf2MonthDateMapping>,
    month_roster: Vec<Sf2MonthStudentMapping>,
    report_year: i32,
    month_number: u32,
) -> Result<LegacyMappings> {
    let expected_sheet = month_workbook_sheet_name(sf2_month_name(month_number), report_year);
    let repository = Sf2Repository::new(pool.clone());
    let legacy_template = repository.latest_template_for_class(class_id)?;
    let Some(legacy) = legacy_template.as_ref() else {
        // Nothing pre-split on record either. Both sets stay empty, which is the
        // honest answer: there are no mappings for this month anywhere.
        return Ok(LegacyMappings {
            dates: month_dates,
            roster: month_roster,
            legacy_template: None,
        });
    };

    let dates = if month_dates.is_empty() {
        let start = month_range_bound(report_year, month_number, 1);
        let end = month_range_bound(
            report_year,
            month_number,
            last_day_of_month(report_year, month_number),
        );
        repository
            .date_mappings_in_month(&legacy.id, &start, &end)?
            .into_iter()
            .map(|mapping| Sf2MonthDateMapping {
                template_id: legacy.id.clone(),
                date: mapping.date,
                column_letter: mapping.column_letter,
                column_index: mapping.column_index,
                // The pre-split table recorded the sheet each day was on, and it
                // is the only record of it - but it is not trusted. Under the old
                // hide/rename/clear cycle the visible sheet was whichever month
                // happened to be current, which is not necessarily this month, so
                // the month being read is the authority and the stored name is
                // kept only for comparison. A disagreement is worth surfacing.
                sheet_name: Some(mapping.sheet_name),
            })
            .inspect(|mapping| {
                if mapping.resolved_sheet_name() != expected_sheet {
                    log::warn!(
                        "SF2 {expected_sheet}: the pre-split mappings record this day on `{}`. \
                         The month being read wins; the stored name is a leftover of the \
                         hide/rename/clear cycle and is not what the app writes to.",
                        mapping.resolved_sheet_name()
                    );
                }
            })
            .collect()
    } else {
        month_dates
    };

    let roster = if month_roster.is_empty() {
        repository
            .student_mappings_for_template(&legacy.id)?
            .into_iter()
            .map(|mapping| Sf2MonthStudentMapping {
                template_id: mapping.template_id,
                student_id: mapping.student_id,
                workbook_name: mapping.workbook_name,
                normalized_name: mapping.normalized_name,
                row_index: mapping.row_index,
                gender_block: mapping.gender_block,
                // The pre-split table never read the DepEd ID out of a workbook,
                // so there is none to copy. `None` says "the school did not tell
                // us", which is what the column means; inventing one would be
                // worse than not having it.
                sf2_learner_id: None,
            })
            .collect()
    } else {
        month_roster
    };

    Ok(LegacyMappings {
        dates,
        roster,
        legacy_template,
    })
}

/// `YYYY-MM-DD` for one day of a month, used as an inclusive range bound.
///
/// Fixed-width ISO-8601, so a `>=` / `<=` comparison against it is chronological
/// without any date arithmetic in SQL.
fn month_range_bound(report_year: i32, month_number: u32, day: u32) -> String {
    format!("{report_year:04}-{month_number:02}-{day:02}")
}

/// The workbook identity a month is described by.
///
/// A month with its own row is described by it. A month that has none yet is
/// described by the pre-split row, because under the §0 A1 layout there is one
/// file holding all twelve sheets - so its school, grade, section and adviser are
/// the identity of the workbook that holds this month's marks too, and blanking
/// them would disable the Open SF2 and Sync roster buttons for a month whose data
/// is on screen.
///
/// Only `report_month` is rewritten, to the month actually being read. The
/// pre-split row carries whichever month was last written to it, and putting that
/// name on a grid full of another month's days is the same lie in a different
/// field.
fn identity_record(
    template: Option<&Sf2MonthTemplate>,
    legacy: Option<&Sf2TemplateRecord>,
    class_id: &str,
    school_year: &str,
    month: &str,
) -> Sf2TemplateRecord {
    let from_legacy = |legacy: &Sf2TemplateRecord| Sf2TemplateRecord {
        id: legacy.id.clone(),
        source_path: legacy.source_path.clone(),
        source_hash: String::new(),
        school_id: meta_text(Some(legacy.school_id.as_str())),
        school_name: meta_text(Some(legacy.school_name.as_str())),
        school_year: school_year.to_string(),
        report_month: month.to_string(),
        grade_level: meta_text(Some(legacy.grade_level.as_str())),
        section: meta_text(Some(legacy.section.as_str())),
        adviser_name: meta_text(Some(legacy.adviser_name.as_str())),
        school_head_name: meta_text(Some(legacy.school_head_name.as_str())),
        layout_fingerprint: String::new(),
        active_class_id: class_id.to_string(),
        imported_at: legacy.imported_at,
        last_synced_at: legacy.last_synced_at,
    };

    match (template, legacy) {
        (Some(template), _) => Sf2TemplateRecord {
            id: template.id.clone(),
            source_path: template.source_path.clone(),
            source_hash: String::new(),
            school_id: meta_text(template.school_id.as_deref()),
            school_name: meta_text(template.school_name.as_deref()),
            school_year: school_year.to_string(),
            report_month: month.to_string(),
            grade_level: meta_text(template.grade_level.as_deref()),
            section: meta_text(template.section.as_deref()),
            adviser_name: meta_text(template.adviser_name.as_deref()),
            school_head_name: meta_text(template.school_head_name.as_deref()),
            layout_fingerprint: String::new(),
            active_class_id: class_id.to_string(),
            imported_at: template.imported_at,
            last_synced_at: template.last_synced_at,
        },
        (None, Some(legacy)) => from_legacy(legacy),
        (None, None) => Sf2TemplateRecord {
            id: String::new(),
            source_path: String::new(),
            source_hash: String::new(),
            school_id: String::new(),
            school_name: String::new(),
            school_year: school_year.to_string(),
            report_month: month.to_string(),
            grade_level: String::new(),
            section: String::new(),
            adviser_name: String::new(),
            school_head_name: String::new(),
            layout_fingerprint: String::new(),
            active_class_id: class_id.to_string(),
            imported_at: 0,
            last_synced_at: None,
        },
    }
}

/// The pre-split row as the template summary the Reports page reads, with the
/// month rewritten to the one on screen.
#[must_use]
pub fn template_summary_from_legacy(legacy: &Sf2TemplateRecord, month: &str) -> Sf2TemplateSummary {
    Sf2TemplateSummary {
        id: legacy.id.clone(),
        source_path: legacy.source_path.clone(),
        school_id: meta_text(Some(legacy.school_id.as_str())),
        school_name: meta_text(Some(legacy.school_name.as_str())),
        school_year: legacy.school_year.clone(),
        report_month: month.to_string(),
        grade_level: meta_text(Some(legacy.grade_level.as_str())),
        section: meta_text(Some(legacy.section.as_str())),
        adviser_name: meta_text(Some(legacy.adviser_name.as_str())),
        school_head_name: meta_text(Some(legacy.school_head_name.as_str())),
        class_id: legacy.active_class_id.clone(),
        imported_at: legacy.imported_at,
    }
}

/// Resolve the month the app opens on launch (spec D5, acceptance #12).
///
/// Today's calendar month, falling back to `settings.last_report_month` when
/// today's month has no file. The fallback is never silent: `fell_back` is set
/// and both months are named, so a teacher who opens the app in June and lands
/// on May is told why rather than left to guess.
///
/// `today` is a parameter rather than a clock read so the rule is testable and
/// the caller decides what "today" means.
pub fn launch_month(
    pool: &DbPool,
    workbook_dir: &Path,
    class_id: Option<&str>,
    today: NaiveDate,
) -> Result<Sf2LaunchMonth> {
    let class_id = resolve_class_id(pool, class_id)?;
    let school_year = resolve_school_year(pool, &class_id, None)?;
    let settings = school_calendar_settings(pool)?;
    let school_start_date = settings.school_start_date.as_deref().and_then(parse_date);

    let today_month = sf2_month_name(today.month()).to_string();
    let today_number = today.month();
    let today_report_year =
        report_year_for_school_month(&school_year, today_number, current_year());

    let describe = |month: &str, report_year: i32| -> Result<MonthOnDisk> {
        let month_number = sf2_month_number(month).unwrap_or(today_number);
        let row = Sf2MonthTemplateRepo::new(pool.clone()).find(&class_id, &school_year, month)?;
        // §0 A1: the file is shared, so "is the workbook there?" is one question
        // for the whole year, and a month is on record when its worksheet is.
        let workbook = resolve_workbook_on_disk(pool, workbook_dir, &class_id, &row)?;
        Ok(MonthOnDisk {
            file_exists: workbook.exists,
            has_template: row.is_some(),
            has_school_days: derive_first_school_day(school_start_date, month_number, report_year)
                .is_some(),
            file_name: workbook.file_name,
        })
    };

    let today_state = describe(&today_month, today_report_year)?;
    let mut issues = Vec::new();
    let mut chosen = MonthChoice {
        month: today_month.clone(),
        report_year: today_report_year,
        fell_back: false,
    };

    if !today_state.is_on_record() {
        chosen.fell_back = true;
        let last = settings
            .last_report_month
            .as_deref()
            .and_then(sf2_month_number)
            .map(sf2_month_name)
            .filter(|name| !name.is_empty())
            .map(str::to_string);
        if let Some(last) = last {
            let last_report_year = report_year_for_school_month(
                &school_year,
                sf2_month_number(&last).unwrap_or(today_number),
                current_year(),
            );
            if describe(&last, last_report_year)?.is_on_record() {
                chosen.month = last;
                chosen.report_year = last_report_year;
            }
        }
        if chosen.month == today_month {
            issues.push(format!(
                "No SF2 workbook exists for {today_month} or for the last month you used."
            ));
        }
    }

    let state = describe(&chosen.month, chosen.report_year)?;
    if !state.has_school_days && !state.is_on_record() {
        issues.push(NO_SCHOOL_DAYS_MESSAGE.to_string());
    }

    Ok(Sf2LaunchMonth {
        file_name: state.file_name.clone(),
        month: chosen.month,
        report_year: chosen.report_year,
        school_year,
        class_id,
        file_exists: state.file_exists,
        has_template: state.has_template,
        has_school_days: state.has_school_days,
        today_month,
        today_report_year,
        fell_back: chosen.fell_back,
        // Only offer a create for a month that is genuinely absent and genuinely
        // has days to record. E2 is the `has_school_days` half of this.
        can_create: !state.is_on_record() && state.has_school_days,
        today_can_create: !today_state.is_on_record() && today_state.has_school_days,
        needs_school_start_date: school_start_date.is_none(),
        issues,
    })
}

/// Write one month's worksheet into the one workbook, so the user can switch to
/// that month in one click (spec edge case E1).
///
/// ## What changed with §0 A1
///
/// There is no per-month file to create any more. A month is a **worksheet** in
/// the one workbook the class already has, so "create this month" means: give the
/// file a `{MONTH} {year}` worksheet, lay that month's day-number grid over it,
/// copy the roster onto it, write the absences the database holds for it, and
/// record the month row and its grid.
///
/// ## What it will not do
///
/// * It does not touch the other eleven worksheets. A full merge does that; a
///   single-month create is additive, and the eleven sheets a teacher has already
///   been reading into are not this function's to rewrite.
/// * It does not create a worksheet with a mark in it. The absences come from
///   `events`, and a month nobody has recorded anything in gets an empty grid.
/// * It refuses a month with no school days (E2) rather than producing a
///   worksheet with no day to record.
///
/// ## When the workbook does not exist yet
///
/// The file is created from the bundled template first, so the worksheet has a
/// form to be built from. Nothing of the user's is at risk: there was nothing
/// there.
/// The month worksheet is written into the workbook the class already has, so the
/// directory is only consulted when there is no workbook on record at all.
#[allow(clippy::too_many_arguments)]
pub fn create_month_worksheet_in_dir(
    pool: &DbPool,
    _workbook_dir: &Path,
    class_id: Option<&str>,
    report_month: &str,
) -> Result<Sf2MonthTemplate> {
    let (month, month_number) = canonical_month(report_month)?;
    let class_id = resolve_class_id(pool, class_id)?;
    let school_year = resolve_school_year(pool, &class_id, None)?;
    let school_start_date = school_calendar_settings(pool)?
        .school_start_date
        .as_deref()
        .and_then(parse_date);
    let report_year = report_year_for_school_month(&school_year, month_number, current_year());

    // Edge case E2, checked before anything else so it is refused for the right
    // reason. A month with no school days - because `school_start_date` is unset,
    // or because classes start after the month ends - has no day to record, and a
    // worksheet with no day column is not something a teacher can use.
    let first_school_day = derive_first_school_day(school_start_date, month_number, report_year)
        .ok_or_else(|| AppError::InvalidInput(NO_SCHOOL_DAYS_MESSAGE.to_string()))?;

    let template_repo = Sf2MonthTemplateRepo::new(pool.clone());
    if template_repo
        .find(&class_id, &school_year, &month)?
        .is_some()
    {
        return Err(AppError::InvalidInput(format!(
            "The SF2 month {month} is already on record."
        )));
    }

    // The one workbook, from the pre-split row: that is the file the merge builds
    // and the file every other month already names.
    let legacy = Sf2Repository::new(pool.clone()).latest_template_for_class(&class_id)?;
    let path =
        match legacy.as_ref() {
            Some(legacy) => PathBuf::from(&legacy.source_path),
            None => return Err(AppError::InvalidInput(
                "There is no SF2 workbook for this class yet. Import the school's SF2 workbook \
                 first."
                    .to_string(),
            )),
        };
    if !path.is_file() {
        return Err(AppError::InvalidInput(format!(
            "The SF2 workbook at {} is missing. Restore it from a backup.",
            path.display()
        )));
    }

    // Roster carry-over. `previous_school_month` walks the school year, so this
    // is the month before in the order classes actually meet, not the month
    // before on the calendar. The roster is the same on every sheet, so the
    // nearest month that has one is as good a source as any.
    let previous = match previous_school_month(&school_year, month_number) {
        Some((name, year)) => template_repo
            .find(&class_id, &school_year, &name)?
            .filter(|template| template.report_year == year),
        None => None,
    };
    let roster = match previous.as_ref() {
        Some(template) => Sf2MonthStudentRepo::new(pool.clone()).for_template(&template.id)?,
        None => Vec::new(),
    };

    let learners = roster
        .iter()
        .map(
            |mapping| crate::sf2::month::workbook_builder::MonthLearnerWrite {
                student_id: mapping.student_id.clone(),
                row_index: mapping.row_index,
                name: mapping.workbook_name.clone(),
                item_number: 0,
                gender_block: mapping.gender_block.clone(),
            },
        )
        .collect::<Vec<_>>();
    // `No.` numbers are 1..n within each gender block; the build writes them.
    let learners = crate::sf2::month::merge::number_the_roster(learners);

    let absences = absences_for_month(pool, &class_id, &month, report_year)?;
    let header = legacy
        .as_ref()
        .map(|row| crate::sf2::month::workbook_builder::MonthHeader {
            school_id: row.school_id.clone(),
            school_name: row.school_name.clone(),
            school_year: row.school_year.clone(),
            report_month: sf2_month_name(month_number).to_string(),
            grade_level: row.grade_level.clone(),
            section: row.section.clone(),
            adviser_name: row.adviser_name.clone(),
            school_head_name: row.school_head_name.clone(),
        })
        .unwrap_or_default();

    let provisional_id = uuid::Uuid::new_v4().to_string();
    let report = crate::sf2::month::workbook_builder::build_school_year_workbook(
        &path,
        &[crate::sf2::month::workbook_builder::MonthSheetBuild {
            request: crate::sf2::month::workbook_builder::MonthBuildRequest {
                template_id: provisional_id.clone(),
                report_month: month.clone(),
                report_year,
                // The same anchor rule the merge uses: the known first school
                // day, else the 1st so every recorded absence has a column.
                first_school_day: crate::sf2::month::merge::grid_anchor_day(
                    Some(first_school_day),
                    report_year,
                    month_number,
                ),
                header,
                learners,
                absences,
                source_female_start_row: 0,
            },
            // Additive: the other eleven worksheets are left alone.
            remove_stale_sheets: false,
        }],
    )?;
    let month_report = &report.months[0];
    if !report.verification.is_verified() {
        return Err(AppError::Internal(format!(
            "The {month} worksheet was not saved: {}. Nothing was changed.",
            report
                .verification
                .mismatch_reason()
                .unwrap_or_else(|| "the build did not verify".to_string())
        )));
    }

    let now = chrono::Utc::now().timestamp();
    template_repo.upsert(&Sf2MonthTemplate {
        id: provisional_id.clone(),
        active_class_id: class_id.clone(),
        school_year: school_year.clone(),
        report_month: month.clone(),
        report_year,
        source_path: path.to_string_lossy().to_string(),
        source_hash: legacy
            .as_ref()
            .map(|row| row.source_hash.clone())
            .unwrap_or_else(|| hash_bytes(BUNDLED_TEMPLATE_BYTES)),
        school_id: previous.as_ref().and_then(|t| t.school_id.clone()),
        school_name: previous.as_ref().and_then(|t| t.school_name.clone()),
        grade_level: previous.as_ref().and_then(|t| t.grade_level.clone()),
        section: previous.as_ref().and_then(|t| t.section.clone()),
        adviser_name: previous.as_ref().and_then(|t| t.adviser_name.clone()),
        school_head_name: previous.as_ref().and_then(|t| t.school_head_name.clone()),
        first_school_day,
        first_school_day_override: None,
        imported_at: now,
        last_synced_at: None,
        workbook_x_count: month_report.written_marks as i64,
        // The build counted the marks it just wrote, so this month is measured
        // rather than "never scanned" (spec §9.1, E4).
        workbook_scanned_at: Some(now),
    })?;

    let dates = month_report
        .dates
        .iter()
        .map(|date| Sf2MonthDateMapping {
            template_id: provisional_id.clone(),
            date: date.date.clone(),
            column_letter: date.column_letter.clone(),
            column_index: date.column_index,
            sheet_name: date.resolved_sheet_name().into(),
        })
        .collect::<Vec<_>>();
    if !dates.is_empty() {
        Sf2MonthDateRepo::new(pool.clone()).replace_for_template(&provisional_id, &dates)?;
    }
    if !roster.is_empty() {
        let rebound = roster
            .into_iter()
            .map(|mut mapping| {
                mapping.template_id = provisional_id.clone();
                mapping
            })
            .collect::<Vec<_>>();
        Sf2MonthStudentRepo::new(pool.clone()).replace_for_template(&provisional_id, &rebound)?;
    }

    template_repo.find_by_id(&provisional_id)?.ok_or_else(|| {
        AppError::Internal(format!(
            "the {month} workbook row vanished after it was written"
        ))
    })
}

/// Every absence the database holds for one month, as `(student, date)` pairs.
fn absences_for_month(
    pool: &DbPool,
    class_id: &str,
    report_month: &str,
    report_year: i32,
) -> Result<Vec<crate::sf2::month::workbook_builder::MonthAbsence>> {
    let month_number = sf2_month_number(report_month).unwrap_or_default();
    let start = format!("{report_year:04}-{month_number:02}-01");
    let end = format!(
        "{report_year:04}-{month_number:02}-{:02}",
        last_day_of_month(report_year, month_number)
    );
    Ok(EventRepository::new(pool.clone())
        .list_for_class_and_date_range(class_id, &start, &end)?
        .into_iter()
        .filter(|event| event.event_type == AttendanceType::Absent)
        .map(|event| crate::sf2::month::workbook_builder::MonthAbsence {
            student_id: event.student_id.to_string(),
            date: event
                .timestamp
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d")
                .to_string(),
        })
        .collect())
}

/// The month before `month_number` in the same school year, as
/// `(name, report_year)`. `None` for SEPTEMBER, the first month, which has no
/// earlier month to carry a roster from.
fn previous_school_month(school_year: &str, month_number: u32) -> Option<(String, i32)> {
    let files = school_year_month_files(school_year, current_year());
    let index = files
        .iter()
        .position(|(name, _)| sf2_month_number(name) == Some(month_number))?;
    index.checked_sub(1).map(|previous| files[previous].clone())
}

/// What a month has: a stored row with a grid, or days to record.
struct MonthOnDisk {
    file_exists: bool,
    has_template: bool,
    has_school_days: bool,
    /// The one workbook's name, which every month shares.
    file_name: String,
}

impl MonthOnDisk {
    /// "On record" means something already exists for the month, so there is
    /// nothing to create and nothing to warn about.
    fn is_on_record(&self) -> bool {
        self.has_template
    }
}

/// The month the app will open, before it is described.
struct MonthChoice {
    month: String,
    report_year: i32,
    fell_back: bool,
}

/// The problems a month read should surface, in the order a teacher needs them.
///
/// Every one of these says "this month is not ready yet", which is the normal
/// state before the split has run. None of them says the database is wrong, and
/// none of them is a reason to write anything.
///
/// `uses_legacy_mappings` changes only the *wording* of the "no workbook of its
/// own" line, never whether it is raised. A month being served from the pre-split
/// tables still has no file of its own, and every write path still has to refuse.
///
/// But telling that teacher "no SF2 workbook is stored for SEPTEMBER yet" while
/// their X marks are on screen is the message that reads as data loss.
fn month_issues(
    month: &str,
    has_template: bool,
    file_exists: &bool,
    mappings: &[Sf2MonthDateMapping],
    roster: &[Sf2MonthStudentMapping],
    class_students: &[Student],
    uses_legacy_mappings: bool,
) -> Vec<String> {
    let mut issues = Vec::new();
    if !has_template {
        issues.push(if uses_legacy_mappings {
            format!(
                "No per-month SF2 workbook is stored for {month} yet. It is being read from the \
                 pre-split workbook, and nothing will be written until its own workbook exists."
            )
        } else {
            format!("No SF2 workbook is stored for {month} yet. Create it to switch to this month.")
        });
    } else if !file_exists {
        issues.push(
            "This month's SF2 workbook is missing from disk. Restore it from a backup; nothing will be written to it until it is back."
                .to_string(),
        );
    }
    if mappings.is_empty() {
        issues.push("No attendance days are mapped to this month yet.".to_string());
    }
    if roster.is_empty() {
        issues.push("No learners are mapped to this month's SF2 workbook yet.".to_string());
    }
    let mapped: HashSet<&str> = roster
        .iter()
        .map(|mapping| mapping.student_id.as_str())
        .collect();
    let unmapped = class_students
        .iter()
        .filter(|student| !mapped.contains(student.id.to_string().as_str()))
        .count();
    if unmapped > 0 {
        issues.push(format!(
            "{unmapped} of the class's {} students {} not mapped to this month's SF2 workbook.",
            class_students.len(),
            if unmapped == 1 { "is" } else { "are" },
        ));
    }
    issues
}

/// The month row as the template summary the Reports page already reads.
///
/// The same school, grade, section, adviser and school head on all twelve month
/// files of a class is the point: the sidebar's identity panel is fed from
/// whichever month is on screen, and a month switch never has to also go and
/// find a "current" template to describe the school with.
#[must_use]
pub fn template_summary_from_month(template: &Sf2MonthTemplate) -> Sf2TemplateSummary {
    Sf2TemplateSummary {
        id: template.id.clone(),
        source_path: template.source_path.clone(),
        school_id: meta_text(template.school_id.as_deref()),
        school_name: meta_text(template.school_name.as_deref()),
        school_year: template.school_year.clone(),
        report_month: template.report_month.clone(),
        grade_level: meta_text(template.grade_level.as_deref()),
        section: meta_text(template.section.as_deref()),
        adviser_name: meta_text(template.adviser_name.as_deref()),
        school_head_name: meta_text(template.school_head_name.as_deref()),
        class_id: template.active_class_id.clone(),
        imported_at: template.imported_at,
    }
}

/// The month roster in the shape the shared preview builder takes.
///
/// `sf2_month_student_mappings` is a superset of `sf2_student_mappings` - the
/// same columns plus the DepEd learner ID - so this conversion loses nothing
/// the grid reads, and duplicating the builder instead would be how the two
/// drift.
fn roster_as_legacy_mappings(roster: &[Sf2MonthStudentMapping]) -> Vec<Sf2StudentMappingRecord> {
    roster
        .iter()
        .map(|mapping| Sf2StudentMappingRecord {
            template_id: mapping.template_id.clone(),
            student_id: mapping.student_id.clone(),
            workbook_name: mapping.workbook_name.clone(),
            normalized_name: mapping.normalized_name.clone(),
            row_index: mapping.row_index,
            gender_block: mapping.gender_block.clone(),
        })
        .collect()
}

/// Read `YYYY-MM-DD`, or `None` for anything else. A malformed stored date is
/// "not set" - never a panic, never a guess.
fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok()
}

fn meta_text(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or_default().to_string()
}

/// Excel column letter for a 1-based index: 6 is `F`, 38 is `AL`.
#[cfg(test)]
fn column_letter(mut index: u32) -> String {
    let mut letters = Vec::new();
    while index > 0 {
        let offset = (index - 1) % 26;
        letters.push((b'A' + u8::try_from(offset).expect("column offset under 26")) as char);
        index = (index - 1) / 26;
    }
    letters.iter().rev().collect()
}

/// The workbook directory, for the command layer.
pub fn workbook_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf> {
    crate::sf2::workbook_files::sf2_workbook_dir(app)
}

#[cfg(test)]
#[path = "__tests__/month_preview_tests.rs"]
mod tests;
