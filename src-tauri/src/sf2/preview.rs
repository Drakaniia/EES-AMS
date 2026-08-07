use crate::domain::error::Result;
use crate::domain::models::{AttendanceEvent, Student, StudentGender};
use crate::sf2::attendance::{absent_events_for_day, absent_student_ids, present_events_for_day};
use crate::sf2::logic::normalize_learner_name;
use crate::sf2::models::{
    Sf2ExportPreview, Sf2ExportReadiness, Sf2PreviewAbsence, Sf2PreviewCell, Sf2PreviewCellStatus,
    Sf2PreviewDate, Sf2PreviewStudentRow, Sf2StudentMappingRecord, Sf2TemplateRecord,
};
use crate::sf2::repository::template_summary;
use std::collections::{HashMap, HashSet};

/// Build an export preview from pre-queried data (no DB queries inside).
/// This eliminates the duplicate DB round-trips that the previous flow had
/// (export_readiness queried everything, then this function queried it all
/// again independently).
pub(super) fn export_preview(
    template: &Sf2TemplateRecord,
    student_mappings: &[Sf2StudentMappingRecord],
    dates: &[Sf2PreviewDate],
    class_name: &str,
    class_students: &[Student],
    events: &[AttendanceEvent],
    readiness: Sf2ExportReadiness,
) -> Result<Sf2ExportPreview> {
    let student_lookup = class_students
        .iter()
        .map(|student| (student.id.to_string(), student))
        .collect::<HashMap<_, _>>();

    // Students with an explicit absent record per day. An absent cell (X) is
    // driven ONLY by these records - untouched students stay blank (present by
    // default), so other students are never affected by a single absent mark.
    let absent_by_day = dates
        .iter()
        .map(|date| {
            (
                date.date.clone(),
                absent_student_ids(
                    events,
                    class_students,
                    &template.active_class_id,
                    &date.date,
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    // Days with at least one attendance record (an 'in' OR an explicit 'absent'
    // event). Days with no records stay 'open' in the preview - attendance was
    // never taken, so every cell is editable and nothing is marked.
    let has_attendance_by_day = dates
        .iter()
        .map(|date| {
            let day = &date.date;
            let has_present =
                !present_events_for_day(events, class_students, &template.active_class_id, day)
                    .is_empty();
            let has_absent =
                !absent_events_for_day(events, class_students, &template.active_class_id, day)
                    .is_empty();
            (day.clone(), has_present || has_absent)
        })
        .collect::<HashMap<_, _>>();

    let mut warnings = readiness.warnings.clone();
    let mut students = Vec::new();
    let mut absent_list = Vec::new();
    let mut present_count = 0;
    let mut absence_count = 0;
    let mut mapped_student_ids = HashSet::new();

    for mapping in student_mappings {
        mapped_student_ids.insert(mapping.student_id.clone());
        let student = student_lookup.get(&mapping.student_id);
        let student_name = student
            .map(|student| student.name.trim().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| mapping.workbook_name.clone());
        let mut row_warnings = Vec::new();
        if student.is_none() {
            row_warnings.push(
                "This SF2 row points to a student record that is no longer in the class."
                    .to_string(),
            );
            warnings.push(format!(
                "{} is mapped in the SF2 workbook but is not in the selected class.",
                mapping.workbook_name
            ));
        }

        let mut row_present_count = 0;
        let mut row_absent_count = 0;
        let cells = dates
            .iter()
            .map(|date| {
                let editable = student.is_some();
                let is_absent = absent_by_day
                    .get(&date.date)
                    .is_some_and(|absent| absent.contains(&mapping.student_id));
                let day_has_attendance = has_attendance_by_day
                    .get(&date.date)
                    .copied()
                    .unwrap_or(false);

                let status = preview_cell_status(is_absent, day_has_attendance);

                match status {
                    Sf2PreviewCellStatus::Present => {
                        row_present_count += 1;
                        present_count += 1;
                    }
                    Sf2PreviewCellStatus::Absent => {
                        row_absent_count += 1;
                        absence_count += 1;
                        absent_list.push(Sf2PreviewAbsence {
                            student_id: mapping.student_id.clone(),
                            student_name: student_name.clone(),
                            date: date.date.clone(),
                            row_index: mapping.row_index,
                        });
                    }
                    Sf2PreviewCellStatus::Open => {}
                }

                Sf2PreviewCell {
                    date: date.date.clone(),
                    status,
                    editable,
                }
            })
            .collect::<Vec<_>>();

        students.push(Sf2PreviewStudentRow {
            student_id: mapping.student_id.clone(),
            student_name,
            workbook_name: mapping.workbook_name.clone(),
            gender: preview_gender(student.and_then(|student| student.gender), mapping),
            row_index: mapping.row_index,
            mapped: true,
            present_count: row_present_count,
            absent_count: row_absent_count,
            warnings: row_warnings,
            cells,
        });
    }

    let mut unmapped_student_count = 0;
    for student in class_students
        .iter()
        .filter(|student| !mapped_student_ids.contains(&student.id.to_string()))
    {
        unmapped_student_count += 1;
        warnings.push(format!(
            "{} is in the class roster but is not mapped to an SF2 learner row.",
            student.name
        ));
        students.push(Sf2PreviewStudentRow {
            student_id: student.id.to_string(),
            student_name: student.name.clone(),
            workbook_name: String::new(),
            gender: preview_gender(
                student.gender,
                &Sf2StudentMappingRecord {
                    template_id: template.id.clone(),
                    student_id: student.id.to_string(),
                    workbook_name: student.name.clone(),
                    normalized_name: normalize_learner_name(&student.name),
                    row_index: 0,
                    gender_block: None,
                },
            ),
            row_index: 0,
            mapped: false,
            present_count: 0,
            absent_count: 0,
            warnings: vec![
                "Not linked to an SF2 workbook row. Update the workbook roster before export."
                    .to_string(),
            ],
            cells: dates
                .iter()
                .map(|date| Sf2PreviewCell {
                    date: date.date.clone(),
                    status: Sf2PreviewCellStatus::Present,
                    editable: false,
                })
                .collect(),
        });
    }

    if student_mappings.is_empty() {
        warnings.push("No learners are mapped to this SF2 workbook.".to_string());
    }

    Ok(Sf2ExportPreview {
        template: Some(template_summary(template.clone())),
        class_id: Some(template.active_class_id.clone()),
        class_name: class_name.to_string(),
        source_path: Some(template.source_path.clone()),
        dates: dates.to_vec(),
        students,
        absent_list,
        mapped_students: readiness.mapped_students,
        mapped_dates: readiness.mapped_dates,
        present_count,
        absence_count,
        unmapped_student_count,
        can_export: readiness.issues.is_empty(),
        issues: readiness.issues,
        warnings,
    })
}

/// Determine the cell status for a student on a given day in the SF2 preview.
///
/// - Absent: the student has an explicit absent record → "X"
/// - Present: the day has attendance taken (any record) and the student has no
///   absent record → empty cell (present by default).
/// - Open: the day has no records at all → nothing taken yet, cell is editable.
pub(super) fn preview_cell_status(
    is_absent: bool,
    day_has_attendance: bool,
) -> Sf2PreviewCellStatus {
    if is_absent {
        Sf2PreviewCellStatus::Absent
    } else if day_has_attendance {
        Sf2PreviewCellStatus::Present
    } else {
        Sf2PreviewCellStatus::Open
    }
}

#[cfg(test)]
#[path = "__tests__/preview_tests.rs"]
mod tests;

fn preview_gender(
    gender: Option<StudentGender>,
    mapping: &Sf2StudentMappingRecord,
) -> Option<String> {
    if let Some(gender) = gender {
        return Some(
            match gender {
                StudentGender::Male => "Male",
                StudentGender::Female => "Female",
            }
            .to_string(),
        );
    }

    mapping
        .gender_block
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            if value.eq_ignore_ascii_case("MALE") {
                "Male".to_string()
            } else if value.eq_ignore_ascii_case("FEMALE") {
                "Female".to_string()
            } else {
                value.to_string()
            }
        })
}
