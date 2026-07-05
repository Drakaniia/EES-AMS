use crate::domain::error::{AppError, Result};
use crate::domain::models::StudentGender;
use crate::infrastructure::database::{
    ClassRepository, DbPool, EventRepository, StudentRepository,
};
use crate::sf2::attendance::present_student_ids;
use crate::sf2::calendar::sf2_date_mappings_for_report_month;
use crate::sf2::logic::normalize_learner_name;
use crate::sf2::models::{
    Sf2ExportPreview, Sf2ExportReadiness, Sf2PreviewAbsence, Sf2PreviewCell, Sf2PreviewCellStatus,
    Sf2PreviewDate, Sf2PreviewStudentRow, Sf2StudentMappingRecord,
};
use crate::sf2::naming::class_name;
use crate::sf2::repository::{template_summary, Sf2Repository};
use chrono::{Local, NaiveDate};
use std::collections::{HashMap, HashSet};

pub(super) fn export_preview(
    pool: DbPool,
    readiness: Sf2ExportReadiness,
) -> Result<Sf2ExportPreview> {
    let Some(summary) = readiness.template.clone() else {
        return Ok(Sf2ExportPreview {
            template: None,
            class_id: None,
            class_name: String::new(),
            source_path: None,
            dates: Vec::new(),
            students: Vec::new(),
            absent_list: Vec::new(),
            mapped_students: readiness.mapped_students,
            mapped_dates: readiness.mapped_dates,
            present_count: 0,
            absence_count: 0,
            unmapped_student_count: 0,
            can_export: false,
            issues: readiness.issues,
            warnings: readiness.warnings,
        });
    };

    let sf2_repo = Sf2Repository::new(pool.clone());
    let template = sf2_repo
        .latest_template_for_class(&summary.class_id)?
        .ok_or_else(|| AppError::InvalidInput("No SF2 template imported".to_string()))?;
    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let date_mappings = sf2_date_mappings_for_report_month(
        &template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );

    let class_repo = ClassRepository::new(pool.clone());
    let class = class_repo.get(&template.active_class_id)?;
    let class_name = class
        .as_ref()
        .map(|class| class.name.clone())
        .unwrap_or_else(|| class_name(&template.grade_level, &template.section));

    let student_repo = StudentRepository::new(pool.clone());
    let class_students = student_repo.list_by_class(Some(&template.active_class_id))?;
    let student_lookup = class_students
        .iter()
        .map(|student| (student.id.to_string(), student))
        .collect::<HashMap<_, _>>();

    let dates = date_mappings
        .iter()
        .map(|mapping| Sf2PreviewDate {
            date: mapping.date.clone(),
            sheet_name: mapping.sheet_name.clone(),
            column_letter: mapping.column_letter.clone(),
            column_index: mapping.column_index,
        })
        .collect::<Vec<_>>();

    let event_repo = EventRepository::new(pool.clone());
    let events = event_repo.list()?;
    let present_by_day = dates
        .iter()
        .map(|date| {
            (
                date.date.clone(),
                present_student_ids(
                    &events,
                    &class_students,
                    &template.active_class_id,
                    &date.date,
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    let mut warnings = readiness.warnings.clone();
    let mut students = Vec::new();
    let mut absent_list = Vec::new();
    let mut present_count = 0;
    let mut absence_count = 0;
    let mut mapped_student_ids = HashSet::new();
    let today = Local::now().date_naive();

    for mapping in &student_mappings {
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
                let is_present = present_by_day
                    .get(&date.date)
                    .is_some_and(|present| present.contains(&mapping.student_id));

                let is_future = NaiveDate::parse_from_str(&date.date, "%Y-%m-%d")
                    .map(|d| d > today)
                    .unwrap_or(false);
                
                let status = if is_present {
                    row_present_count += 1;
                    present_count += 1;
                    Sf2PreviewCellStatus::Present
                } else if is_future {
                    Sf2PreviewCellStatus::Open
                } else {
                    row_absent_count += 1;
                    absence_count += 1;
                    absent_list.push(Sf2PreviewAbsence {
                        student_id: mapping.student_id.clone(),
                        student_name: student_name.clone(),
                        date: date.date.clone(),
                        row_index: mapping.row_index,
                    });
                    Sf2PreviewCellStatus::Absent
                };

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
                    status: Sf2PreviewCellStatus::Absent,
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
        class_id: Some(template.active_class_id),
        class_name,
        source_path: Some(template.source_path),
        dates,
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
