use crate::domain::error::{AppError, Result};
use crate::domain::models::AttendanceType;
use crate::infrastructure::database::{ClassRepository, DbPool};
use crate::sf2::attendance::attendance_events::{
    has_absent_event_for_day, set_attendance_event_for_day,
};
use crate::sf2::calendar::parse_date;
use crate::sf2::excel;
use crate::sf2::logic::SF2_ABSENT_MARK;
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord};
use crate::sf2::repository::Sf2Repository;
use crate::sf2::sf2_metadata::sf2_date_mappings_for_report_month;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Recorded on every event this module writes, so the audit trail distinguishes
/// marks recovered from the workbook from marks typed in the app.
const IMPORT_REASON: &str = "SF2 workbook import";

/// Result of reading "X" marks back out of the SF2 working workbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2AttendanceImportOutcome {
    pub class_id: String,
    pub report_month: String,
    /// Learner-row × date-column cells inspected in the workbook.
    pub scanned_cells: usize,
    /// Absences newly written to the database.
    pub imported: usize,
    /// Workbook "X" cells the database already knew about (re-run is a no-op).
    pub already_recorded: usize,
    /// Distinct school days that had at least one "X" in the workbook.
    pub dates_with_marks: usize,
    /// Mapped learner rows the workbook grid was checked for.
    pub mapped_rows: usize,
    /// Mapped date columns the workbook grid was checked for.
    pub mapped_dates: usize,
}

/// Import absence ("X") marks from the SF2 working workbook into the database.
///
/// The workbook is the school's official record, so it is the only surviving
/// copy of a day's absences when the app's database has been reset — names and
/// SF2 details are rebuilt from the workbook on import, but attendance marks
/// were historically write-only. This closes that gap.
///
/// The scan is **additive only**: an "X" becomes an `absent` event unless the
/// database already records that learner absent for that day. Marks present in
/// the database but absent from the workbook are never removed here; the next
/// workbook sync reconciles the two directions, after which `last_synced_at`
/// is cleared so that sync actually runs.
pub fn import_absent_marks_from_workbook(
    pool: DbPool,
    class_id: &str,
) -> Result<Sf2AttendanceImportOutcome> {
    let sf2_repo = Sf2Repository::new(pool.clone());
    let template = sf2_repo
        .latest_template_for_class(class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("No SF2 template imported for this class".to_string())
        })?;

    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let date_mappings = sf2_date_mappings_for_report_month(
        &template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    if student_mappings.is_empty() || date_mappings.is_empty() {
        return Ok(Sf2AttendanceImportOutcome {
            class_id: class_id.to_string(),
            report_month: template.report_month.clone(),
            scanned_cells: 0,
            imported: 0,
            already_recorded: 0,
            dates_with_marks: 0,
            mapped_rows: 0,
            mapped_dates: date_mappings.len(),
        });
    }

    let workbook_path = PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let class = ClassRepository::new(pool.clone())
        .get(class_id)?
        .ok_or_else(|| AppError::InvalidInput("Selected class was not found".to_string()))?;

    let cells = grid_cells_to_scan(&student_mappings, &date_mappings);
    let values = excel::read_cell_texts(&workbook_path, &cells)?;

    let mut imported = 0usize;
    let mut already_recorded = 0usize;
    let mut dates_with_marks: BTreeSet<&str> = BTreeSet::new();

    for date in &date_mappings {
        for mapping in &student_mappings {
            let key = (
                date.sheet_name.clone(),
                format!("{}{}", date.column_letter, mapping.row_index),
            );
            let Some(text) = values.get(&key) else {
                continue;
            };
            if !is_absent_mark(text) {
                continue;
            }

            let parsed_date = parse_date(&date.date)?;
            if has_absent_event_for_day(&pool, &mapping.student_id, class_id, parsed_date)? {
                already_recorded += 1;
            } else {
                set_attendance_event_for_day(
                    pool.clone(),
                    &mapping.student_id,
                    class_id,
                    parsed_date,
                    &class.day_start,
                    AttendanceType::Absent,
                    IMPORT_REASON,
                )?;
                imported += 1;
            }
            dates_with_marks.insert(date.date.as_str());
        }
    }

    // The workbook and the database now agree, but `last_synced_at` still
    // claims the workbook is current. Clearing it makes the next SF2 open
    // rewrite the grid from the (now complete) database instead of skipping.
    if imported > 0 {
        sf2_repo.set_last_synced_at(&template.id, None)?;
    }

    Ok(Sf2AttendanceImportOutcome {
        class_id: class_id.to_string(),
        report_month: template.report_month.clone(),
        scanned_cells: cells.len(),
        imported,
        already_recorded,
        dates_with_marks: dates_with_marks.len(),
        mapped_rows: student_mappings.len(),
        mapped_dates: date_mappings.len(),
    })
}

/// Does this workbook cell text count as an absence?
///
/// Compared against the same constant the writer uses, so a change to the
/// mark written to Excel automatically changes what is read back.
pub(crate) fn is_absent_mark(text: &str) -> bool {
    text.trim().eq_ignore_ascii_case(SF2_ABSENT_MARK)
}

/// Every `(sheet, address)` pair the import needs to inspect: one cell per
/// mapped learner row × mapped date column, de-duplicated.
fn grid_cells_to_scan(
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> Vec<(String, String)> {
    let mut seen = BTreeSet::new();
    let mut cells = Vec::with_capacity(student_mappings.len() * date_mappings.len());
    for date in date_mappings {
        for mapping in student_mappings {
            if mapping.row_index == 0 {
                continue;
            }
            let address = format!("{}{}", date.column_letter, mapping.row_index);
            if seen.insert((date.sheet_name.clone(), address.clone())) {
                cells.push((date.sheet_name.clone(), address));
            }
        }
    }
    cells
}

#[cfg(test)]
#[path = "../__tests__/attendance_import_tests.rs"]
mod tests;
