//! The read-only mark diagnostic (spec §0 A5, brief R1).
//!
//! ## What it answers
//!
//! For each of the school's twelve months: how many `X` marks the workbook
//! holds, how many absences the database holds, and which specific
//! `(learner, day)` cells each side has and the other does not. Plus a verdict,
//! and the row counts of the legacy and per-month mapping tables.
//!
//! ## What it is not allowed to do
//!
//! **It never writes.** Not to the workbook, not to the database. That is not a
//! convention this module follows, it is a property it cannot violate:
//!
//! * the database is opened by [`db_read::read_snapshot`] on its own connection
//!   with `SQLITE_OPEN_READ_ONLY`, so no statement in this module *could* write
//!   - SQLite would refuse it;
//! * the workbook is opened through [`workbook_probe::probe_workbook`], which
//!   calls `with_workbook(path, read_only = true, save_on_close = false)` and
//!   contains no writing call of any kind;
//! * the only other database helper reached from here is
//!   [`crate::backup::fingerprint`]'s *reader*.
//!
//! The tests in `__tests__` assert the first two against a real database file:
//! its bytes and modification time are unchanged after a run, and so is the
//! workbook's.
//!
//! ## Why the workbook is read for itself rather than trusted
//!
//! The stored day grid is exactly what the destructive-sync chain destroys
//! (spec §4 step 3), so a diagnostic that read the grid out of the database
//! would report the damage as though it were the truth. Every worksheet's
//! day-number row is read out of the file instead, and a month whose stored grid
//! names a worksheet the file no longer has is reported as such.
//!
//! ## The false-zero rule
//!
//! A count of `0` is a claim. This module makes it only after a measurement, and
//! every other outcome - no sheet, no grid, no roster, no Excel, no file - is
//! reported as a refusal with a reason. See [`model::MarkCounts`].

pub(crate) mod compare;
pub(crate) mod db_read;
pub(crate) mod model;
pub(crate) mod workbook_probe;

use crate::backup::fingerprint::{load_db_fingerprint, DbFingerprint};
use crate::domain::error::Result;
use crate::sf2::calendar::{sf2_month_name, sf2_report_year};
use crate::sf2::guard::evaluate::measure_workbook_marks;
use crate::sf2::logic::normalize_learner_name;
use model::{
    incomparable_summary, verdict_for, verdict_reason, MappingSource, MarkCell, MarkSourceStatus,
    MonthMarkComparison, RosterResolution, RosterRow, Sf2MarkDiagnostic, UnplacedSheet,
    WorkbookFileReport,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use workbook_probe::{probe_workbook, RawSheet, RawWorkbook};

/// The twelve months of a school year.
const SCHOOL_YEAR_MONTHS: [u32; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

/// The folder inside `sf2-workbooks/` the split keeps the pre-split workbook in
/// (spec §6.1). Looked into as well, so a month whose file only lives there is
/// still measured rather than reported as having no sheet.
const LEGACY_WORKBOOK_DIR: &str = "_legacy";

/// The workbook folder the app uses when the database names no file.
const DEFAULT_WORKBOOK_DIR: &str = "sf2-workbooks";

/// Measure, per month, what the workbook holds against what the database holds.
///
/// `db_path` is the live `attendance.db`. It is opened read-only; the workbook
/// is opened read-only and closed without saving. Nothing else is touched.
pub fn diagnose_sf2_marks(db_path: &Path) -> Result<Sf2MarkDiagnostic> {
    let snapshot = db_read::read_snapshot(db_path)?;
    let app_dir = db_path.parent().unwrap_or_else(|| Path::new("."));
    let school_year = snapshot.school_year().unwrap_or_default().to_string();
    let active_class_id = snapshot.active_class_id().unwrap_or_default().to_string();

    let referenced = referenced_workbook_path(&snapshot);
    let workbook_dir = referenced
        .as_ref()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| app_dir.join(DEFAULT_WORKBOOK_DIR));

    let probes = probe_all(&workbook_candidates(referenced.as_deref(), &workbook_dir));
    let class_absent = class_absentences(&snapshot, &active_class_id);
    let (months, consumed) = measure_months(
        &snapshot,
        &probes,
        referenced.as_deref(),
        &school_year,
        &class_absent,
    );

    let unplaced = unplaced_sheets(&probes, &consumed);
    let unplaced_x_cells: usize = unplaced.iter().map(|sheet| sheet.marks.len()).sum();
    let (verdict, reason) = verdict_for_months(&months, unplaced_x_cells);
    let live = live_fingerprint(&snapshot);
    let guard_agreements: Vec<bool> = months
        .iter()
        .filter_map(|month| month.counts.guard_agrees)
        .collect();
    let recorded = load_db_fingerprint(app_dir).ok().flatten();
    let decrease_notice =
        recorded.and_then(|baseline| DbFingerprint::decrease_notice(&baseline, &live));

    Ok(Sf2MarkDiagnostic {
        generated_at: chrono::Utc::now().timestamp(),
        database_path: db_path.display().to_string(),
        schema_version: snapshot.schema_version,
        workbook_path: referenced.as_ref().map(|path| path.display().to_string()),
        workbook_dir: workbook_dir.display().to_string(),
        active_class_id: snapshot.active_class_id().map(str::to_string),
        school_year: snapshot.school_year().map(str::to_string),
        stored_report_month: snapshot
            .anchor_template()
            .map(|template| template.report_month.clone()),
        mapping_source: snapshot
            .anchor_template()
            .map_or(MappingSource::None, |template| {
                snapshot.roster_for(&template.id).1
            }),
        incomparable_months: months
            .iter()
            .filter(|month| !month.source_status.is_comparable())
            .map(incomparable_summary)
            .collect(),
        months,
        verdict,
        verdict_reason: reason,
        unplaced_sheets: unplaced,
        workbooks: workbook_reports(&probes, referenced.as_deref()),
        tables: snapshot.tables,
        event_counts: snapshot.event_counts,
        total_absent_events: snapshot.total_absent_events,
        absent_events_without_class: (snapshot.total_absent_events - class_absent.len() as i64)
            .max(0),
        recorded_fingerprint: recorded,
        absent_decreased_since_fingerprint: decrease_notice.is_some(),
        fingerprint_decrease_notice: decrease_notice,
        guard_scan_matches_direct_read: (!guard_agreements.is_empty())
            .then(|| guard_agreements.iter().all(|agrees| *agrees)),
    })
}

/// The workbook the database's own template row points at.
fn referenced_workbook_path(snapshot: &db_read::DbSnapshot) -> Option<PathBuf> {
    snapshot
        .anchor_template()
        .map(|template| PathBuf::from(&template.source_path))
        .or_else(|| {
            snapshot
                .month_templates
                .first()
                .map(|template| PathBuf::from(&template.source_path))
        })
        .filter(|path| !path.as_os_str().is_empty())
}

/// Every file worth opening, the referenced one first.
///
/// The others in the directory are not decoration. A month whose marks sit in a
/// working copy the database no longer points at is a month the app cannot see,
/// and reporting it as `NoSheet` would be wrong in the way that matters: it
/// would hide marks that do exist.
///
/// Deliberately *not* filtered on the `.xls` extension. Two of the four files in
/// this install's workbook directory have no extension at all - they are older
/// per-class working copies named after their template id - and one of them
/// holds the only `AUGUST 2026` sheet in the directory, with six X marks in it.
/// An extension filter would have reported August as having no sheet. A file
/// that is not a workbook fails to open and is reported as unreadable, which is
/// the truth about it.
fn workbook_candidates(referenced: Option<&Path>, workbook_dir: &Path) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut push = |path: PathBuf| {
        if !path.is_file() {
            return;
        }
        let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if seen.insert(key) {
            candidates.push(path);
        }
    };

    if let Some(referenced) = referenced {
        push(referenced.to_path_buf());
    }
    for dir in [
        workbook_dir.to_path_buf(),
        workbook_dir.join(LEGACY_WORKBOOK_DIR),
    ] {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut found: Vec<PathBuf> = entries
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .collect();
        found.sort();
        for path in found {
            push(path);
        }
    }
    candidates
}

/// One workbook's probe, successful or not.
struct WorkbookProbe {
    path: PathBuf,
    workbook: Option<RawWorkbook>,
    read_error: Option<String>,
}

impl WorkbookProbe {
    fn sheets(&self) -> &[RawSheet] {
        self.workbook
            .as_ref()
            .map_or(&[], |workbook| workbook.sheets.as_slice())
    }
}

/// Read every candidate workbook, one read-only session each.
///
/// A workbook that will not open does not stop the others: its failure is kept
/// as the [`MarkSourceStatus::ExcelUnavailable`] reason for the months it would
/// have covered, and the rest of the diagnostic carries on. That is the whole
/// difference between "the user has the file open in Excel" and a failed tool.
fn probe_all(candidates: &[PathBuf]) -> Vec<WorkbookProbe> {
    candidates
        .iter()
        .map(|path| match probe_workbook(path) {
            Ok(workbook) => WorkbookProbe {
                path: path.clone(),
                workbook: Some(workbook),
                read_error: None,
            },
            Err(error) => WorkbookProbe {
                path: path.clone(),
                workbook: None,
                read_error: Some(error.to_string()),
            },
        })
        .collect()
}

/// The absences that belong to this workbook's class.
///
/// Same test as `sf2::attendance::event_belongs_to_class`: the event names the
/// class, or the student is in it. An absence that fails both is reported in
/// [`Sf2MarkDiagnostic::absent_events_without_class`] rather than dropped, so it
/// is never quietly folded into a month's number.
fn class_absentences(
    snapshot: &db_read::DbSnapshot,
    active_class_id: &str,
) -> Vec<model::AbsentRecord> {
    let class_students: HashSet<&str> = snapshot
        .students
        .iter()
        .filter(|student| student.class_id.as_deref() == Some(active_class_id))
        .map(|student| student.id.as_str())
        .collect();
    snapshot
        .absent_events
        .iter()
        .filter(|record| {
            record.class_id.as_deref() == Some(active_class_id)
                || class_students.contains(record.student_id.as_str())
        })
        .cloned()
        .collect()
}

/// Walk the twelve months, and report which worksheets each one consumed.
fn measure_months(
    snapshot: &db_read::DbSnapshot,
    probes: &[WorkbookProbe],
    referenced: Option<&Path>,
    school_year: &str,
    class_absent: &[model::AbsentRecord],
) -> (Vec<MonthMarkComparison>, HashSet<(PathBuf, String)>) {
    let mut months = Vec::with_capacity(SCHOOL_YEAR_MONTHS.len());
    let mut consumed: HashSet<(PathBuf, String)> = HashSet::new();

    for month in SCHOOL_YEAR_MONTHS {
        let name = sf2_month_name(month);
        let comparison = match locate_sheet(probes, month) {
            None => unmeasured_month(probes, referenced, school_year, month, name),
            Some((path, sheet)) => {
                consumed.insert((path.clone(), sheet.sheet_name.clone()));
                measure_one_month(
                    snapshot,
                    referenced,
                    school_year,
                    month,
                    name,
                    path,
                    sheet,
                    class_absent,
                )
            }
        };
        months.push(comparison);
    }
    (months, consumed)
}

/// The first worksheet that names `month`, across every workbook, referenced
/// file first.
fn locate_sheet(probes: &[WorkbookProbe], month: u32) -> Option<(&PathBuf, &RawSheet)> {
    probes.iter().find_map(|probe| {
        let sheet = probe.workbook.as_ref()?.sheet_for_month(month)?;
        Some((&probe.path, sheet))
    })
}

/// The month's status when no worksheet anywhere names it.
fn unmeasured_month(
    probes: &[WorkbookProbe],
    referenced: Option<&Path>,
    school_year: &str,
    month: u32,
    name: &str,
) -> MonthMarkComparison {
    let year = sf2_report_year(school_year, month);
    let reference_missing = referenced.is_none_or(|path| !path.exists());

    let (status, reason) = if reference_missing && probes.is_empty() {
        (
            MarkSourceStatus::WorkbookMissing,
            format!(
                "The database names no SF2 workbook on disk ({}), so no month could be measured.",
                referenced
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "no template row at all".to_string())
            ),
        )
    } else if let Some(failures) = excel_failures(probes) {
        (
            MarkSourceStatus::ExcelUnavailable,
            format!(
                "Excel could not read {} file(s) in the workbook directory, so a worksheet for \
                 {name} {year} may exist behind one of them: {failures}. This is not a statement \
                 about the month's marks.",
                probes.iter().filter(|p| p.read_error.is_some()).count()
            ),
        )
    } else {
        let examined = probes
            .iter()
            .filter_map(|probe| probe.workbook.as_ref())
            .map(|workbook| workbook.sheets.len())
            .sum::<usize>();
        (
            MarkSourceStatus::NoSheet,
            format!(
                "No worksheet in any of the {} workbook(s) examined ({examined} sheet(s) read \
                 across all of them) names {name} {year}. The workbook holds no sheet for this \
                 month, so nothing could be compared - which is not the same as it holding no \
                 marks.",
                probes.len()
            ),
        )
    };
    MonthMarkComparison::unmeasured(name, year, status, reason)
}

/// Every workbook Excel would not open, one line each, or `None` when it opened
/// all of them.
///
/// All of them and not the first: a month is only `ExcelUnavailable` when *no*
/// readable file names it, and the user needs to know which files were not read
/// before they trust the rest.
fn excel_failures(probes: &[WorkbookProbe]) -> Option<String> {
    let failures: Vec<String> = probes
        .iter()
        .filter_map(|probe| {
            probe
                .read_error
                .as_ref()
                .map(|error| format!("{}: {error}", probe.path.display()))
        })
        .collect();
    (!failures.is_empty()).then(|| failures.join(" | "))
}

/// Measure one month against the database.
#[allow(clippy::too_many_arguments)]
fn measure_one_month(
    snapshot: &db_read::DbSnapshot,
    referenced: Option<&Path>,
    school_year: &str,
    month: u32,
    name: &str,
    path: &Path,
    sheet: &RawSheet,
    class_absent: &[model::AbsentRecord],
) -> MonthMarkComparison {
    let is_referenced = referenced.is_some_and(|reference| same_file(reference, path));
    let year = sheet
        .year_from_name
        .unwrap_or_else(|| sf2_report_year(school_year, month));
    let workbook_path = path.display().to_string();
    let template_id = anchor_template_id(snapshot);

    if sheet.day_numbers.is_empty() {
        return unmeasured(
            name,
            year,
            MarkSourceStatus::NoMappings,
            format!(
                "Worksheet '{}' holds {} X cell(s) but its day-number row is empty, so no cell on \
                 it can be resolved to a date. The marks are there; nothing can place them.",
                sheet.sheet_name,
                sheet.mark_count()
            ),
            sheet,
            &workbook_path,
        );
    }

    let (roster, mapping_source, roster_resolution) =
        roster_for(sheet, snapshot, &template_id, is_referenced);
    if roster.is_empty() {
        return unmeasured(
            name,
            year,
            MarkSourceStatus::NoMappings,
            format!(
                "Worksheet '{}' has a day grid of {} column(s) but no learner could be placed on a \
                 row, so no cell on it can be compared.",
                sheet.sheet_name,
                sheet.day_numbers.len()
            ),
            sheet,
            &workbook_path,
        );
    }

    let date_mappings = compare::build_date_mappings(
        &template_id,
        &sheet.sheet_name,
        year,
        month,
        &sheet.day_numbers,
    );
    let student_mappings = compare::build_student_mappings(&template_id, &roster);

    // The workbook side of the comparison, from the block the probe has already
    // read in full, over exactly the scope the guard and the differential clear
    // use. Not from `measure_workbook_marks` - see
    // `compare::workbook_x_cells_in_scope` for why, and
    // `guard_cross_check` below for how the disagreement is reported rather
    // than hidden.
    let scope =
        crate::sf2::attendance_marks::attendance_scope_cells(&student_mappings, &date_mappings);
    let workbook_cells =
        compare::workbook_x_cells_in_scope(&sheet.marks, &sheet.sheet_name, &scope);

    let guard = guard_cross_check(path, &student_mappings, &date_mappings);

    let database_cells = compare::database_x_cells(&roster, &date_mappings, class_absent);
    let mut comparison = compare::month_from_measurement(
        name,
        year,
        &sheet.sheet_name,
        &workbook_path,
        mapping_source,
        roster_resolution,
        &roster,
        &date_mappings,
        &workbook_cells,
        &database_cells,
        scope.len(),
        compare::absent_count_in_month(class_absent, year, month),
    );
    comparison.counts.guard_x_count = guard.as_ref().map(|count| count.len());
    comparison.counts.guard_agrees = guard
        .as_ref()
        .map(|count| count.len() == workbook_cells.len());
    if matches!(comparison.counts.guard_agrees, Some(false)) {
        comparison.reason.push_str(&format!(
            " NOTE: the destructive-sync guard's own read (measure_workbook_marks) found {} X \
             cell(s) where a direct read of the same cells found {}. The guard cannot currently \
             see this month's marks.",
            guard.as_ref().map_or(0, Vec::len),
            workbook_cells.len()
        ));
    }
    comparison.roster_rows = roster.len();
    comparison
}

/// What `measure_workbook_marks` - the guard's own bulk range read - sees for
/// the same month, the same scope and the same file.
///
/// Called for the report, not for the answer. A disagreement is the single most
/// important thing this diagnostic can surface: the guard's `Proven` verdict is
/// `db_count >= workbook_count` plus cell containment, and a guard that reads
/// zero X cells satisfies both for *any* month, so it would clear the grid on a
/// count of nothing. See [`compare::workbook_x_cells_in_scope`].
fn guard_cross_check(
    path: &Path,
    student_mappings: &[crate::sf2::models::Sf2StudentMappingRecord],
    date_mappings: &[crate::sf2::models::Sf2DateMappingRecord],
) -> Option<Vec<crate::sf2::attendance_marks::Sf2GridCell>> {
    // A guard that cannot read the month at all resolves to `Unmeasured`,
    // which is the safe outcome. Recorded as absent, never as zero.
    measure_workbook_marks(path, student_mappings, date_mappings)
        .x_cells()
        .map(<[crate::sf2::attendance_marks::Sf2GridCell]>::to_vec)
}

/// An unmeasured month that still carries the worksheet it was refused on.
fn unmeasured(
    name: &str,
    year: i32,
    status: MarkSourceStatus,
    reason: String,
    sheet: &RawSheet,
    workbook_path: &str,
) -> MonthMarkComparison {
    MonthMarkComparison {
        sheet_name: Some(sheet.sheet_name.clone()),
        workbook_path: Some(workbook_path.to_string()),
        day_columns: sheet.day_numbers.len(),
        roster_rows: sheet.roster_names.len(),
        ..MonthMarkComparison::unmeasured(name, year, status, reason)
    }
}

/// The id the month's mappings are looked up under.
///
/// The legacy row's id when there is one - it is the row whose `source_path`
/// names the workbook, and the per-month backfill copies that same id - and
/// the first month row's otherwise. Only the id matters here: it keys the
/// roster, and the grid is read out of the file.
fn anchor_template_id(snapshot: &db_read::DbSnapshot) -> String {
    snapshot
        .anchor_template()
        .map(|template| template.id.clone())
        .or_else(|| {
            snapshot
                .month_templates
                .first()
                .map(|template| template.id.clone())
        })
        .unwrap_or_default()
}

/// The roster rows a month is compared with, and where they came from.
///
/// A worksheet on the referenced file uses the database's own row mappings: they
/// were derived from that exact sheet, so the row indices mean the same thing.
/// A worksheet on *any other* file does not - a row index means nothing across
/// workbooks - so its roster is resolved by matching the worksheet's own `NAME`
/// column against the database's students. Getting this wrong would report a
/// month full of differences that are only differences of layout.
fn roster_for(
    sheet: &RawSheet,
    snapshot: &db_read::DbSnapshot,
    template_id: &str,
    is_referenced: bool,
) -> (Vec<RosterRow>, MappingSource, RosterResolution) {
    if is_referenced {
        let (rows, source) = snapshot.roster_for(template_id);
        let resolution = if rows.is_empty() {
            RosterResolution::Unresolved
        } else {
            RosterResolution::DatabaseRowMappings
        };
        return (rows, source, resolution);
    }
    let matched = match_roster_by_name(sheet, snapshot);
    let resolution = if matched.is_empty() {
        RosterResolution::Unresolved
    } else {
        RosterResolution::WorkbookNameMatch
    };
    (matched, MappingSource::None, resolution)
}

/// Match a worksheet's own learner names against the database's students.
fn match_roster_by_name(sheet: &RawSheet, snapshot: &db_read::DbSnapshot) -> Vec<RosterRow> {
    let by_name: HashMap<String, &str> = snapshot
        .students
        .iter()
        .map(|student| (normalize_learner_name(&student.name), student.id.as_str()))
        .collect();
    sheet
        .roster_names
        .iter()
        .filter_map(|row| {
            let student_id = by_name
                .get(&normalize_learner_name(&row.workbook_name))
                .copied()?;
            Some(RosterRow {
                student_id: student_id.to_string(),
                workbook_name: row.workbook_name.clone(),
                row_index: row.row_index,
            })
        })
        .collect()
}

/// Every worksheet that holds marks and was not used for a month.
///
/// This list is what keeps a "the database is complete" answer honest: a
/// worksheet nobody could place is not a worksheet with nothing on it.
fn unplaced_sheets(
    probes: &[WorkbookProbe],
    consumed: &HashSet<(PathBuf, String)>,
) -> Vec<UnplacedSheet> {
    let mut unplaced = Vec::new();
    for probe in probes {
        for sheet in probe.sheets() {
            if consumed.contains(&(probe.path.clone(), sheet.sheet_name.clone()))
                || sheet.mark_count() == 0
            {
                continue;
            }
            unplaced.push(UnplacedSheet {
                workbook_path: probe.path.display().to_string(),
                sheet_name: sheet.sheet_name.clone(),
                visible: sheet.visible,
                unrowed_x_count: sheet.mark_count(),
                reason: unplaced_reason(sheet),
                marks: sheet
                    .marks
                    .iter()
                    .map(|mark| MarkCell {
                        student_name: sheet
                            .name_at(mark.row_index)
                            .unwrap_or("(no name in this row)")
                            .to_string(),
                        date: "(no day grid on this sheet)".to_string(),
                        sheet_name: sheet.sheet_name.clone(),
                        cell_address: mark.address(),
                    })
                    .collect(),
            });
        }
    }
    unplaced
}

/// Why a worksheet holding marks could not be tied to a month.
fn unplaced_reason(sheet: &RawSheet) -> String {
    if sheet.month_from_name.is_none() && sheet.day_numbers.is_empty() {
        return format!(
            "Worksheet '{}' carries {} X cell(s) but its name says no month and its day-number \
             row is empty, so no cell on it can be resolved to a date. Its learner rows also \
             belong to a roster the database does not hold. These cells were not compared with \
             anything.",
            sheet.sheet_name,
            sheet.mark_count()
        );
    }
    if sheet.day_numbers.is_empty() {
        return format!(
            "Worksheet '{}' carries {} X cell(s) but its day-number row is empty, so no cell on \
             it can be resolved to a date.",
            sheet.sheet_name,
            sheet.mark_count()
        );
    }
    format!(
        "Worksheet '{}' was not used for a month, so its {} X cell(s) were not compared.",
        sheet.sheet_name,
        sheet.mark_count()
    )
}

/// One row per candidate workbook.
fn workbook_reports(
    probes: &[WorkbookProbe],
    referenced: Option<&Path>,
) -> Vec<WorkbookFileReport> {
    probes
        .iter()
        .map(|probe| {
            let sheets = probe.sheets();
            WorkbookFileReport {
                path: probe.path.display().to_string(),
                is_referenced_by_database: referenced
                    .is_some_and(|path| same_file(path, &probe.path)),
                sheet_count: sheets.len(),
                month_sheets_measured: sheets
                    .iter()
                    .filter(|sheet| {
                        sheet.month_from_name.is_some() && !sheet.day_numbers.is_empty()
                    })
                    .count(),
                total_x_count: sheets.iter().map(RawSheet::mark_count).sum(),
                read_error: probe.read_error.clone(),
            }
        })
        .collect()
}

/// Do two paths name the same file?
fn same_file(left: &Path, right: &Path) -> bool {
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

/// The counts the update-time gate compares between versions, read from the
/// live database rather than from the sidecar.
fn live_fingerprint(snapshot: &db_read::DbSnapshot) -> DbFingerprint {
    let month_date_mappings = snapshot
        .tables
        .per_month
        .iter()
        .find(|table| table.table == "sf2_month_date_mappings")
        .and_then(|table| table.rows)
        .unwrap_or(0);
    DbFingerprint {
        events: snapshot.event_counts.iter().map(|count| count.rows).sum(),
        absent: snapshot.total_absent_events,
        month_date_mappings,
    }
}

/// `verdict_reason` needs the verdict, and the verdict needs the months; this
/// is the one place both are derived so the two cannot be taken from different
/// versions of the month list.
fn verdict_for_months(
    months: &[MonthMarkComparison],
    unplaced_x_cells: usize,
) -> (model::MarkVerdict, String) {
    let verdict = verdict_for(months);
    (verdict, verdict_reason(verdict, months, unplaced_x_cells))
}

#[cfg(test)]
#[path = "__tests__/diagnose_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "__tests__/real_data_tests.rs"]
mod real_data_tests;
