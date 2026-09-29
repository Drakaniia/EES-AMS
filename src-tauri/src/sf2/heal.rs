//! The startup self-heal (spec D6, D8, §8; acceptance #15).
//!
//! This is the last piece of the durability story. Phases 0-5 made the app *stop
//! destroying* X marks: the §9.1 guard refuses to clear a grid the database
//! cannot account for, and §9.2's differential clear computes the cells a sync is
//! allowed to blank from the diff. This module makes the app *recover* the marks
//! a bad update already took, with nobody watching.
//!
//! ## Why it is safe to run unattended (spec §8.3)
//!
//! The recovery is **additive**, and - since the delete-then-insert write it goes
//! through is only additive on a day the app has nothing recorded - it now
//! **refuses to write at all** where the two records disagree. A workbook `X`
//! becomes an `absent` event *unless* the database already records that learner
//! absent for that day ([`has_absent_event_for_day`]) **or** already records them
//! present ([`has_present_event_for_day`]). Nothing here deletes an event, and
//! nothing here writes to the workbook. So the worst a wrong run can do is add an
//! absence the teacher already believes in - never remove one they recorded, and
//! never turn one they made into its opposite.
//!
//! The three properties that make that true, in the order they matter:
//!
//! 1. **The workbook is only ever read.** [`measure_workbook_marks`] opens it
//!    read-only and closes without saving; this module calls no Excel writer at
//!    all. Enforced by a structural test over this file's own source.
//! 2. **The database is only ever added to.** The single write path is
//!    [`set_attendance_event_for_day`] with [`AttendanceType::Absent`], guarded
//!    on both sides - by [`has_absent_event_for_day`] so it is not repeated, and
//!    by [`has_present_event_for_day`] so it never replaces an explicit mark -
//!    and every write is recorded with [`SELF_HEAL_REASON`] so the audit trail
//!    says the app recovered a mark rather than the teacher typing one.
//! 3. **Nothing is measured, so nothing is cleared.** A workbook that cannot be
//!    read resolves to [`Sf2HealOutcome::ExcelUnavailable`], a month row whose
//!    file is gone to [`Sf2HealOutcome::WorkbookMissing`], and a month with no
//!    mappings to [`Sf2HealOutcome::NotApplicable`]. Each is a no-op, and each
//!    leaves §9.1's guard to refuse a write on the next Open/Export.
//!
//! ## Startup is never blocked
//!
//! A COM pass over forty learners is seconds of Excel, not milliseconds of SQL.
//! [`heal_current_month_workbook`] is therefore spawned onto its own thread from
//! `app_lib::run`'s `setup` and **not** joined - see `lib.rs`. The latch in
//! [`HealLatch`] is what makes "once per launch" true even though the caller
//! cannot wait to hear whether it finished.
//!
//! That background thread does *not* make it a free-for-all: §8.2 requires the
//! heal not to run while another Excel task is in flight, so its one COM pass
//! goes through the same process-wide gate as every other COM path - see
//! `sf2::excel::excel_lock`. The gate is taken on the spawned Excel thread, so a
//! heal that arrives while the teacher is opening a workbook **waits for the
//! teacher**; the launch itself is not delayed by either of them.
//!
//! ## The two measurement primitives, and why there is only one
//!
//! The workbook half of the comparison is [`measure_workbook_marks`], the guard's
//! own scanner. This module does not contain a second reader: a second
//! implementation of "which cells hold X" is exactly how "what we measured" and
//! "what we may clear" stop being the same set, which is the failure §9.1 exists
//! to prevent.
//!
//! What the guard's signature does *not* take is a month row. It takes the
//! pre-split shapes [`Sf2StudentMappingRecord`] and [`Sf2DateMappingRecord`],
//! because Phase 5 landed before Phase 2's per-month tables. The reconciliation
//! is [`month_students_as_guard_mappings`] and [`month_dates_as_guard_mappings`]
//! below, and it is lossless in the only direction that matters - see those two
//! functions for what each side of the conversion drops and why.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::domain::error::{AppError, Result};
use crate::domain::models::AttendanceType;
use crate::infrastructure::database::{ClassRepository, DbPool};
use crate::sf2::attendance::attendance_events::{
    has_absent_event_for_day, has_present_event_for_day, set_attendance_event_for_day,
};
use crate::sf2::attendance::attendance_marks::{export_marks, Sf2GridCell};
use crate::sf2::guard;
use crate::sf2::guard::evaluate::measure_workbook_marks;
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord};
use crate::sf2::month::{
    Sf2MonthDateMapping, Sf2MonthDateRepo, Sf2MonthStudentMapping, Sf2MonthStudentRepo,
    Sf2MonthTemplate, Sf2MonthTemplateRepo,
};
use crate::sf2::month_preview::{self, Sf2LaunchMonth};
use crate::sf2::workbook_files::month_workbook_sheet_name;

/// The reason recorded on every absence this module writes.
///
/// Distinct from `attendance_import::IMPORT_REASON` on purpose: that import is a
/// deliberate user action, this one is unattended, and an audit trail that cannot
/// tell them apart is an audit trail that cannot answer "who added this?".
pub const SELF_HEAL_REASON: &str = "SF2 workbook self-heal";

/// The month resolution has nothing to heal.
const NOT_APPLICABLE_NO_CLASS: &str =
    "No class is set up yet, so there is no SF2 workbook to check.";

// ── once per launch (D8) ─────────────────────────────────────────────────────

/// The once-per-launch latch.
///
/// A type rather than a bare `static AtomicBool` so the semantics are testable:
/// a static can only be claimed once per *process*, which is exactly the property
/// under test and therefore useless as a test subject. The production code uses
/// one process-wide instance, [`LAUNCH_LATCH`], and the tests use their own.
#[derive(Debug, Default)]
pub struct HealLatch {
    claimed: AtomicBool,
}

impl HealLatch {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            claimed: AtomicBool::new(false),
        }
    }

    /// Take the one claim this launch is allowed. `true` for the first caller,
    /// `false` for every caller after it.
    ///
    /// `compare_exchange` rather than `swap`: two callers racing at startup must
    /// produce exactly one `true`, and a swap-then-compare would let the loser
    /// of the race see the winner's `true` and run the heal as well.
    pub fn try_claim(&self) -> bool {
        self.claimed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

/// The process-wide latch the startup call goes through.
static LAUNCH_LATCH: HealLatch = HealLatch::new();

/// Claim the launch's single heal run. See [`HealLatch::try_claim`].
#[must_use]
pub fn claim_heal_for_this_launch() -> bool {
    LAUNCH_LATCH.try_claim()
}

/// Start this launch's heal and return immediately (spec §8.2).
///
/// **This is the only place the heal is started, and it never joins the thread
/// it starts.** A COM pass over forty learners is seconds of Excel; running it on
/// the UI thread would turn app launch into a multi-second hang and reintroduce
/// exactly the slowness D9 removed. Nothing downstream may wait on it: the
/// measurement is persisted for §12.2's status line to pick up on its next read,
/// and the toast is an event the frontend may not be listening for yet, which is
/// why nothing here reports a failure upward - there is no upward.
///
/// The claim comes first, so a second call in the same process is a no-op even
/// though the first thread is still running.
pub fn spawn_heal_at_startup<R: tauri::Runtime>(app: tauri::AppHandle<R>, pool: DbPool) {
    if !claim_heal_for_this_launch() {
        log::debug!("SF2 self-heal already ran this launch; not starting another");
        return;
    }
    std::thread::spawn(move || {
        if let Err(error) = heal_current_month_workbook(&app, &pool) {
            log::warn!("SF2 self-heal failed: {error}");
        }
    });
}

// ── step 1 + step 2: which month, and is there a row to heal ────────────────

/// The month row the heal is about, and how it was chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealTarget {
    pub month: String,
    pub report_year: i32,
    pub school_year: String,
    pub class_id: String,
    /// Today's month had nothing on record and this is the last month used
    /// instead (spec D5, edge case E1). The app is showing this month too,
    /// because both go through [`month_preview::launch_month`] - a heal of a
    /// different month than the one on screen would be a heal nobody can see.
    pub fell_back: bool,
}

/// Step 1 and step 2 of §8.2, as one pure decision.
///
/// `launch` is the D5 resolution the app has just used to choose what to display.
/// The heal deliberately does **not** re-resolve the month: two resolutions of
/// the same rule are two answers, and the one that recovers marks for a month the
/// teacher is not looking at is the one that generates a support call.
///
/// `None` means there is no `sf2_month_templates` row for the resolved month, so
/// the heal has nothing to measure. That is the normal state of every month
/// before the split has run, and it is [`Sf2HealOutcome::NotApplicable`], not an
/// error.
#[must_use]
pub fn heal_target(launch: &Sf2LaunchMonth) -> Option<HealTarget> {
    if !launch.has_template {
        return None;
    }
    Some(HealTarget {
        month: launch.month.clone(),
        report_year: launch.report_year,
        school_year: launch.school_year.clone(),
        class_id: launch.class_id.clone(),
        fell_back: launch.fell_back,
    })
}

// ── step 4: the count comparison table, verbatim ─────────────────────────────

/// What the counts say the heal should do (§8.2 step 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealAction {
    /// `db_count > workbook_count`. The database already holds at least as many
    /// absences as the file shows marks.
    ///
    /// "Nothing to do" means nothing to *import* - emphatically not "remove the
    /// surplus". A database ahead of the file is the ordinary state after a
    /// teacher marks someone present in the app before the next export, and it
    /// is the one state in which a naive reconcile would delete real marks.
    NothingToDo,
    /// `db_count == workbook_count`. The measurement is recorded and the run
    /// ends. No import, because by the count there is nothing missing.
    RecordOnly,
    /// `db_count < workbook_count`. The file holds marks the database has no
    /// record of; import them.
    Import { shortfall: usize },
}

/// The §8.2 step-4 comparison table, as a pure function.
///
/// `db_count` and `workbook_count` are both counts of `X` **in the same cell
/// vocabulary** - the mapped learner rows × the mapped day columns of this one
/// month. A raw absent-event count would not be comparable: it would include
/// learners the month file does not have a row for, and days the grid has no
/// column for, and the comparison would then disagree with §9.1's, which is the
/// one that decides whether a write is allowed.
#[must_use]
pub fn decide_heal(db_count: usize, workbook_count: usize) -> HealAction {
    match db_count.cmp(&workbook_count) {
        std::cmp::Ordering::Greater => HealAction::NothingToDo,
        std::cmp::Ordering::Equal => HealAction::RecordOnly,
        std::cmp::Ordering::Less => HealAction::Import {
            shortfall: workbook_count - db_count,
        },
    }
}

// ── the month records → the guard's legacy shapes ──────────────────────────

/// The month's roster in the shape [`measure_workbook_marks`] takes.
///
/// `sf2_month_student_mappings` is `sf2_student_mappings` plus `sf2_learner_id`,
/// so this conversion drops exactly one column and adds nothing. The identity
/// that matters - `student_id` ↔ `row_index` - is carried through untouched, and
/// it is the only identity the scanner reads.
///
/// The DepEd learner ID is dropped because the scanner has no use for it: it
/// resolves *cells*, not learners. It stays in the database for the one thing
/// that needs it, `sf2_learner_id` roster matching across a reshuffle.
#[must_use]
pub fn month_students_as_guard_mappings(
    roster: &[Sf2MonthStudentMapping],
) -> Vec<Sf2StudentMappingRecord> {
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

/// The month's day grid in the shape [`measure_workbook_marks`] takes, with the
/// one column it dropped filled back in.
///
/// §6.2 removed `sheet_name` from `sf2_month_date_mappings` on the grounds that
/// a month file has exactly one worksheet, so the sheet is *derivable* rather
/// than stored. The guard's signature predates that and still wants the name
/// spelled out. So it is derived here, from the same
/// [`month_workbook_sheet_name`] the month read and the split use, and from the
/// **stored** `report_month`/`report_year` - never from the clock, for the reason
/// `month_preview`'s module docs give at length: the legacy June year rule and
/// the school-year September rule disagree about AUGUST, and a year recomputed
/// here would point the scanner at a worksheet name the file does not have.
#[must_use]
pub fn month_dates_as_guard_mappings(
    mappings: &[Sf2MonthDateMapping],
    sheet_name: &str,
) -> Vec<Sf2DateMappingRecord> {
    mappings
        .iter()
        .map(|mapping| Sf2DateMappingRecord {
            template_id: mapping.template_id.clone(),
            sheet_name: sheet_name.to_string(),
            date: mapping.date.clone(),
            column_letter: mapping.column_letter.clone(),
            column_index: mapping.column_index,
        })
        .collect()
}

/// The one worksheet of a month file, from its own stored row.
#[must_use]
pub fn month_sheet_name(template: &Sf2MonthTemplate) -> String {
    month_workbook_sheet_name(&template.report_month, template.report_year)
}

// ── the X cells → (student, day) ────────────────────────────────────────────

/// One workbook `X`, resolved to the student and the day it means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealedAbsence {
    pub student_id: String,
    /// `YYYY-MM-DD`.
    pub date: String,
}

/// The workbook's `X` cells the database cannot produce.
///
/// The set difference, in the same cell vocabulary on both sides. This is the
/// list §8.2 step 4 calls "the missing absences", and it is what makes `imported`
/// equal the `shortfall` the count table reported rather than the whole scan.
///
/// The two sides can disagree - the same number of `X` in both places but on
/// entirely different days is the interesting case - and that is precisely why
/// the count is only the *gate*: a `Stale` verdict on counts with nothing to
/// recover on cells is a workbook and a database that have drifted apart, and it
/// is logged as such rather than papered over.
#[must_use]
pub fn missing_x_cells<'a>(
    workbook: &'a [Sf2GridCell],
    database: &[Sf2GridCell],
) -> Vec<&'a Sf2GridCell> {
    let held: std::collections::HashSet<&Sf2GridCell> = database.iter().collect();
    workbook
        .iter()
        .filter(|cell| !held.contains(*cell))
        .collect()
}

/// Resolve the measured `X` cells onto `(student, day)` using the month's own
/// mappings.
///
/// Takes any iterator of borrowed cells, so the same function resolves a whole
/// scan or just the missing part of one - which is what the caller does, and
/// which would otherwise need the cells cloned to change.
///
/// A cell the mappings cannot place is **dropped, not guessed**. Two cases
/// produce one, and neither is recoverable by guessing: a learner row with no
/// mapping (so the app has no student id to write an event for), and a day
/// column with no mapping (so the app has no date). `measure_workbook_marks`
/// already filters the scan to the scope those two mappings define, so in
/// practice this is the belt to its braces - and it is a belt that must be
/// dropped rather than approximated, because an `X` attributed to the wrong
/// learner is a wrong absence nobody asked for.
#[must_use]
pub fn resolve_x_cells<'a, I>(
    sheet_name: &str,
    x_cells: I,
    roster: &[Sf2MonthStudentMapping],
    dates: &[Sf2MonthDateMapping],
) -> Vec<HealedAbsence>
where
    I: IntoIterator<Item = &'a Sf2GridCell>,
{
    let student_by_row: HashMap<u32, &str> = roster
        .iter()
        .filter(|mapping| mapping.row_index > 0)
        .map(|mapping| (mapping.row_index, mapping.student_id.as_str()))
        .collect();
    let date_by_column: HashMap<&str, &str> = dates
        .iter()
        .map(|mapping| (mapping.column_letter.as_str(), mapping.date.as_str()))
        .collect();

    let mut resolved = Vec::new();
    for cell in x_cells {
        if cell.sheet_name != sheet_name {
            continue;
        }
        let (Some(student_id), Some(date)) = (
            student_by_row.get(&cell.row_index).copied(),
            date_by_column.get(cell.column_letter.as_str()).copied(),
        ) else {
            continue;
        };
        resolved.push(HealedAbsence {
            student_id: student_id.to_string(),
            date: date.to_string(),
        });
    }
    resolved
}

// ── the outcome ─────────────────────────────────────────────────────────────

/// The toast of §8.2 step 6, worded as the spec words it.
///
/// `file_name` is the month's own `SF2-SEPTEMBER-2026.xls` - the file the marks
/// were recovered *from*, which is the whole point of naming it.
#[must_use]
pub fn recovered_toast(file_name: &str, imported: usize) -> String {
    format!("Recovered {imported} X marks from {file_name}")
}

/// What one heal run did (spec §8.2). Every variant is a *report*: none of them
/// is an error, because a workbook the app cannot read is a state the app
/// handles, not a failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Sf2HealOutcome {
    /// The launch's single run already happened (D8).
    AlreadyRan,
    /// There is no `sf2_month_templates` row for the resolved month (§8.2
    /// step 2). The state of every month before the split has run.
    #[serde(rename_all = "camelCase")]
    NotApplicable { reason: String },
    /// Excel could not read the workbook - no Excel, the file is locked, the
    /// layout is not what the scanner expects (§8.2, edges E5/E6). The app
    /// starts normally and §9.1's guard takes over on the next Open/Export.
    #[serde(rename_all = "camelCase")]
    ExcelUnavailable { reason: String },
    /// The month row exists but the file is not on disk (edge case E4). Never
    /// measured, never written.
    #[serde(rename_all = "camelCase")]
    WorkbookMissing { reason: String },
    /// `db_count > workbook_count`. The database is ahead of the file. Nothing
    /// was imported and **nothing was removed**.
    #[serde(rename_all = "camelCase")]
    UpToDate {
        month: String,
        db_count: usize,
        workbook_count: usize,
        scanned_at: i64,
    },
    /// `db_count == workbook_count`. The measurement was recorded; nothing was
    /// imported.
    #[serde(rename_all = "camelCase")]
    InSync {
        month: String,
        db_count: usize,
        workbook_count: usize,
        scanned_at: i64,
    },
    /// The file held marks the database did not, and they were recorded
    /// (acceptance #15).
    #[serde(rename_all = "camelCase")]
    Recovered {
        month: String,
        file_name: String,
        /// Absences actually written.
        imported: usize,
        /// Workbook `X` the database already recorded - the idempotent no-ops.
        already_recorded: usize,
        /// Workbook `X` the database answers with an explicit `present`. Left
        /// alone, counted here so the shortfall and the outcome can be
        /// reconciled by whoever reads it.
        skipped_present: usize,
        workbook_count: usize,
        db_count_before: usize,
        db_count_after: usize,
        scanned_at: i64,
        /// §8.2 step 6, ready to hand to the frontend verbatim.
        toast: String,
    },
}

impl Sf2HealOutcome {
    /// What to tell the user, if anything.
    ///
    /// Only a recovery is worth interrupting anyone for. Every other variant is
    /// either the expected state (no row, already ran) or a condition §9.1
    /// already handles on the next Open/Export, and a toast for each of those
    /// would train a teacher to dismiss messages from this app.
    #[must_use]
    pub fn toast(&self) -> Option<&str> {
        match self {
            Self::Recovered { toast, .. } => Some(toast),
            _ => None,
        }
    }
}

// ── the run ─────────────────────────────────────────────────────────────────

/// Compare this month's workbook against the database and recover what the
/// database is missing (spec §8.2, §8.3).
///
/// Blocking: it opens Excel. It must be called off the UI thread - see
/// [`claim_heal_for_this_launch`]'s module docs on the startup spawn.
pub fn heal_current_month_workbook<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    pool: &DbPool,
) -> Result<Sf2HealOutcome> {
    let today = chrono::Local::now().date_naive();
    heal_month_on(app, pool, today)
}

/// [`heal_current_month_workbook`] with "today" injected, so the D5 rule is
/// testable without a clock.
pub fn heal_month_on<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    pool: &DbPool,
    today: chrono::NaiveDate,
) -> Result<Sf2HealOutcome> {
    let workbook_dir = month_preview::workbook_dir(app)?;

    // Step 1. A fresh install has no class at all, which `launch_month`
    // reports as an error rather than a month - and a fresh install has nothing
    // to heal, so that is `NotApplicable`, not a launch-time failure.
    let launch = match month_preview::launch_month(pool, &workbook_dir, None, today) {
        Ok(launch) => launch,
        Err(error) => {
            log::info!("SF2 self-heal skipped: {error}");
            return Ok(Sf2HealOutcome::NotApplicable {
                reason: NOT_APPLICABLE_NO_CLASS.to_string(),
            });
        }
    };
    // Step 2.
    let Some(target) = heal_target(&launch) else {
        return Ok(Sf2HealOutcome::NotApplicable {
            reason: format!("No SF2 workbook is stored for {}.", launch.month),
        });
    };
    log::info!(
        "SF2 self-heal: checking {} for class {} (fell back: {})",
        target.month,
        target.class_id,
        target.fell_back
    );

    let templates = Sf2MonthTemplateRepo::new(pool.clone());
    let template = templates
        .find(&target.class_id, &target.school_year, &target.month)?
        .ok_or_else(|| {
            AppError::Internal(format!(
                "the {} SF2 month row vanished between the launch read and the heal",
                target.month
            ))
        })?;

    let roster = Sf2MonthStudentRepo::new(pool.clone()).for_template(&template.id)?;
    let dates = Sf2MonthDateRepo::new(pool.clone()).for_template(&template.id)?;
    if dates.is_empty() {
        return Ok(Sf2HealOutcome::NotApplicable {
            reason: guard::NO_MAPPED_DATES.to_string(),
        });
    }
    if roster.is_empty() {
        return Ok(Sf2HealOutcome::NotApplicable {
            reason: guard::NO_MAPPED_LEARNERS.to_string(),
        });
    }

    // Edge case E4: the row exists, the file does not. There is nothing to
    // measure, and there is nothing to open.
    let path = Path::new(&template.source_path);
    if !path.is_file() {
        return Ok(Sf2HealOutcome::WorkbookMissing {
            reason: guard::missing_workbook_file_message(&template.report_month),
        });
    }

    // Step 3. One read-only COM pass over the mapped rows x mapped columns.
    let sheet_name = month_sheet_name(&template);
    let scan = measure_workbook_marks(
        path,
        &month_students_as_guard_mappings(&roster),
        &month_dates_as_guard_mappings(&dates, &sheet_name),
    );
    // A workbook that could not be measured is not a workbook with no absences.
    // `x_cells()` is `None` for exactly that case, and this heal must not record
    // a zero or import from one.
    let Some(workbook_cells) = scan.x_cells() else {
        log::warn!(
            "SF2 self-heal could not measure '{}': {}",
            path.display(),
            scan.failure_reason().unwrap_or("unknown reason")
        );
        return Ok(Sf2HealOutcome::ExcelUnavailable {
            reason: guard::WORKBOOK_NOT_READABLE.to_string(),
        });
    };
    let workbook_count = workbook_cells.len();

    // Step 4. The database half, counted in the guard's own cell vocabulary so
    // the two numbers are comparable.
    let db_cells = database_x_cells(pool, &template, &roster, &dates, &sheet_name)?;
    let db_count = db_cells.len();

    let action = decide_heal(db_count, workbook_count);
    let scanned_at = chrono::Utc::now().timestamp();

    // Step 5. Persist the measurement. A month whose file was counted must say
    // so even when nothing was imported - that is what separates "the file has
    // no X" from "the file was never measured", and it is what §12.2's status
    // line reads.
    templates.record_workbook_x_count(&template.id, workbook_count as i64, scanned_at)?;

    match action {
        HealAction::NothingToDo => Ok(Sf2HealOutcome::UpToDate {
            month: template.report_month.clone(),
            db_count,
            workbook_count,
            scanned_at,
        }),
        HealAction::RecordOnly => Ok(Sf2HealOutcome::InSync {
            month: template.report_month.clone(),
            db_count,
            workbook_count,
            scanned_at,
        }),
        // The import is the missing *set*, not every X the file holds. The count
        // table is the gate; the set difference is what it means, and importing
        // only that keeps `imported` equal to the `shortfall` the table reported.
        // `import_recovered_marks` still re-checks each one against the database
        // before writing, so the two are belt and braces rather than either/or.
        HealAction::Import { shortfall } => {
            let missing = missing_x_cells(workbook_cells, &db_cells);
            let RecoveredMarks {
                imported,
                already_recorded,
                skipped_present,
            } = import_recovered_marks(
                pool,
                &template,
                &class_day_start(pool, &template.active_class_id)?,
                &resolve_x_cells(&sheet_name, missing.iter().copied(), &roster, &dates),
            )?;
            log::info!(
                "SF2 self-heal: {} needs recovering for {}, wrote {imported} \
                 ({} already recorded, {skipped_present} left as recorded present, \
                 {shortfall} missing by count)",
                missing.len(),
                template.report_month,
                already_recorded
            );
            if imported + already_recorded + skipped_present != shortfall {
                // The two disagree when the counts and the cells do: the same
                // number of X in both places but on different days, an X the
                // month's mappings cannot place, or an X the database answers
                // with a `present` the workbook disagrees with. Never fatal -
                // the import is additive and the guard is conservative either way -
                // but it is the shape of a workbook and a database that have
                // drifted, and it is worth a line in the log.
                log::warn!(
                    "SF2 self-heal: {} is {} X ahead by count but only {} cells were \
                     unaccounted for; the counts and the cells disagree",
                    template.report_month,
                    shortfall,
                    missing.len()
                );
            }
            let db_count_after =
                match database_x_cells(pool, &template, &roster, &dates, &sheet_name) {
                    Ok(cells) => cells.len(),
                    Err(error) => {
                        // The import itself already succeeded; only the reported
                        // after-count is unavailable, and the arithmetic is exact
                        // because the import is additive.
                        log::warn!(
                            "SF2 self-heal could not recount the database for {}: {error}",
                            template.report_month
                        );
                        db_count + imported
                    }
                };
            let file_name = Path::new(&template.source_path).file_name().map_or_else(
                || template.report_month.clone(),
                |name| name.to_string_lossy().to_string(),
            );
            Ok(Sf2HealOutcome::Recovered {
                toast: recovered_toast(&file_name, imported),
                month: template.report_month.clone(),
                file_name,
                imported,
                already_recorded,
                skipped_present,
                workbook_count,
                db_count_before: db_count,
                db_count_after,
                scanned_at,
            })
        }
    }
}

/// The `X` marks the database proves, in the scanner's own cell vocabulary.
///
/// The same read the §9.1 guard does ([`export_marks`] is the writer's own mark
/// generator, so "what the database holds" and "what a write would put in the
/// file" cannot drift). Nothing is written and no Excel is touched.
fn database_x_cells(
    pool: &DbPool,
    template: &Sf2MonthTemplate,
    roster: &[Sf2MonthStudentMapping],
    dates: &[Sf2MonthDateMapping],
    sheet_name: &str,
) -> Result<Vec<Sf2GridCell>> {
    let days: Vec<String> = dates.iter().map(|m| m.date.clone()).collect();
    let marks = export_marks(
        pool.clone(),
        &template.active_class_id,
        &days,
        &month_students_as_guard_mappings(roster),
        &month_dates_as_guard_mappings(dates, sheet_name),
    )?;
    Ok(marks.iter().filter_map(Sf2GridCell::from_mark).collect())
}

fn class_day_start(pool: &DbPool, class_id: &str) -> Result<String> {
    ClassRepository::new(pool.clone())
        .get(class_id)?
        .map(|class| class.day_start)
        .ok_or_else(|| AppError::InvalidInput("Selected class was not found".to_string()))
}

/// What one import of measured `X` cells did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredMarks {
    /// Absences written.
    pub imported: usize,
    /// Workbook `X` the database already recorded - the idempotent no-ops.
    pub already_recorded: usize,
    /// Workbook `X` over a day the database holds an explicit `present` for.
    /// Left alone on purpose; see [`import_recovered_marks`].
    pub skipped_present: usize,
}

/// The additive import, and the only place this module writes.
///
/// ## Why this is not a call to `attendance_import::import_absent_marks_from_workbook`
///
/// That function is additive in exactly the right way, and it is the right
/// *semantics*. It cannot be the right *call*: it resolves its month through
/// `Sf2Repository::latest_template_for_class`, i.e. the pre-split `sf2_templates`
/// table, and reads the legacy 12-tab workbook behind it. On any install that has
/// run the split there is no such template, so it fails with *"No SF2 template
/// imported for this class"* - the startup heal would report a failure on
/// exactly the installs it exists to protect. And §8.2 step 4 specifies a
/// different `reason` string from the manual import's, so it could not be that
/// function even if it resolved the right month.
///
/// ## What this writes, and the one thing it refuses to write
///
/// The same three lines that make the manual import additive are here, over the
/// month's own mappings and the cells this run already measured:
///
/// 1. [`has_absent_event_for_day`] - the learner is already recorded absent.
/// 2. [`has_present_event_for_day`] - **the guard**. `set_attendance_event_for_day`
///    clears any existing record for that learner and day before inserting, so
///    without this an `X` over a day the app holds a `present` for turns that
///    record into an absence.
/// 3. `set_attendance_event_for_day(.., AttendanceType::Absent, ..)` - the write.
///
/// Step 2 is D1, and the whole spec rests on it: X marks must never be deleted by
/// an app action without the database proving it holds them. §8.3's argument for
/// running this unattended is that the import is *additive only* - and
/// delete-then-insert is not additive, it is additive-and-destructive, which is
/// only invisible when the day is already blank. A workbook `X` over a recorded
/// `present` is the one case where the two disagree, and it is decided here in
/// favour of **skipping the cell**: the `present` stays, the count is reported,
/// and the workbook is left as the app's problem to reconcile. Writing the
/// absence instead would be defensible *with a teacher watching* - the workbook is
/// the school's official record - but an unattended process is not entitled to
/// overrule a mark somebody explicitly made, and the cost of skipping is one
/// wrong absence the teacher can see, against the cost of writing it being an
/// absence nobody asked for.
///
/// Inherited unchanged from `160002d`: a database mark the workbook does not have
/// is never touched, and the workbook itself is only ever read.
fn import_recovered_marks(
    pool: &DbPool,
    template: &Sf2MonthTemplate,
    day_start: &str,
    absences: &[HealedAbsence],
) -> Result<RecoveredMarks> {
    let mut imported = 0usize;
    let mut already_recorded = 0usize;
    let mut skipped_present = 0usize;
    for absence in absences {
        let date = crate::sf2::calendar::parse_date(&absence.date)?;
        if has_absent_event_for_day(pool, &absence.student_id, &template.active_class_id, date)? {
            already_recorded += 1;
            continue;
        }
        if has_present_event_for_day(pool, &absence.student_id, &template.active_class_id, date)? {
            skipped_present += 1;
            log::warn!(
                "SF2 self-heal: {} has an X on {date} but the database records that \
                 learner present; leaving the present in place (D1)",
                absence.student_id
            );
            continue;
        }
        set_attendance_event_for_day(
            pool.clone(),
            &absence.student_id,
            &template.active_class_id,
            date,
            day_start,
            AttendanceType::Absent,
            SELF_HEAL_REASON,
        )?;
        imported += 1;
    }
    Ok(RecoveredMarks {
        imported,
        already_recorded,
        skipped_present,
    })
}

#[cfg(test)]
#[path = "__tests__/heal_tests.rs"]
mod tests;
