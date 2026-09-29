//! The comparison itself, with no Excel and no database in it.
//!
//! Everything in this file is a function of its arguments, which is what makes
//! the two rules that matter testable without a user's data:
//!
//! * a cell the database cannot produce is never counted as one it has
//!   ([`database_x_cells`]), and
//! * a month that was not measured never reports a count
//!   ([`MonthMarkComparison::unmeasured`]).
//!
//! The workbook side is *not* computed here. It comes from
//! [`crate::sf2::guard::evaluate::measure_workbook_marks`], which is the same
//! bulk range read the destructive-sync guard uses - so "what the diagnostic
//! counted" and "what a write path is allowed to clear" are the same set by
//! construction, and this diagnostic cannot measure a different thing from the
//! one that decides whether clearing is safe.

use super::model::{MarkCell, MarkSourceStatus, MonthMarkComparison, RosterRow};
use super::workbook_probe;
use crate::sf2::attendance_marks::Sf2GridCell;
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord};
use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};

/// Turn a sheet's own day numbers into dated grid columns.
///
/// Returns an empty vector for a sheet whose day grid cannot be resolved, and
/// that emptiness is what the caller reports as [`MarkSourceStatus::NoMappings`]
/// rather than as a month with no absences.
#[must_use]
pub fn build_date_mappings(
    template_id: &str,
    sheet_name: &str,
    report_year: i32,
    report_month: u32,
    day_numbers: &[(u32, u32)],
) -> Vec<Sf2DateMappingRecord> {
    day_numbers
        .iter()
        .filter_map(|(column_index, day)| {
            let date = NaiveDate::from_ymd_opt(report_year, report_month, *day)?;
            Some(Sf2DateMappingRecord {
                template_id: template_id.to_string(),
                sheet_name: sheet_name.to_string(),
                date: date.format("%Y-%m-%d").to_string(),
                column_letter: column_number_to_letter(*column_index),
                column_index: *column_index,
            })
        })
        .collect()
}

/// Turn roster rows into the mapping shape the guard and the writer speak.
#[must_use]
pub fn build_student_mappings(
    template_id: &str,
    roster: &[RosterRow],
) -> Vec<Sf2StudentMappingRecord> {
    roster
        .iter()
        .map(|row| Sf2StudentMappingRecord {
            template_id: template_id.to_string(),
            student_id: row.student_id.clone(),
            workbook_name: row.workbook_name.clone(),
            normalized_name: crate::sf2::logic::normalize_learner_name(&row.workbook_name),
            row_index: row.row_index,
            gender_block: None,
        })
        .collect()
}

/// The cells the database can prove, in the same vocabulary as the workbook scan.
///
/// The join is over the month's own grid: an absence is only a cell if the
/// month has a column for its day **and** the roster has a row for the child.
/// Anything else is a real absence in the database that no grid can hold, and it
/// is counted by the caller rather than quietly becoming a cell - a cell it
/// cannot place would be a mark the app believes it holds and cannot write.
#[must_use]
pub fn database_x_cells(
    roster: &[RosterRow],
    date_mappings: &[Sf2DateMappingRecord],
    absent: &[super::model::AbsentRecord],
) -> Vec<Sf2GridCell> {
    let row_by_student: HashMap<&str, u32> = roster
        .iter()
        .map(|row| (row.student_id.as_str(), row.row_index))
        .collect();
    let column_by_date: HashMap<&str, &Sf2DateMappingRecord> = date_mappings
        .iter()
        .map(|mapping| (mapping.date.as_str(), mapping))
        .collect();

    let mut cells: HashSet<Sf2GridCell> = HashSet::new();
    for record in absent {
        let (Some(row_index), Some(mapping)) = (
            row_by_student.get(record.student_id.as_str()),
            column_by_date.get(record.date.as_str()),
        ) else {
            continue;
        };
        cells.insert(Sf2GridCell {
            sheet_name: mapping.sheet_name.clone(),
            column_letter: mapping.column_letter.clone(),
            row_index: *row_index,
        });
    }
    cells.into_iter().collect()
}

/// Name a cell, so a difference is reported as a child and a day.
#[must_use]
pub fn label(
    cell: &Sf2GridCell,
    roster: &[RosterRow],
    date_mappings: &[Sf2DateMappingRecord],
) -> MarkCell {
    MarkCell {
        student_name: roster
            .iter()
            .find(|row| row.row_index == cell.row_index)
            .map_or_else(
                || format!("learner row {}", cell.row_index),
                |row| row.workbook_name.clone(),
            ),
        date: date_mappings
            .iter()
            .find(|mapping| {
                mapping.sheet_name == cell.sheet_name && mapping.column_letter == cell.column_letter
            })
            .map_or_else(
                || format!("unmapped column {}", cell.column_letter),
                |mapping| mapping.date.clone(),
            ),
        sheet_name: cell.sheet_name.clone(),
        cell_address: cell.address(),
    }
}

/// The two sides of the difference, each sorted so the list reads the same way
/// every run.
#[must_use]
pub fn diff_cells(
    roster: &[RosterRow],
    date_mappings: &[Sf2DateMappingRecord],
    workbook_cells: &[Sf2GridCell],
    database_cells: &[Sf2GridCell],
) -> (Vec<MarkCell>, Vec<MarkCell>) {
    let database: HashSet<&Sf2GridCell> = database_cells.iter().collect();
    let workbook: HashSet<&Sf2GridCell> = workbook_cells.iter().collect();

    let mut only_in_workbook: Vec<MarkCell> = workbook_cells
        .iter()
        .filter(|cell| !database.contains(*cell))
        .map(|cell| label(cell, roster, date_mappings))
        .collect();
    only_in_workbook.sort();
    only_in_workbook.dedup();

    let mut only_in_database: Vec<MarkCell> = database_cells
        .iter()
        .filter(|cell| !workbook.contains(*cell))
        .map(|cell| label(cell, roster, date_mappings))
        .collect();
    only_in_database.sort();
    only_in_database.dedup();

    (only_in_workbook, only_in_database)
}

/// How many absences the database holds on days inside `year-month`.
///
/// String-prefix matching on the `YYYY-MM-DD` dates the reader already
/// produced, which is the same comparison a range filter would make and cannot
/// spill into an adjacent month the way a day-number comparison can.
#[must_use]
pub fn absent_count_in_month(
    absent: &[super::model::AbsentRecord],
    year: i32,
    month: u32,
) -> usize {
    let prefix = format!("{year:04}-{month:02}");
    absent
        .iter()
        .filter(|record| record.date.starts_with(&prefix))
        .count()
}

/// The `X` cells a worksheet holds that fall inside `scope`.
///
/// The same scope [`crate::sf2::attendance_marks::attendance_scope_cells`] gives
/// the destructive-sync guard and the differential clear: the mapped learner rows
/// and the mapped day columns of the month. Sharing it is the point - a mark
/// outside the scope is one the app has no way to reproduce or clear, and
/// counting it here would make every comparison permanently disagree.
///
/// ## Why this exists instead of only
/// [`measure_workbook_marks`](crate::sf2::guard::evaluate::measure_workbook_marks)
///
/// Measured on this project's Excel, `measure_workbook_marks`'s row-mask
/// formula does not evaluate: it is a `TEXTJOIN` with one term per day column
/// (33 of them), and `Application.Evaluate` answers a 33-term `TEXTJOIN` with an
/// error variant. An 8-term one evaluates correctly, and a `COUNTIF` over the
/// same range is right. So `measure_workbook_marks` returns **zero** X cells for
/// a month that demonstrably has them, on this machine - a false zero, from the
/// very function the guard clears the grid on the strength of.
///
/// This module therefore takes the count from the block it has already read in
/// full, and [`crate::sf2::diagnose`] calls `measure_workbook_marks` alongside
/// it purely to *report* the disagreement rather than to hide it.
#[must_use]
pub fn workbook_x_cells_in_scope(
    sheet_marks: &[workbook_probe::RawMark],
    sheet_name: &str,
    scope: &[Sf2GridCell],
) -> Vec<Sf2GridCell> {
    let scope: HashSet<&Sf2GridCell> = scope.iter().collect();
    let mut cells: Vec<Sf2GridCell> = sheet_marks
        .iter()
        .map(|mark| Sf2GridCell {
            sheet_name: sheet_name.to_string(),
            column_letter: column_number_to_letter(mark.column_index),
            row_index: mark.row_index,
        })
        .filter(|cell| scope.contains(cell))
        .collect();
    cells.sort();
    cells.dedup();
    cells
}

/// Assemble a month's comparison from measurements that were actually taken.
///
/// There is no constructor here that produces a count without also producing a
/// [`MarkSourceStatus::Comparable`] and a cell scan, and no constructor that
/// produces [`MarkSourceStatus::Comparable`] without counts. That is the whole
/// design: the two cannot be got out of step.
///
/// The argument list is long and flat on purpose: a struct here would bundle
/// the measurements with the labels taken from the *same* measurements, and a
/// mismatched pair is exactly the mistake the parameter order is meant to make
/// obvious.
#[allow(clippy::too_many_arguments)]
pub fn month_from_measurement(
    report_month: &str,
    report_year: i32,
    sheet_name: &str,
    workbook_path: &str,
    mapping_source: super::model::MappingSource,
    roster_resolution: super::model::RosterResolution,
    roster: &[RosterRow],
    date_mappings: &[Sf2DateMappingRecord],
    workbook_cells: &[Sf2GridCell],
    database_cells: &[Sf2GridCell],
    cells_scanned: usize,
    raw_absent_in_month: usize,
) -> MonthMarkComparison {
    let (cells_only_in_workbook, cells_only_in_database) =
        diff_cells(roster, date_mappings, workbook_cells, database_cells);

    let reason = if cells_only_in_workbook.is_empty() {
        "The database holds every X the workbook shows, on the same cells.".to_string()
    } else {
        format!(
            "{} X mark(s) in this month's sheet have no record in the database.",
            cells_only_in_workbook.len()
        )
    };

    MonthMarkComparison {
        report_month: report_month.to_string(),
        report_year,
        counts: super::model::MarkCounts {
            workbook_x_count: Some(workbook_cells.len()),
            db_absent_count: Some(raw_absent_in_month),
            db_mapped_absent_count: Some(database_cells.len()),
            cells_scanned: Some(cells_scanned),
            ..super::model::MarkCounts::unmeasured()
        },
        cells_only_in_workbook,
        cells_only_in_database,
        source_status: MarkSourceStatus::Comparable,
        reason,
        sheet_name: Some(sheet_name.to_string()),
        workbook_path: Some(workbook_path.to_string()),
        mapping_source,
        roster_resolution,
        roster_rows: roster.len(),
        day_columns: date_mappings.len(),
    }
}

/// 1-based Excel column index to its letter, e.g. `10 -> "J"`.
fn column_number_to_letter(mut column: u32) -> String {
    let mut letter = String::new();
    while column > 0 {
        let modulo = (column - 1) % 26;
        letter.insert(0, (b'A' + u8::try_from(modulo).unwrap_or(0)) as char);
        column = (column - modulo) / 26;
    }
    letter
}
