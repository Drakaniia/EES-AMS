//! The merge job: turn one pre-split workbook into the **one file with twelve
//! month worksheets** (spec §0 A1, A4, A5; D14 re-pointed).
//!
//! ## What it does
//!
//! ```text
//! 1. Resolve the class's workbook identity from the pre-split `sf2_templates`
//!    row - school, grade, section, adviser, school year - and the one file that
//!    identity names.
//! 2. Take a workbooks backup, and copy the file into `sf2-workbooks/_legacy/`
//!    *before anything is written*. The copy is required, not best effort: the
//!    file being rebuilt is the user's only known copy of the original marks, and
//!    `_legacy/` is what it becomes.
//! 3. Read the pre-split workbook read-only, month by month, and compare each
//!    month's `X` marks with the database's absences for that month. This is
//!    §0 A5's required read-only comparison, run before any write path opens.
//! 4. Build all twelve month worksheets in the one file, from the bundled
//!    DepEd template, each with its own day-number grid, its own roster, its own
//!    header and its own `X` marks written from `events`.
//! 5. Delete every School Form 2 worksheet that is not one of the twelve - which
//!    is where a leftover `__SF2_HIDDEN_{n}` from the retired hide/rename/clear
//!    cycle goes, along with the bundled template's own sample sheets and their
//!    36 sample marks.
//! 6. Record twelve `sf2_month_templates` rows (all naming the same file), twelve
//!    day grids carrying their `sheet_name`, and the roster.
//! 7. Stamp `settings.sf2_split_completed_at` only when all twelve verified.
//! ```
//!
//! ## What it will not do
//!
//! * **It will not delete anything outside the one file's worksheets.** The
//!   pre-split workbook stays readable in `_legacy/`. A per-month `.xls` left by
//!   an earlier build stays where it is; its marks are folded in (see
//!   [`fold_stray_month_files`]) and the file is then left alone.
//! * **It will not import a mark from a worksheet it cannot vouch for.** A month
//!   whose pre-split sheet holds an `X` the database cannot produce is reported
//!   as needing attention and the file is not written, because §0 A5 says the
//!   `.xls` is the recovery source and the recovery runs first.
//! * **It will not write a month with no school days.** A month with no day
//!   column gets a worksheet with an empty grid, and zero marks. Never a
//!   fabricated day, and never a mark it invented.
//! * **It will not re-run over a finished merge.** Twelve verified rows naming
//!   the one file is the "done" state, and re-running is a no-op - the file may
//!   hold marks Excel wrote since.
//!
//! ## Naming
//!
//! The Rust types are named for the merge; the wire names are unchanged
//! (`splitCompletedAt`, `run_sf2_workbook_split`, "Re-run the workbook split")
//! because the Settings screen that calls them is outside this change's scope. The
//! setting key is `settings.sf2_split_completed_at` for the same reason: renaming
//! a stored key would mean a migration that could lose the stamp that tells this
//! job it has already run.

use crate::domain::error::{AppError, Result};
use crate::domain::models::{AttendanceType, CreateStudentRequest, StudentGender, StudentId};
use crate::infrastructure::database::{DbPool, EventRepository, StudentRepository};
use crate::sf2::calendar::{sf2_month_name, sf2_month_number};
use crate::sf2::logic::{is_learner_name, SF2_ABSENT_MARK};
use crate::sf2::models::{Sf2TemplateRecord, Sf2WorkbookLearner};
use crate::sf2::month::first_school_day::{
    current_year, derive_first_school_day, effective_first_school_day, school_year_start_year,
};
use crate::sf2::month::schema_v24;
use crate::sf2::month::student_repo::Sf2MonthStudentRepo;
use crate::sf2::month::template_repo::Sf2MonthTemplateRepo;
use crate::sf2::month::workbook_builder::{
    build_school_year_workbook, read_legacy_months, LegacyMonthSnapshot, MonthAbsence,
    MonthBuildRequest, MonthHeader, MonthLearnerWrite, MonthSheetBuild,
};
use crate::sf2::month::{
    match_roster_learner, Sf2LearnerMatchKind, Sf2MonthDateRepo, Sf2MonthStudentMapping,
    Sf2MonthTemplate, FIRST_SCHOOL_DAY_UNDETERMINED,
};
use crate::sf2::repository::Sf2Repository;
use crate::sf2::roster_parser::unique_normalized_name;
use crate::sf2::workbook_files::{
    school_year_month_files, sf2_legacy_workbook_dir, sf2_workbook_dir, single_workbook_file_name,
    single_workbook_path, write_bundled_template_to,
};
use chrono::{Datelike, NaiveDate};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const GET_SCHOOL_START_DATE_SQL: &str = include_str!("../sql/month_get_school_start_date.sql");
const GET_SPLIT_COMPLETED_AT_SQL: &str = include_str!("../sql/month_get_split_completed_at.sql");
const SET_SPLIT_COMPLETED_AT_SQL: &str = include_str!("../sql/month_set_split_completed_at.sql");
const SET_LAST_REPORT_MONTH_SQL: &str = include_str!("../sql/month_set_last_report_month.sql");
const LEGACY_FIRST_SCHOOL_DAY_SQL: &str = include_str!("../sql/month_legacy_first_school_day.sql");

/// How many months a school year has, and therefore how many must verify before
/// the completion stamp may be written.
pub const SPLIT_MONTH_COUNT: usize = 12;

/// What happened to one month of the school year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MergeMonthStatus {
    /// The month's worksheet is in the file, visible, and holds exactly the marks
    /// the database holds.
    Verified,
    /// A previous run already merged this month. Left untouched, because the
    /// file may hold marks written to it since.
    AlreadyMerged,
    /// The month could not be built, or could not be proven. `detail` says why and
    /// nothing was written.
    NeedsAttention,
}

/// One month of the merge, as the UI renders it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeMonthOutcome {
    /// Canonical uppercase month name, e.g. `SEPTEMBER`.
    pub report_month: String,
    pub report_year: i32,
    /// The one worksheet this month now lives on, e.g. `SEPTEMBER 2026`. Empty
    /// for a month that could not be built.
    pub sheet_name: String,
    /// The one file all twelve months share, e.g. `SF2-GRADE-3-MATAPAT-3b635890.xls`.
    ///
    /// The same value on all twelve rows: that is the point of §0 A1, and a
    /// teacher reading the list should see that there is one file, not twelve.
    pub file_name: String,
    pub status: MergeMonthStatus,
    /// How many `X` marks the month's worksheet holds.
    pub x_marks: usize,
    pub learner_rows: usize,
    /// Why a month needs attention. `None` for a month that is fine.
    pub detail: Option<String>,
}

impl MergeMonthOutcome {
    /// Is this month accounted for - either merged now, or by an earlier run?
    #[must_use]
    pub fn is_accounted_for(&self) -> bool {
        self.status != MergeMonthStatus::NeedsAttention
    }
}

/// The whole merge, as the UI renders it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeOutcome {
    /// When all twelve months verified, or `None` while any still needs
    /// attention. The same value as `settings.sf2_split_completed_at`.
    pub split_completed_at: Option<i64>,
    pub months: Vec<MergeMonthOutcome>,
    pub verified_count: usize,
    pub needs_attention_count: usize,
    /// The one workbook all twelve months live in.
    pub workbook_path: String,
    /// Where the untouched pre-split workbook lives.
    pub legacy_file_path: String,
    /// The workbooks backup taken before anything was written, when one could be
    /// taken.
    pub legacy_backup_path: Option<String>,
    /// Absences the database holds in months this school year has no worksheet
    /// for, so this workbook cannot show them.
    ///
    /// Never silently zero when it is not: they are in the database and in the
    /// reports grid, and a complete-looking file that quietly omits nine of a
    /// teacher's absences is how data appears to vanish.
    pub absences_outside_school_year: usize,
    /// The sentence the Settings screen shows.
    pub message: String,
}

/// The months the user has to do something about, named the way the report does.
#[must_use]
pub fn needs_attention_labels(months: &[MergeMonthOutcome]) -> Vec<String> {
    months
        .iter()
        .filter(|month| month.status == MergeMonthStatus::NeedsAttention)
        .map(|month| format!("{} {}", month.report_month, month.report_year))
        .collect()
}

/// How many months are accounted for.
#[must_use]
pub fn verified_count(months: &[MergeMonthOutcome]) -> usize {
    months
        .iter()
        .filter(|month| month.is_accounted_for())
        .count()
}

/// May the completion stamp be written?
///
/// Only when every one of the twelve months is accounted for. A partial run
/// leaves it `NULL`, which is what makes the next run a resume rather than a
/// second, conflicting build.
#[must_use]
pub fn is_merge_complete(months: &[MergeMonthOutcome]) -> bool {
    months.len() == SPLIT_MONTH_COUNT && months.iter().all(MergeMonthOutcome::is_accounted_for)
}

/// The sentence the Settings screen shows under *Re-run the workbook split*.
///
/// Always says where the original workbook is kept, because the merge is the one
/// operation in this app that rewrites the file the original marks lived in, and
/// the user is entitled to know the original is still there.
#[must_use]
pub fn merge_summary_message(
    completed_at: Option<i64>,
    months: &[MergeMonthOutcome],
    legacy_file_name: &str,
) -> String {
    let verified = verified_count(months);
    let attention = needs_attention_labels(months);
    let kept = format!(
        "All {SPLIT_MONTH_COUNT} months are on one workbook, each on its own visible worksheet. \
         The original workbook is kept in sf2-workbooks\\_legacy ({legacy_file_name})."
    );

    if completed_at.is_some() {
        return format!("{kept} All {verified} verified.");
    }
    if attention.is_empty() {
        return format!("{kept} {verified} verified.");
    }
    format!(
        "{kept} {verified} verified, {} needs attention ({}).",
        attention.len(),
        attention.join(", "),
    )
}

/// The first attendance day of one month, from what is known.
///
/// An override the user typed always wins. Then the D16 derivation from
/// `school_start_date`. Then - and only then - the day the pre-split workbook
/// already recorded for that month.
///
/// `None` means the month genuinely cannot be dated yet, and the merge reports
/// it rather than inventing a day: a guessed first day would silently mis-date
/// every column of that month's worksheet.
#[must_use]
pub fn resolve_first_school_day(
    school_start_date: Option<NaiveDate>,
    report_month: u32,
    report_year: i32,
    override_day: Option<u32>,
    legacy_day: Option<u32>,
) -> Option<u32> {
    let known = |day: Option<u32>| day.filter(|day| *day != FIRST_SCHOOL_DAY_UNDETERMINED);
    let derived = derive_first_school_day(school_start_date, report_month, report_year);
    effective_first_school_day(known(override_day), derived.or_else(|| known(legacy_day)))
}

/// The `No.` values a roster prints: 1..n within each gender block, in row order.
#[must_use]
pub fn item_numbers_for(learners: &[&Sf2WorkbookLearner]) -> Vec<u32> {
    let mut male = 0u32;
    let mut female = 0u32;
    learners
        .iter()
        .map(|learner| {
            if learner.gender_block.as_deref() == Some("FEMALE") {
                female += 1;
                female
            } else {
                male += 1;
                male
            }
        })
        .collect()
}

// ── The job ─────────────────────────────────────────────────────────────────

/// Build the single twelve-sheet workbook from the class's pre-split one.
///
/// Idempotent, resumable, and safe to re-run. A month that cannot be proven is
/// abandoned on its own and the file is not written at all - see the module docs
/// for why a partial file would be worse than none.
pub fn merge_workbooks(app: &tauri::AppHandle, pool: &DbPool) -> Result<MergeOutcome> {
    let workbook_dir = sf2_workbook_dir(app)?;
    let legacy = legacy_template_row(pool)?;
    let school_year = resolve_school_year(pool, &legacy)?;
    let months = school_year_month_files(&school_year, current_year());
    let workbook = single_workbook_path(
        &workbook_dir,
        &legacy.id,
        &legacy.grade_level,
        &legacy.section,
    );
    let file_name = single_workbook_file_name(&legacy.id, &legacy.grade_level, &legacy.section);

    let already_completed_at = split_completed_at(pool)?;
    if already_completed_at.is_some()
        && all_months_merged(
            pool,
            &workbook,
            &legacy.active_class_id,
            &school_year,
            &months,
        )
    {
        // Done, and every month still names the one file. Doing nothing is the
        // correct answer and the only safe one: the file may have been written to
        // since.
        return Ok(already_merged_outcome(
            already_completed_at,
            &months,
            &workbook,
            &file_name,
            &workbook_dir,
        ));
    }

    // §0 A5: the workbooks backup and the preserved copy, both before anything is
    // written. `require_preserved_legacy_workbook` refuses rather than
    // proceeding without a verified copy.
    let backup_path = take_workbooks_backup(&workbook_dir);
    let legacy_source = legacy_source_path(&workbook_dir, &legacy);
    let legacy_copy = require_preserved_legacy_workbook(&workbook_dir, &legacy_source)?;

    let template_repo = Sf2MonthTemplateRepo::new(pool.clone());
    let students_repo = StudentRepository::new(pool.clone());
    let school_start_date = school_start_date(pool)?;
    let existing_student_ids = students_repo
        .list_by_class(Some(&legacy.active_class_id))?
        .iter()
        .map(|student| student.id)
        .collect::<HashSet<StudentId>>();

    let reference = reference_mappings(pool, &legacy)?;
    let roster = resolve_roster(
        &students_repo,
        &legacy.active_class_id,
        &reference,
        &existing_student_ids,
    )?;

    // What the pre-split workbook holds, read without writing to it. Only a
    // worksheet whose name parses to a month of this school year is ever read, so
    // a `__SF2_HIDDEN_{n}` left by the retired cycle cannot be.
    let mut pre_existing = if legacy_source.is_file() {
        read_pre_existing_months(&legacy_source, &months)?
    } else {
        HashMap::new()
    };
    // An old per-month `.xls` from the retired twelve-file model, if one was ever
    // written. Its marks are folded into the same comparison; the file itself is
    // never deleted or moved.
    let folded = fold_stray_month_files(&workbook_dir, &months)?;
    for (report_month, report_year, snapshot) in folded {
        log::warn!(
            "workbook merge: folding {} {} from the old per-month file {}. The file is left \
             exactly where it is.",
            report_month,
            report_year,
            crate::sf2::workbook_files::month_workbook_file_name(&report_month, report_year)
        );
        pre_existing.insert((report_month, report_year), snapshot);
    }

    let (absences, outside_school_year) = load_absences(pool, &legacy.active_class_id, &months)?;

    // §0 A5: a workbook mark the database cannot produce blocks the write. The
    // database is the source of truth and the workbook is a mirror - but the
    // mirror is the recovery source until a comparison proves otherwise, so this
    // is checked, not assumed.
    let unproven = unproven_months(&pre_existing, &absences, &roster.row_by_student);
    if !unproven.is_empty() {
        return Ok(merge_outcome(
            None,
            months
                .iter()
                .map(|(month, year)| {
                    let key = month.to_uppercase();
                    MergeMonthOutcome {
                        report_month: key.clone(),
                        report_year: *year,
                        sheet_name: String::new(),
                        file_name: file_name.clone(),
                        status: if unproven.contains_key(&key) {
                            MergeMonthStatus::NeedsAttention
                        } else {
                            MergeMonthStatus::AlreadyMerged
                        },
                        x_marks: 0,
                        learner_rows: roster.writes.len(),
                        detail: unproven.get(&key).cloned(),
                    }
                })
                .collect(),
            &workbook,
            &legacy_copy,
            backup_path,
            outside_school_year.len(),
        ));
    }

    // The twelve requests. Every one names the same roster and the same header
    // block; only the day grid and the marks differ, which is what makes the grid
    // genuinely month-specific rather than twelve copies of one month.
    let mut builds = Vec::with_capacity(SPLIT_MONTH_COUNT);
    let mut resolved_days = Vec::with_capacity(SPLIT_MONTH_COUNT);
    for (report_month, report_year) in &months {
        let month_number = sf2_month_number(report_month).unwrap_or_default();
        let existing_row =
            template_repo.find(&legacy.active_class_id, &school_year, report_month)?;
        let first_school_day = resolve_first_school_day(
            school_start_date,
            month_number,
            *report_year,
            existing_row
                .as_ref()
                .and_then(|row| row.first_school_day_override),
            legacy_first_school_day(pool, &legacy.id, month_number)?,
        );
        resolved_days.push((*report_year, first_school_day));

        let month_key = report_month.to_uppercase();
        builds.push(MonthSheetBuild {
            request: MonthBuildRequest {
                // Provisional: the row may already exist under another id (the v22
                // backfill reuses the pre-split template's id for its own month),
                // and the real one is read back before the mappings are written.
                template_id: uuid::Uuid::new_v4().to_string(),
                report_month: month_key.clone(),
                report_year: *report_year,
                first_school_day: grid_anchor_day(first_school_day, *report_year, month_number),
                header: MonthHeader {
                    school_id: legacy.school_id.clone(),
                    school_name: legacy.school_name.clone(),
                    school_year: legacy.school_year.clone(),
                    // Each worksheet carries **its own** month in the header. The
                    // retired model wrote one `report_month` to every sheet in the
                    // file, which under twelve months would label eleven of them
                    // with the wrong one.
                    report_month: sf2_month_name(month_number).to_string(),
                    grade_level: legacy.grade_level.clone(),
                    section: legacy.section.clone(),
                    adviser_name: legacy.adviser_name.clone(),
                    school_head_name: legacy.school_head_name.clone(),
                },
                learners: roster.writes.clone(),
                absences: absences.get(&month_key).cloned().unwrap_or_default(),
                // The roster is laid out the way a fresh bundled template is, so
                // the same row index means the same learner on all twelve sheets.
                source_female_start_row: 0,
            },
            // The full build removes the template's sample sheets and any leftover
            // `__SF2_HIDDEN_*`; a single-month build leaves the other eleven alone.
            remove_stale_sheets: true,
        });
    }

    if !workbook.exists() {
        // Nothing of the user's is here yet, so there is nothing to read and
        // nothing to lose: start from the bundled template.
        write_bundled_template_to(&workbook)?;
    }

    let report = build_school_year_workbook(&workbook, &builds)?;
    let verified = report.verification.is_verified();

    let mut results = Vec::with_capacity(SPLIT_MONTH_COUNT);
    for (index, (report_month, report_year)) in months.iter().enumerate() {
        let month_report = &report.months[index];
        let month_key = report_month.to_uppercase();
        let stored_first_school_day = resolved_days[index].1;

        if !verified {
            results.push(MergeMonthOutcome {
                report_month: month_key,
                report_year: *report_year,
                sheet_name: month_report.sheet_name.clone(),
                file_name: file_name.clone(),
                status: MergeMonthStatus::NeedsAttention,
                x_marks: 0,
                learner_rows: roster.writes.len(),
                detail: Some(format!(
                    "The workbook was not saved: {}. Nothing was changed, and the original \
                     workbook still holds every mark.",
                    report
                        .verification
                        .mismatch_reason()
                        .unwrap_or_else(|| "the build did not verify".to_string())
                )),
            });
            continue;
        }

        match record_month(
            pool,
            &legacy,
            &workbook,
            &MonthOnRecord {
                school_year: &school_year,
                report_month,
                report_year: *report_year,
                first_school_day: stored_first_school_day,
            },
            month_report,
            &roster.mappings,
        ) {
            Ok(()) => results.push(MergeMonthOutcome {
                report_month: month_key,
                report_year: *report_year,
                sheet_name: month_report.sheet_name.clone(),
                file_name: file_name.clone(),
                status: MergeMonthStatus::Verified,
                x_marks: month_report.written_marks,
                learner_rows: roster.writes.len(),
                detail: None,
            }),
            Err(error) => {
                log::error!("workbook merge: {month_key} {report_year} needs attention: {error}");
                results.push(MergeMonthOutcome {
                    report_month: month_key,
                    report_year: *report_year,
                    sheet_name: month_report.sheet_name.clone(),
                    file_name: file_name.clone(),
                    status: MergeMonthStatus::NeedsAttention,
                    x_marks: 0,
                    learner_rows: roster.writes.len(),
                    detail: Some(error.to_string()),
                });
            }
        }
    }

    // The completion stamp, and only once all twelve are fine.
    let completed_at = if verified && is_merge_complete(&results) {
        let stamp = unix_now();
        set_split_completed_at(pool, stamp)?;
        Some(stamp)
    } else {
        None
    };
    let _ = set_last_report_month(pool, &legacy.report_month);

    Ok(merge_outcome(
        completed_at,
        results,
        &workbook,
        &legacy_copy,
        backup_path,
        outside_school_year.len(),
    ))
}

/// The day a month's day-number grid is anchored on.
///
/// ## Why this is not the first school day
///
/// It usually is - the D16 derivation, or the day the pre-split workbook already
/// recorded. When neither is known it is **1**, and that is a deliberate
/// difference: a grid anchored on the 1st covers the whole month, so every school
/// day the database holds a record for has a column to sit in. A narrower grid
/// would leave a recorded absence with nowhere to go, and an absence with nowhere
/// to go is the reported bug - SEPTEMBER 2026 on the install this was written for
/// had 17 absences and no day columns at all.
///
/// The month row still stores `FIRST_SCHOOL_DAY_UNDETERMINED` in that case, so
/// the Settings list shows the month as undated, the prompt for *Classes started
/// on* still fires once, and nothing anywhere claims a school day that was never
/// established. What the grid says is "these are the days of this month", not
/// "class started on the 1st".
#[must_use]
pub fn grid_anchor_day(first_school_day: Option<u32>, report_year: i32, report_month: u32) -> u32 {
    let last_day = days_in_month(report_year, report_month);
    first_school_day
        .filter(|day| *day >= 1 && *day <= last_day)
        .unwrap_or(1)
}

/// The last calendar day of a month.
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

/// Every month the pre-split workbook has a real, parseable sheet for.
fn read_pre_existing_months(
    path: &Path,
    months: &[(String, i32)],
) -> Result<HashMap<(String, i32), LegacyMonthSnapshot>> {
    Ok(read_legacy_months(path, months)?
        .into_iter()
        .filter_map(|read| {
            read.snapshot
                .map(|snapshot| ((read.report_month, read.report_year), snapshot))
        })
        .collect())
}

/// The affordance the Settings screen calls.
///
/// No `force` flag, on purpose. The job decides for itself whether there is
/// anything to do, so pressing it twice is safe, and a finished merge is never
/// rebuilt over the marks Excel has written into the file since.
#[tauri::command]
pub fn run_sf2_workbook_split(
    app: tauri::AppHandle,
    pool: tauri::State<'_, DbPool>,
) -> std::result::Result<MergeOutcome, String> {
    merge_workbooks(&app, pool.inner()).map_err(|error| error.to_string())
}

// ── §0 A5: the read-only comparison ─────────────────────────────────────────

/// Every `absent` event the database holds, split by whether this school year
/// has a worksheet for it.
///
/// ## The half that is not in the school year
///
/// A school year is twelve months, SEPTEMBER -> AUGUST, so a database that
/// predates it - or that simply spans two of them - holds absences in months no
/// worksheet exists for. On the install this was written for that is nine
/// absences, in JUNE 2026 and AUGUST 2026, which belong to school year 2025-2026.
///
/// They are **not** silently dropped. They are counted and reported
/// ([`MergeOutcome::absences_outside_school_year`]) so a teacher is told that the
/// workbook cannot show them, rather than concluding from a complete-looking file
/// that those absences were never recorded. They remain in the database and in the
/// grid, which reads `events` and does not need a worksheet.
/// The absences of one school year, keyed by upper-case month name.
///
/// Named because `HashMap<String, Vec<MonthAbsence>>` spelled out as a return
/// type is a wall of generics that hides what the caller is being handed.
type AbsencesByMonth = HashMap<String, Vec<MonthAbsence>>;

fn load_absences(
    pool: &DbPool,
    class_id: &str,
    months: &[(String, i32)],
) -> Result<(AbsencesByMonth, Vec<MonthAbsence>)> {
    let events = EventRepository::new(pool.clone()).list()?;
    let mut by_month: AbsencesByMonth = HashMap::new();
    for (report_month, _) in months {
        by_month.entry(report_month.to_uppercase()).or_default();
    }

    let mut outside = Vec::new();
    for event in events {
        if event.event_type != AttendanceType::Absent {
            continue;
        }
        if event.class_id.as_deref() != Some(class_id) {
            continue;
        }
        let local = event.timestamp.with_timezone(&chrono::Local);
        let key = sf2_month_name(local.month()).to_uppercase();
        let year = local.year();
        let in_school_year = months
            .iter()
            .any(|(month, report_year)| month.eq_ignore_ascii_case(&key) && *report_year == year);
        let absence = MonthAbsence {
            student_id: event.student_id.to_string(),
            date: local.format("%Y-%m-%d").to_string(),
        };
        if in_school_year && by_month.contains_key(&key) {
            by_month.entry(key).or_default().push(absence);
        } else {
            outside.push(absence);
        }
    }
    if !outside.is_empty() {
        let dates = outside
            .iter()
            .map(|absence| absence.date.as_str())
            .collect::<Vec<_>>();
        log::warn!(
            "{} absence(s) the database holds fall in months this school year has no worksheet \
             for ({} ...). They are not lost - they are in the database and in the reports grid - \
             but this workbook cannot show them.",
            outside.len(),
            dates.first().copied().unwrap_or("?"),
        );
    }
    Ok((by_month, outside))
}

/// Months the pre-split workbook holds marks the database cannot produce.
///
/// A month with no pre-split worksheet is not in here: there is nothing in the
/// workbook to compare against, and §0 A5's comparison is a comparison, not an
/// assumption. A `__SF2_HIDDEN_{n}` worksheet can never land in this set either -
/// it carries no month in its name, so `read_legacy_months` never matches one,
/// which is what makes "never import the template's sample data" structural rather
/// than a promise.
fn unproven_months(
    pre_existing: &HashMap<(String, i32), LegacyMonthSnapshot>,
    absences: &HashMap<String, Vec<MonthAbsence>>,
    row_by_student: &HashMap<String, u32>,
) -> HashMap<String, String> {
    let student_by_row = row_by_student
        .iter()
        .map(|(student_id, row)| (*row, student_id.as_str()))
        .collect::<HashMap<_, _>>();

    let mut unproven = HashMap::new();
    for ((report_month, _), snapshot) in pre_existing {
        let key = report_month.to_uppercase();
        let Some(month_absences) = absences.get(&key) else {
            continue;
        };
        let placeable = month_absences
            .iter()
            .filter(|absence| row_by_student.contains_key(&absence.student_id))
            .map(|absence| (absence.student_id.as_str(), absence.date.as_str()))
            .collect::<HashSet<_>>();

        let mut unmatched: Vec<String> = Vec::new();
        for mark in &snapshot.marks {
            if !mark.value.eq_ignore_ascii_case(SF2_ABSENT_MARK) {
                continue;
            }
            // A mark in a column the sheet prints no day for cannot be turned
            // into a date, so it cannot be compared. It is not counted as a
            // missing database record, because there is no record it could be.
            let Some(date) = snapshot.date_in_column(mark.column_index) else {
                continue;
            };
            let Some(student_id) = student_by_row.get(&mark.row_index).copied() else {
                continue;
            };
            if !placeable.contains(&(student_id, date.as_str())) {
                unmatched.push(format!("row {} ({date})", mark.row_index));
            }
        }
        if !unmatched.is_empty() {
            unproven.insert(
                key,
                format!(
                    "the original {} sheet holds {} X mark(s) the database has no record of \
                     (first: {}). Nothing was written. Recover those absences from the workbook \
                     first, then run this again.",
                    report_month,
                    unmatched.len(),
                    unmatched[0]
                ),
            );
        }
    }
    unproven
}

// ── Which months still need building ────────────────────────────────────────

/// Is every month of the school year naming the one workbook?
fn all_months_merged(
    pool: &DbPool,
    workbook: &Path,
    class_id: &str,
    school_year: &str,
    months: &[(String, i32)],
) -> bool {
    if !workbook.is_file() {
        return false;
    }
    let template_repo = Sf2MonthTemplateRepo::new(pool.clone());
    months.iter().all(|(month, year)| {
        let row = template_repo.find(class_id, school_year, month);
        let Ok(Some(row)) = row else {
            return false;
        };
        if Path::new(&row.source_path) != workbook {
            return false;
        }
        // The stored year has to be the one this school year gives the month, or
        // a row left over from a previous school year would read as merged.
        row.report_year == *year
            && Sf2MonthDateRepo::new(pool.clone())
                .for_template(&row.id)
                .is_ok_and(|d| !d.is_empty())
    })
}

// ── The roster ──────────────────────────────────────────────────────────────

/// The one roster, shared by all twelve worksheets.
struct ResolvedRoster {
    /// The learner rows to write onto every sheet, in row order.
    writes: Vec<MonthLearnerWrite>,
    /// The per-month mappings, in row order. Identical for all twelve months,
    /// because there is one class and one roster - which is what makes one row
    /// index mean the same learner on every sheet of the file.
    mappings: Vec<Sf2MonthStudentMapping>,
    /// `student_id` -> `row_index`, for the §0 A5 comparison.
    row_by_student: HashMap<String, u32>,
}

/// Turn the class's roster into the rows every one of the twelve worksheets is
/// written with.
///
/// The order is the pre-split mapping's own row order, grouped by gender block,
/// so the roster the user already sees keeps its shape. Row indices are then
/// re-derived from the block sizes, which is what makes them identical on all
/// twelve sheets: males from row 8, females from just after the MALE TOTAL row at
/// `8 + max(male_count, 21)`. That is the same capacity rule
/// [`crate::sf2::roster::roster_parser::bundled_template_total_rows`] and the
/// formula writers use, so a row index is never a guess and never drifts between
/// the roster, the `COUNTIF` formulas and the guard's measurement.
///
/// A learner the workbook only knows is created; a learner the app already has is
/// reused rather than duplicated. Nothing here renames a student: the worksheets
/// get the names the pre-split workbook had, and the name in the database is the
/// teacher's to change.
fn resolve_roster(
    student_repo: &StudentRepository,
    class_id: &str,
    reference: &[Sf2MonthStudentMapping],
    existing_student_ids: &HashSet<StudentId>,
) -> Result<ResolvedRoster> {
    let named = reference
        .iter()
        .filter(|mapping| is_learner_name(&mapping.workbook_name))
        .cloned()
        .collect::<Vec<_>>();
    let male_count = named
        .iter()
        .filter(|mapping| mapping.gender_block.as_deref() != Some("FEMALE"))
        .count();
    let female_first_row = female_block_start(male_count);

    let mut ordered = named;
    ordered.sort_by_key(|mapping| {
        (
            if mapping.gender_block.as_deref() == Some("FEMALE") {
                1
            } else {
                0
            },
            mapping.row_index,
        )
    });

    let mut writes = Vec::with_capacity(ordered.len());
    let mut mappings = Vec::with_capacity(ordered.len());
    let mut row_by_student = HashMap::with_capacity(ordered.len());
    let mut seen_names = HashSet::new();
    let mut claimed: HashSet<String> = HashSet::new();
    let mut male = 0u32;
    let mut female = 0u32;

    for mapping in &ordered {
        let name = mapping.workbook_name.trim();
        let is_female = mapping.gender_block.as_deref() == Some("FEMALE");
        let student_id = resolve_student_id(
            student_repo,
            class_id,
            name,
            mapping.sf2_learner_id.as_deref(),
            mapping.gender_block.as_deref(),
            &ordered,
            existing_student_ids,
            &claimed,
        )?;
        claimed.insert(student_id.to_string());

        let normalized_name =
            unique_normalized_name(&mut seen_names, name, &mapping.row_index.to_string());
        let (row_index, item_number) = if is_female {
            let row = female_first_row + female;
            female += 1;
            (row, female)
        } else {
            let row = FIRST_MALE_ROW + male;
            male += 1;
            (row, male)
        };
        row_by_student.insert(student_id.to_string(), row_index);

        writes.push(MonthLearnerWrite {
            student_id: student_id.to_string(),
            row_index,
            name: name.to_string(),
            item_number,
            gender_block: Some(if is_female { "FEMALE" } else { "MALE" }.to_string()),
        });
        mappings.push(Sf2MonthStudentMapping {
            // Filled in with the stored id by `record_month`.
            template_id: String::new(),
            student_id: student_id.to_string(),
            workbook_name: name.to_string(),
            normalized_name,
            row_index,
            gender_block: Some(if is_female { "FEMALE" } else { "MALE" }.to_string()),
            sf2_learner_id: mapping.sf2_learner_id.clone(),
        });
    }

    Ok(ResolvedRoster {
        writes,
        mappings,
        row_by_student,
    })
}

/// The first learner row of the female block, for a roster of `male_count` males.
///
/// Males run from row 8 and the MALE TOTAL row sits at `8 + max(male_count, 21)`,
/// because a bundled template always has room for 21 before it has to grow. The
/// same arithmetic is in
/// [`crate::sf2::roster::roster_parser::bundled_template_total_rows`], which the
/// build uses to place the TOTAL rows - so the roster and the formulas agree by
/// construction rather than by two copies of a constant.
#[must_use]
pub fn female_block_start(male_count: usize) -> u32 {
    // The first female row is the one *after* the MALE TOTAL row, which sits at
    // 8 + max(male_count, 21). A bundled template always has room for 21
    // males before it has to grow, which is the same rule
    // expanded_roster_slots uses to place the female block.
    FIRST_MALE_ROW + 1 + MALE_SLOTS.max(male_count as u32)
}

/// The `No.` values a roster prints: 1..n within each gender block, in row order.
///
/// Applied to a roster that arrived without them - the "create this month" path
/// carries the roster over from another month, and the numbers are a property of
/// the block rather than of the month, so they are the same on all twelve sheets.
#[must_use]
pub fn number_the_roster(learners: Vec<MonthLearnerWrite>) -> Vec<MonthLearnerWrite> {
    let mut male = 0u32;
    let mut female = 0u32;
    learners
        .into_iter()
        .map(|mut learner| {
            learner.item_number = if learner.gender_block.as_deref() == Some("FEMALE") {
                female += 1;
                female
            } else {
                male += 1;
                male
            };
            learner
        })
        .collect()
}

/// The row males start on, and how many male slots a bundled template ships with.
const FIRST_MALE_ROW: u32 = 8;
const MALE_SLOTS: u32 = 21;

/// Which student a learner row belongs to.
///
/// [`match_roster_learner`] owns the order - DepEd learner ID, then name, then
/// row - and a row-index match is logged, because position is not identity and a
/// match on it is the reshuffle that re-points a student's marks at somebody else.
/// A learner already claimed by an earlier row is not reused:
/// `sf2_month_student_mappings` keys on `(template_id, student_id)`, and two
/// workbook rows pointing at one student would collide.
#[allow(clippy::too_many_arguments)]
fn resolve_student_id(
    student_repo: &StudentRepository,
    class_id: &str,
    workbook_name: &str,
    learner_id: Option<&str>,
    gender_block: Option<&str>,
    reference: &[Sf2MonthStudentMapping],
    existing_student_ids: &HashSet<StudentId>,
    claimed: &HashSet<String>,
) -> Result<StudentId> {
    let probe = Sf2WorkbookLearner {
        name: workbook_name.to_string(),
        row_index: 0,
        gender_block: gender_block.map(str::to_string),
        sf2_learner_id: learner_id.map(str::to_string),
    };
    if let Some(matched) = match_roster_learner(reference, &probe) {
        if matched.matched_by == Sf2LearnerMatchKind::RowIndex {
            log::warn!(
                "workbook merge: `{workbook_name}` was matched by position, not by name or DepEd \
                 ID; check this learner's X marks"
            );
        }
        if let Ok(id) = uuid::Uuid::parse_str(&matched.student_id) {
            let id = StudentId(id);
            if existing_student_ids.contains(&id) && !claimed.contains(&id.to_string()) {
                return Ok(id);
            }
        }
    }

    Ok(student_repo
        .create(CreateStudentRequest {
            name: workbook_name.to_string(),
            gender: StudentGender::from_sf2_block(gender_block),
            card_serial: None,
            class_id: Some(class_id.to_string()),
        })?
        .id)
}

// ── One month, recorded ─────────────────────────────────────────────────────

/// Which month a set of rows belongs to, as the school year and the day it
/// opened.
///
/// Grouped into one value because a month is one fact, and handing the four
/// halves of it around separately is how a row ends up filed under a school year
/// it was not built for.
struct MonthOnRecord<'a> {
    school_year: &'a str,
    report_month: &'a str,
    report_year: i32,
    first_school_day: Option<u32>,
}

/// Write the month's rows: the template row, the day grid, and the roster.
///
/// All twelve of these point `source_path` at the same file, which is the whole
/// of A0 A4's "one file, twelve rows, one file named by all of them".
fn record_month(
    pool: &DbPool,
    legacy: &Sf2TemplateRecord,
    workbook: &Path,
    month: &MonthOnRecord<'_>,
    report: &crate::sf2::month::workbook_builder::MonthBuildReport,
    mappings: &[Sf2MonthStudentMapping],
) -> Result<()> {
    let MonthOnRecord {
        school_year,
        report_month,
        report_year,
        first_school_day,
    } = *month;
    let template_repo = Sf2MonthTemplateRepo::new(pool.clone());
    let now = unix_now();
    let report_month = report_month.to_uppercase();
    let existing = template_repo.find(&legacy.active_class_id, school_year, &report_month)?;

    template_repo.upsert(&Sf2MonthTemplate {
        id: uuid::Uuid::new_v4().to_string(),
        active_class_id: legacy.active_class_id.clone(),
        school_year: school_year.to_string(),
        report_month: report_month.clone(),
        report_year,
        source_path: workbook.to_string_lossy().to_string(),
        // The file is a copy of the bundled template, so it keeps the pre-split
        // row's hash: `template_owns_roster` still recognises it.
        source_hash: legacy.source_hash.clone(),
        school_id: non_empty(&legacy.school_id),
        school_name: non_empty(&legacy.school_name),
        grade_level: non_empty(&legacy.grade_level),
        section: non_empty(&legacy.section),
        adviser_name: non_empty(&legacy.adviser_name),
        school_head_name: non_empty(&legacy.school_head_name),
        first_school_day: first_school_day.unwrap_or(FIRST_SCHOOL_DAY_UNDETERMINED),
        first_school_day_override: existing
            .as_ref()
            .and_then(|row| row.first_school_day_override),
        imported_at: now,
        last_synced_at: existing.as_ref().and_then(|row| row.last_synced_at),
        workbook_x_count: report.written_marks as i64,
        // The build counted the marks in the file it just wrote, so the month is
        // measured - not "never scanned" - from here on (spec §9.1, E4).
        workbook_scanned_at: Some(now),
    })?;

    // The conflict target is the month, not the id, so a row the v22 backfill
    // already created survives with its own id. Read it back rather than assume.
    let stored_id = template_repo
        .find(&legacy.active_class_id, school_year, &report_month)?
        .ok_or_else(|| {
            AppError::Internal(format!(
                "the {report_month} {report_year} row could not be stored"
            ))
        })?
        .id;

    // A derived day never overwrites one the user typed; the repository reports
    // that by updating nothing. A month with no known day is left undated rather
    // than given a guess.
    if let Some(day) = first_school_day {
        let _ = template_repo.derive_first_school_day(&stored_id, day)?;
    }
    let _ = template_repo.record_workbook_x_count(&stored_id, report.written_marks as i64, now)?;

    // The day grid, each row carrying the worksheet it lives on.
    let dates = report
        .dates
        .iter()
        .map(|date| crate::sf2::month::Sf2MonthDateMapping {
            template_id: stored_id.clone(),
            date: date.date.clone(),
            column_letter: date.column_letter.clone(),
            column_index: date.column_index,
            sheet_name: date.resolved_sheet_name().into(),
        })
        .collect::<Vec<_>>();
    if dates.is_empty() {
        // A month with no day columns keeps whatever grid it already had. The
        // grid is not this job's to destroy, and an empty grid is the state every
        // write path refuses to act on anyway.
        log::warn!(
            "workbook merge: {report_month} {report_year} has no day columns in the sheet, so \
             its stored day grid was left as it was"
        );
    } else {
        Sf2MonthDateRepo::new(pool.clone()).replace_for_template(&stored_id, &dates)?;
    }

    let month_mappings = mappings
        .iter()
        .map(|mapping| Sf2MonthStudentMapping {
            template_id: stored_id.clone(),
            ..mapping.clone()
        })
        .collect::<Vec<_>>();
    let student_repo = Sf2MonthStudentRepo::new(pool.clone());
    student_repo.replace_for_template(&stored_id, &month_mappings)?;

    // §6.3's backfill: the DepEd IDs this roster proved, stored on the students so
    // a later run can recognise the learner without opening a workbook. The
    // repository refuses a pair that would give one student another learner's ID.
    let learner_ids = month_mappings
        .iter()
        .filter_map(|mapping| {
            mapping
                .sf2_learner_id
                .as_deref()
                .map(|learner_id| (mapping.student_id.clone(), learner_id.to_string()))
        })
        .collect::<Vec<_>>();
    let _ = student_repo.set_student_learner_ids(&learner_ids);

    Ok(())
}

// ── Settings this job owns ──────────────────────────────────────────────────

fn school_start_date(pool: &DbPool) -> Result<Option<NaiveDate>> {
    let conn = pool.get()?;
    let raw = conn
        .query_row(GET_SCHOOL_START_DATE_SQL, [], |row| {
            row.get::<_, Option<String>>(0)
        })
        .optional()?
        .flatten();
    Ok(raw.and_then(
        |value| match NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d") {
            Ok(date) => Some(date),
            Err(_) => {
                log::warn!(
                "`school_start_date` holds `{value}`, which is not YYYY-MM-DD; treating it as unset"
            );
                None
            }
        },
    ))
}

fn split_completed_at(pool: &DbPool) -> Result<Option<i64>> {
    let conn = pool.get()?;
    Ok(conn.query_row(GET_SPLIT_COMPLETED_AT_SQL, [], |row| row.get(0))?)
}

fn set_split_completed_at(pool: &DbPool, completed_at: i64) -> Result<()> {
    let conn = pool.get()?;
    if conn.execute(SET_SPLIT_COMPLETED_AT_SQL, params![completed_at])? == 0 {
        return Err(AppError::Internal(
            "the workbook finished but there is no settings row to record it on".to_string(),
        ));
    }
    Ok(())
}

fn set_last_report_month(pool: &DbPool, report_month: &str) -> Result<()> {
    let canonical = canonical_month_name(report_month);
    if canonical.is_empty() {
        return Ok(());
    }
    let conn = pool.get()?;
    conn.execute(SET_LAST_REPORT_MONTH_SQL, params![canonical])?;
    Ok(())
}

/// The school year to build, from the pre-split row first and the settings
/// label second.
fn resolve_school_year(pool: &DbPool, legacy: &Sf2TemplateRecord) -> Result<String> {
    let stored = legacy.school_year.trim().to_string();
    if school_year_start_year(&stored).is_some() {
        return Ok(stored);
    }
    let fallback = crate::infrastructure::database::SettingsRepository::new(pool.clone())
        .get()?
        .school_year
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    if school_year_start_year(&fallback).is_some() {
        return Ok(fallback);
    }
    Err(AppError::InvalidInput(format!(
        "the workbook's school year (`{stored}`) holds no year, so the twelve month worksheets \
         cannot be named"
    )))
}

/// The pre-split workbook's own first attendance day for one month, read from the
/// dates it already recorded. This is data, not a guess.
fn legacy_first_school_day(pool: &DbPool, template_id: &str, month: u32) -> Result<Option<u32>> {
    let conn = pool.get()?;
    let day: Option<u32> = conn.query_row(
        LEGACY_FIRST_SCHOOL_DAY_SQL,
        params![template_id, format!("{month:02}")],
        |row| row.get(0),
    )?;
    Ok(day.filter(|day| *day > 0))
}

// ── The workbook file ───────────────────────────────────────────────────────

/// The pre-split workbook row: the class's workbook identity.
fn legacy_template_row(pool: &DbPool) -> Result<Sf2TemplateRecord> {
    let sf2_repo = Sf2Repository::new(pool.clone());
    let summary = sf2_repo
        .list_templates()?
        .into_iter()
        .next()
        .ok_or_else(|| {
            AppError::InvalidInput(
                "There is no SF2 workbook yet. Import the school's SF2 workbook first.".to_string(),
            )
        })?;
    sf2_repo
        .latest_template_for_class(&summary.class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("The stored SF2 workbook could not be read.".to_string())
        })
}

/// Where the pre-split workbook actually is, preferring the preserved copy.
///
/// After a first merge the copy in `_legacy/` is what the comparison read, so a
/// re-run is unaffected by anything that happened to the original in between.
fn legacy_source_path(workbook_dir: &Path, legacy: &Sf2TemplateRecord) -> PathBuf {
    let original = PathBuf::from(&legacy.source_path);
    let preserved = sf2_legacy_workbook_dir(workbook_dir).join(file_name_of(&legacy.source_path));
    if preserved.is_file() {
        preserved
    } else {
        original
    }
}

/// Copy the original into `_legacy/` and return the copy's path - or fail.
///
/// Unlike the pre-split job's best-effort copy, this one is **required**. The
/// file being rebuilt is the user's only known copy of the original marks, and
/// this copy is the artefact that protects them. An install that cannot make it
/// gets a refusal, not a rebuild.
fn require_preserved_legacy_workbook(workbook_dir: &Path, source: &Path) -> Result<PathBuf> {
    if !source.is_file() {
        // Nothing to preserve means nothing to lose: the file does not exist yet.
        return Ok(source.to_path_buf());
    }
    let legacy_dir = sf2_legacy_workbook_dir(workbook_dir);
    let destination = legacy_dir.join(file_name_of(&source.to_string_lossy()));
    if source == destination {
        return Ok(destination);
    }
    if destination.is_file() {
        // A copy that already exists is left exactly as it is: it is the snapshot
        // of the pre-merge workbook, and overwriting it with a later copy would
        // replace the one artefact that holds the original marks.
        return Ok(destination);
    }
    std::fs::create_dir_all(&legacy_dir).map_err(|error| {
        AppError::Internal(format!(
            "could not create {}: {error}. Nothing was written to the workbook.",
            legacy_dir.display()
        ))
    })?;
    std::fs::copy(source, &destination).map_err(|error| {
        AppError::Internal(format!(
            "could not preserve the pre-merge workbook at {}: {error}. Nothing was written to \
             the workbook.",
            destination.display()
        ))
    })?;
    verify_preserved_copy(source, &destination)?;
    Ok(destination)
}

/// Prove the preserved copy is a whole, identical copy before anything is
/// overwritten.
///
/// A copy that is a different length is a copy that failed halfway, and a failed
/// halfway copy is worse than no copy at all: it looks like a backup and is not
/// one. A full byte comparison is cheap next to a term of marks.
fn verify_preserved_copy(source: &Path, copy: &Path) -> Result<()> {
    let source_len = std::fs::metadata(source)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let copy_len = std::fs::metadata(copy).map(|meta| meta.len()).unwrap_or(0);
    if source_len == 0 || source_len != copy_len {
        return Err(AppError::Internal(format!(
            "the preserved copy at {} is {copy_len} bytes and the original is {source_len}. \
             Nothing was written to the workbook.",
            copy.display()
        )));
    }
    let source_bytes = std::fs::read(source).map_err(|error| {
        AppError::Internal(format!(
            "could not read {} to verify the copy: {error}",
            source.display()
        ))
    })?;
    let copy_bytes = std::fs::read(copy).map_err(|error| {
        AppError::Internal(format!(
            "could not read {} to verify the copy: {error}",
            copy.display()
        ))
    })?;
    if source_bytes != copy_bytes {
        return Err(AppError::Internal(format!(
            "the preserved copy at {} is not identical to the original. Nothing was written to \
             the workbook.",
            copy.display()
        )));
    }
    Ok(())
}

/// Fold anything a stray per-month `.xls` holds into the single file, then leave
/// the files alone.
///
/// The twelve-file model is retired, but a build that shipped it may already have
/// written `SF2-SEPTEMBER-2026.xls` and a user may have recorded marks in it
/// directly in Excel. Those marks are read and offered to the caller; the file
/// itself is **never deleted** - it may hold something the single file does not,
/// and deleting is how this user lost data once already.
fn fold_stray_month_files(
    workbook_dir: &Path,
    months: &[(String, i32)],
) -> Result<Vec<(String, i32, LegacyMonthSnapshot)>> {
    let mut folded = Vec::new();
    for (report_month, report_year) in months {
        let path = crate::sf2::workbook_files::month_workbook_path(
            workbook_dir,
            report_month,
            *report_year,
        );
        if !path.is_file() {
            continue;
        }
        let one = vec![(report_month.clone(), *report_year)];
        match read_legacy_months(&path, &one) {
            Ok(reads) => {
                for read in reads {
                    if let Some(snapshot) = read.snapshot {
                        folded.push((report_month.clone(), *report_year, snapshot));
                    }
                }
            }
            Err(error) => log::warn!(
                "workbook merge: could not read the old per-month file {}: {error}. The file is \
                 left exactly where it is.",
                path.display()
            ),
        }
    }
    Ok(folded)
}

/// The workbooks backup, taken before anything is written.
///
/// Best effort on purpose. The verified `_legacy/` copy is the artefact that
/// actually protects the marks, and refusing to build because a *backup folder*
/// could not be written would trade a real risk for a paperwork one.
fn take_workbooks_backup(workbook_dir: &Path) -> Option<String> {
    let app_dir = workbook_dir.parent()?;
    match crate::backup::backup_ops::create_workbooks_backup(app_dir) {
        Ok(status) => status.last_workbooks_backup_path,
        Err(error) => {
            log::warn!("workbook merge: the workbooks backup failed, carrying on: {error}");
            None
        }
    }
}

/// The roster mappings the pre-split app had, as per-month mappings.
///
/// The v22 backfill already copied them onto the pre-split template's own month
/// row, and those copies carry whatever DepEd learner ID the student already had.
/// When the backfill found nothing - an install that predates it, or a row that
/// has since been removed - the pre-split table is read directly.
fn reference_mappings(
    pool: &DbPool,
    legacy: &Sf2TemplateRecord,
) -> Result<Vec<Sf2MonthStudentMapping>> {
    let month_repo = Sf2MonthStudentRepo::new(pool.clone());
    let existing = month_repo.for_template(&legacy.id)?;
    if !existing.is_empty() {
        return Ok(existing);
    }

    Ok(Sf2Repository::new(pool.clone())
        .student_mappings_for_template(&legacy.id)?
        .into_iter()
        .map(|mapping| Sf2MonthStudentMapping {
            template_id: legacy.id.clone(),
            student_id: mapping.student_id,
            workbook_name: mapping.workbook_name,
            normalized_name: mapping.normalized_name,
            row_index: mapping.row_index,
            gender_block: mapping.gender_block,
            sf2_learner_id: None,
        })
        .collect())
}

// ── Outcomes ────────────────────────────────────────────────────────────────

fn merge_outcome(
    completed_at: Option<i64>,
    months: Vec<MergeMonthOutcome>,
    workbook: &Path,
    legacy_copy: &Path,
    backup_path: Option<String>,
    absences_outside_school_year: usize,
) -> MergeOutcome {
    let needs_attention = months
        .iter()
        .filter(|month| !month.is_accounted_for())
        .count();
    let verified = verified_count(&months);
    let mut message = merge_summary_message(
        completed_at,
        &months,
        &file_name_of(&legacy_copy.to_string_lossy()),
    );
    if absences_outside_school_year > 0 {
        // Said out loud, because the alternative is a complete-looking workbook
        // and a teacher concluding those absences were never recorded.
        message.push_str(&format!(
            " {absences_outside_school_year} absence(s) the app holds fall in months this school \
             year has no worksheet for; they are in the database and in the reports grid, but this \
             workbook cannot show them."
        ));
    }
    MergeOutcome {
        split_completed_at: completed_at,
        months,
        verified_count: verified,
        needs_attention_count: needs_attention,
        workbook_path: workbook.to_string_lossy().to_string(),
        legacy_file_path: legacy_copy.to_string_lossy().to_string(),
        legacy_backup_path: backup_path,
        absences_outside_school_year,
        message,
    }
}

/// The outcome of a merge that has nothing left to do.
fn already_merged_outcome(
    completed_at: Option<i64>,
    months: &[(String, i32)],
    workbook: &Path,
    file_name: &str,
    workbook_dir: &Path,
) -> MergeOutcome {
    let results = months
        .iter()
        .map(|(month, year)| MergeMonthOutcome {
            report_month: month.to_uppercase(),
            report_year: *year,
            sheet_name: crate::sf2::month::workbook_sheets::month_sheet_name(
                sf2_month_number(month).unwrap_or_default(),
                *year,
            ),
            file_name: file_name.to_string(),
            status: MergeMonthStatus::AlreadyMerged,
            x_marks: 0,
            learner_rows: 0,
            detail: None,
        })
        .collect::<Vec<_>>();
    let legacy_dir = sf2_legacy_workbook_dir(workbook_dir);
    let legacy_name = std::fs::read_dir(&legacy_dir)
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .next()
        })
        .unwrap_or_else(|| "the original workbook".to_string());
    merge_outcome(
        completed_at,
        results,
        workbook,
        &legacy_dir.join(legacy_name),
        None,
        0,
    )
}

// ── Small helpers ───────────────────────────────────────────────────────────

fn canonical_month_name(report_month: &str) -> String {
    sf2_month_number(report_month)
        .map(sf2_month_name)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_default()
}

fn file_name_of(path: &str) -> String {
    Path::new(path).file_name().map_or_else(
        || path.to_string(),
        |name| name.to_string_lossy().to_string(),
    )
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

/// The v24 schema the job's `sheet_name` column depends on.
///
/// Referenced here so the dependency is stated in the module that needs it, and
/// so `cargo` keeps the include alive.
const _: () = {
    let _ = schema_v24::SCHEMA_VERSION;
};

#[cfg(test)]
#[path = "__tests__/merge_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "__tests__/roster_match_tests.rs"]
mod roster_match_tests;
