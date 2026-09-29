//! The destructive-sync guard (spec §9.1, §9.2; decisions D1, D7).
//!
//! This module exists to make one property true, and it is the reason the whole
//! per-month workbook project exists:
//!
//! > **An X mark in a workbook can only be removed by the user explicitly
//! > marking that student present.**
//!
//! The database is the source of truth and the workbook is a mirror. A write
//! path that blanks the mirror before refilling it turns one failure anywhere
//! upstream into "the marks are gone from both copies, permanently" - spec §4.
//! So clearing is made conditional on evidence, and the evidence has a
//! conservative default.
//!
//! Two independent layers enforce the property, on purpose:
//!
//! 1. **The permit.** A write path evaluates [`SyncPermit`] and acts on
//!    [`action_for`]. [`SyncPermit::Unmeasured`] is the default: a missing file,
//!    an absent Excel, a locked workbook, an empty mapping set - every one of
//!    them resolves to `Unmeasured`, never to [`SyncPermit::Proven`]. **When in
//!    doubt, do not clear.**
//! 2. **The differential clear** (spec §9.2,
//!    [`attendance_marks::differential_clear_marks`]). The cells a sync blanks
//!    are computed from the diff, so a caller that forgot the guard still
//!    cannot wipe the grid. The over-broad clear is structurally impossible
//!    rather than merely guarded.
//!
//! Layer 2 is what makes layer 1's absence survivable, and layer 1 is what
//! stops a legitimate-looking sync from blanking a cell whose absence the user
//! never retracted.

pub(crate) mod evaluate;
pub(crate) mod read_only;

use crate::domain::error::Result;
use crate::sf2::attendance_marks::Sf2GridCell;
use std::collections::HashMap;

/// `Unmeasured` when the month has no mapped attendance dates.
///
/// An empty date-mapping set is the signature of a degenerate workbook analysis
/// (spec §4 step 2): the target month sheet was not visible, or its name could
/// not be parsed, so the analysis returned no dates. Treating that as "the
/// month has no absences" is what turns a failed refresh into a wiped grid.
pub const NO_MAPPED_DATES: &str =
    "This SF2 workbook has no mapped attendance dates, so the app cannot tell which workbook \
     cells belong to which day. Nothing was cleared.";

/// `Unmeasured` when the month has no mapped learners.
pub const NO_MAPPED_LEARNERS: &str =
    "This SF2 workbook has no learners mapped to workbook rows, so the app cannot tell which \
     cells hold attendance. Nothing was cleared.";

/// `Unmeasured` when Excel could not read the workbook at all.
pub const WORKBOOK_NOT_READABLE: &str =
    "The SF2 workbook could not be read, so the app cannot prove the database holds its marks. \
     Nothing was cleared.";

/// May the write path clear this month's attendance grid?
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncPermit {
    /// The database proves it holds every absence the workbook shows.
    ///
    /// `db_count >= workbook_count` alone is not enough to get here - see
    /// [`decide`] - because a database can hold the right *number* of marks on
    /// entirely the wrong days. Both counts are scoped to this month's mapped
    /// learner rows and mapped day columns.
    Proven {
        db_count: usize,
        workbook_count: usize,
    },
    /// The workbook holds X marks the database has no record of. **Do not
    /// clear.** Import the listed absences, then re-evaluate.
    Stale {
        db_count: usize,
        workbook_count: usize,
        /// `(student, date)` for every workbook X the database cannot produce.
        /// Concrete rather than a bare count, because this is the list the
        /// recovery import needs to act on.
        missing: Vec<(String, String)>,
    },
    /// The workbook could not be measured. **Treat as `Stale`: never clear.**
    Unmeasured { reason: String },
}

impl SyncPermit {
    /// The conservative outcome. Every error path funnels through here.
    pub fn unmeasured(reason: impl Into<String>) -> Self {
        Self::Unmeasured {
            reason: reason.into(),
        }
    }
}

/// Names a grid cell, so a missing mark can be reported as a learner and a day
/// rather than as a bare count.
#[derive(Debug, Clone, Default)]
pub struct CellLabels {
    by_cell: HashMap<(String, String), (String, String)>,
}

impl CellLabels {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Teach the label lookup about one cell.
    pub fn insert(
        &mut self,
        cell: &Sf2GridCell,
        student: impl Into<String>,
        date: impl Into<String>,
    ) {
        self.by_cell
            .insert(cell.key(), (student.into(), date.into()));
    }

    /// The `(student, date)` pair for a cell, falling back to the cell's own
    /// coordinates when nothing labelled it. A cell the mappings do not cover
    /// is still reported - a silently shorter list is a silently shorter
    /// recovery.
    #[must_use]
    pub fn label(&self, cell: &Sf2GridCell) -> (String, String) {
        self.by_cell.get(&cell.key()).cloned().unwrap_or_else(|| {
            (
                format!("learner row {}", cell.row_index),
                format!("unmapped column {}", cell.column_letter),
            )
        })
    }
}

/// The whole permit decision, with no Excel and no database in sight.
///
/// `workbook_x_cells` is `None` when the workbook could not be measured at all.
/// That is the single input that produces [`SyncPermit::Unmeasured`], and
/// `reason` is the text the user is shown.
///
/// `Proven` requires **both** halves of D7's condition:
///
/// * `db_count >= workbook_count` - the database holds at least as many
///   absences as the workbook shows X marks, and
/// * every workbook X is one the database can produce.
///
/// The second half is a deliberate strengthening of the count rule. Counts
/// alone do not prove *which* absences are held, and clearing a cell on a
/// count match alone would remove marks the user never retracted - the exact
/// failure this module exists to prevent. Both halves only ever make the
/// answer more conservative, so nothing D7 permits is refused by it.
#[must_use]
pub fn decide(
    db_x_cells: &[Sf2GridCell],
    workbook_x_cells: Option<&[Sf2GridCell]>,
    labels: &CellLabels,
    reason: &str,
) -> SyncPermit {
    let Some(workbook_x_cells) = workbook_x_cells else {
        return SyncPermit::unmeasured(reason);
    };

    let db_count = db_x_cells.len();
    let workbook_count = workbook_x_cells.len();
    let db_x: std::collections::HashSet<&Sf2GridCell> = db_x_cells.iter().collect();

    if db_count >= workbook_count && workbook_x_cells.iter().all(|cell| db_x.contains(cell)) {
        return SyncPermit::Proven {
            db_count,
            workbook_count,
        };
    }

    let missing = workbook_x_cells
        .iter()
        .filter(|cell| !db_x.contains(*cell))
        .map(|cell| labels.label(cell))
        .collect();
    SyncPermit::Stale {
        db_count,
        workbook_count,
        missing,
    }
}

/// What a write path must do with a permit (spec §9.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncAction {
    /// `Proven`: clear the diff and rewrite the grid from the database.
    Rewrite {
        db_count: usize,
        workbook_count: usize,
    },
    /// `Stale`: the workbook is ahead of the database. Import the listed
    /// absences, re-evaluate, and proceed only if that now returns `Proven`.
    ImportThenRecheck {
        db_count: usize,
        workbook_count: usize,
        missing: Vec<(String, String)>,
    },
    /// `Unmeasured`: open the workbook read-only so the user can see their
    /// marks. Never write.
    ReadOnly { reason: String },
    /// The workbook was still ahead after the import. Nothing was cleared, and
    /// the write is refused. Only [`guard_before_write`] can produce this.
    Aborted { message: String },
}

impl SyncAction {
    /// Is this the only action that lets the write path run?
    #[must_use]
    pub fn permits_write(&self) -> bool {
        matches!(self, Self::Rewrite { .. })
    }
}

/// The guard→action table. `Proven` rewrites, `Stale` imports and rechecks,
/// `Unmeasured` opens read-only. Nothing else clears.
#[must_use]
pub fn action_for(permit: &SyncPermit) -> SyncAction {
    match permit {
        SyncPermit::Proven {
            db_count,
            workbook_count,
        } => SyncAction::Rewrite {
            db_count: *db_count,
            workbook_count: *workbook_count,
        },
        SyncPermit::Stale {
            db_count,
            workbook_count,
            missing,
        } => SyncAction::ImportThenRecheck {
            db_count: *db_count,
            workbook_count: *workbook_count,
            missing: missing.clone(),
        },
        SyncPermit::Unmeasured { reason } => SyncAction::ReadOnly {
            reason: reason.clone(),
        },
    }
}

/// Run the guard in front of a write: evaluate, and when the workbook is ahead
/// of the database run the additive workbook→database import and evaluate
/// again.
///
/// Both steps are injected so the loop is exercisable without Excel or a
/// database. The import is the *additive only* import
/// (`attendance_import::import_absent_marks_from_workbook`): an X becomes an
/// `absent` event unless the database already records that learner absent that
/// day. It never removes a database mark, so re-evaluating after it is the only
/// thing that can turn `Stale` into `Rewrite` - the import on its own is not a
/// repair, and neither is a successful call to it.
///
/// The loop runs at most twice. A second `Stale` is a real refusal, not a
/// retry: it means the workbook's marks sit somewhere the import cannot reach.
pub fn guard_before_write<E, I>(mut evaluate: E, mut import_missing: I) -> Result<SyncAction>
where
    E: FnMut() -> SyncPermit,
    I: FnMut(&[(String, String)]) -> Result<()>,
{
    let missing = match action_for(&evaluate()) {
        SyncAction::Rewrite {
            db_count,
            workbook_count,
        } => {
            return Ok(SyncAction::Rewrite {
                db_count,
                workbook_count,
            });
        }
        SyncAction::ReadOnly { reason } => return Ok(SyncAction::ReadOnly { reason }),
        SyncAction::ImportThenRecheck { missing, .. } => missing,
        // `action_for` never produces this, but the match stays total so a
        // future permit variant cannot slip past unhandled.
        SyncAction::Aborted { message } => return Ok(SyncAction::Aborted { message }),
    };

    import_missing(&missing)?;

    Ok(match action_for(&evaluate()) {
        SyncAction::Rewrite {
            db_count,
            workbook_count,
        } => SyncAction::Rewrite {
            db_count,
            workbook_count,
        },
        // The counts agreeing but the measurement failing is not permission to
        // write. Re-check it and fall back to read-only.
        SyncAction::ReadOnly { reason } => SyncAction::ReadOnly { reason },
        SyncAction::ImportThenRecheck { missing, .. } => SyncAction::Aborted {
            message: stale_abort_message(missing.len()),
        },
        SyncAction::Aborted { message } => SyncAction::Aborted { message },
    })
}

/// The refusal shown when the workbook is still ahead after the import.
#[must_use]
pub fn stale_abort_message(unmatched_x_marks: usize) -> String {
    format!(
        "The workbook has {unmatched_x_marks} X marks the app has no record of. \
         Nothing was changed. Restore a backup or run workbook recovery."
    )
}

/// The one guard call every write path makes.
///
/// It lives here, below the command layer and above the individual write paths,
/// because that is the only layer a new write path cannot route around. The
/// export path used to have no guard at all: it evaluated nothing, wrote the
/// marks, copied the file and opened the result. Reaching for the same function
/// is what stops that from happening again - a write path that wants the grid
/// rewritten has to ask this module first (spec §9.1, acceptance #2).
///
/// The `Stale` -> additive import -> re-evaluate loop is
/// [`guard_before_write`]'s, and the import is the additive-only one: it can add
/// absences, it can never remove one, which is exactly why the re-evaluation
/// afterwards is what decides whether the write proceeds.
pub fn run_write_guard(
    pool: &crate::infrastructure::database::DbPool,
    template: &crate::sf2::models::Sf2TemplateRecord,
    student_mappings: &[crate::sf2::models::Sf2StudentMappingRecord],
    date_mappings: &[crate::sf2::models::Sf2DateMappingRecord],
) -> Result<SyncAction> {
    let evaluate =
        || crate::sf2::guard::evaluate::evaluate(pool, template, student_mappings, date_mappings);
    let import_missing = |_missing: &[(String, String)]| {
        crate::sf2::attendance_import::import_absent_marks_from_workbook(
            pool.clone(),
            &template.active_class_id,
        )
        .map(|_| ())
    };
    guard_before_write(evaluate, import_missing)
}

/// The export path's refusal (spec §9.1, acceptance #2).
///
/// *"Same setup, then Review Export: the export is refused with a message naming
/// the count mismatch. No output file is written."* The count is
/// [`stale_abort_message`]'s - how many `X` marks the workbook holds that the
/// database cannot produce - and the export context is spelled out so the user
/// knows no file appeared rather than wondering where it went.
#[must_use]
pub fn export_refused_message(stale_abort: &str) -> String {
    format!("The export was not written. {stale_abort}")
}

/// The E4 refusal: the month's template row exists but the file is not on disk.
/// The guard is `Unmeasured`, so nothing is cleared (spec §13, E4).
#[must_use]
pub fn missing_workbook_file_message(report_month: &str) -> String {
    format!("SF2-{report_month}.xls is missing. Restore it from a backup.")
}

#[cfg(test)]
#[path = "../__tests__/guard_tests.rs"]
mod tests;
