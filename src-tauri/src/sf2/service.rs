use crate::domain::error::{AppError, Result};
use crate::domain::models::{
    AttendanceEvent, Class, CreateClassRequest, CreateStudentRequest, Settings, Student,
    StudentGender, UpdateStudentRequest,
};
use crate::infrastructure::database::{
    ClassRepository, DbPool, EventRepository, SettingsRepository, StudentRepository,
};
use crate::sf2::excel;
use crate::sf2::logic::{
    attendance_marks_for_closed_day, is_learner_name, normalize_learner_name, Sf2AttendanceEvent,
    Sf2CellMark, Sf2StudentMapping,
};
use crate::sf2::models::{
    Sf2CloseDaySummary, Sf2DateMappingRecord, Sf2ExportPreview, Sf2ExportReadiness,
    Sf2ExportResult, Sf2ImportSummary, Sf2PreviewAbsence, Sf2PreviewCell, Sf2PreviewCellStatus,
    Sf2PreviewDate, Sf2PreviewStudentRow, Sf2StudentMappingRecord, Sf2TemplateDraft,
    Sf2TemplateRecord, Sf2WorkbookAnalysis, Sf2WorkbookMetadata, Sf2WorkbookSettings,
};
use crate::sf2::repository::{template_summary, Sf2Repository};
use chrono::{Datelike, Local, NaiveDate, Utc};
use rusqlite::params;
use std::collections::{HashMap, HashSet};
use std::hash::Hasher;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

const BUNDLED_TEMPLATE_BYTES: &[u8] =
    include_bytes!("../../resources/sf2/TEMPLATE_AUTOMATED_SF2.xls");
const SF2_NAME_COLUMN: &str = "C";
const SF2_MALE_ROWS: std::ops::RangeInclusive<u32> = 8..=28;
const SF2_FEMALE_ROWS: std::ops::RangeInclusive<u32> = 30..=48;

pub fn import_workbook(app: tauri::AppHandle, pool: DbPool) -> Result<Sf2ImportSummary> {
    let workbook_path = pick_workbook_path(&app)?;
    let analysis = excel::analyze_workbook(&workbook_path)?;
    let source_hash = file_hash(&workbook_path)?;
    let layout_fingerprint = layout_fingerprint(&analysis);
    let class_name = class_name(&analysis.grade_level, &analysis.section);

    let class_repo = ClassRepository::new(pool.clone());
    let student_repo = StudentRepository::new(pool.clone());
    let sf2_repo = Sf2Repository::new(pool.clone());

    let class = find_or_create_class(&class_repo, &class_name, None)?;
    let existing_students = student_repo.list_by_class(Some(&class.id))?;
    let mut existing_by_name: HashMap<String, Student> = existing_students
        .into_iter()
        .map(|student| (normalize_learner_name(&student.name), student))
        .collect();

    let template_id = sf2_repo
        .find_template(&source_hash, &analysis.grade_level, &analysis.section)?
        .map(|template| template.id)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let working_copy_path =
        copy_workbook_to_app_data(&app, &workbook_path, &template_id, &analysis)?;
    let metadata = metadata_from_analysis(&analysis);
    excel::write_metadata(&working_copy_path, &metadata)?;

    let mut seen_names = HashSet::new();
    let mut student_mappings = Vec::new();
    let mut students_created = 0;
    let mut students_reused = 0;

    for learner in analysis
        .learners
        .iter()
        .filter(|learner| is_learner_name(&learner.name))
    {
        let normalized_name = normalize_learner_name(&learner.name);
        if !seen_names.insert(normalized_name.clone()) {
            continue;
        }
        let learner_gender = StudentGender::from_sf2_block(learner.gender_block.as_deref());

        let student = if let Some(student) = existing_by_name.get(&normalized_name) {
            students_reused += 1;
            let mut student = student.clone();
            if let Some(gender) = learner_gender {
                if student.gender != Some(gender) {
                    student = student_repo.update(
                        student.id,
                        UpdateStudentRequest {
                            name: None,
                            gender: Some(gender),
                            card_serial: None,
                            class_id: None,
                        },
                    )?;
                    existing_by_name.insert(normalized_name.clone(), student.clone());
                }
            }
            student
        } else {
            let created = student_repo.create(CreateStudentRequest {
                name: learner.name.clone(),
                gender: learner_gender,
                card_serial: None,
                class_id: Some(class.id.clone()),
            })?;
            existing_by_name.insert(normalized_name.clone(), created.clone());
            students_created += 1;
            created
        };

        student_mappings.push(Sf2StudentMappingRecord {
            template_id: template_id.clone(),
            student_id: student.id.to_string(),
            workbook_name: learner.name.clone(),
            normalized_name,
            row_index: learner.row_index,
            gender_block: learner.gender_block.clone(),
        });
    }

    let date_mappings: Vec<Sf2DateMappingRecord> = analysis
        .dates
        .iter()
        .map(|date| Sf2DateMappingRecord {
            template_id: template_id.clone(),
            sheet_name: date.sheet_name.clone(),
            date: date.date.clone(),
            column_letter: date.column_letter.clone(),
            column_index: date.column_index,
        })
        .collect();

    let template = Sf2TemplateRecord {
        id: template_id.clone(),
        source_path: working_copy_path.to_string_lossy().to_string(),
        source_hash,
        school_id: metadata.school_id,
        school_name: metadata.school_name,
        school_year: metadata.school_year,
        report_month: metadata.report_month,
        grade_level: metadata.grade_level,
        section: metadata.section,
        adviser_name: metadata.adviser_name,
        school_head_name: metadata.school_head_name,
        layout_fingerprint,
        active_class_id: class.id.clone(),
        imported_at: chrono::Utc::now().timestamp(),
    };

    sf2_repo.upsert_template_with_mappings(&template, &student_mappings, &date_mappings)?;
    let closed_days = sf2_repo.closed_days_for_class(&class.id)?;
    if let Err(error) = write_template_marks_for_days(pool, &template, &closed_days) {
        log::warn!("failed to backfill imported SF2 workbook marks: {error}");
    }

    Ok(Sf2ImportSummary {
        template_id,
        class_id: class.id,
        class_name,
        source_path: working_copy_path.to_string_lossy().to_string(),
        school_year: template.school_year,
        grade_level: template.grade_level,
        section: template.section,
        learners_found: student_mappings.len(),
        students_created,
        students_reused,
        dates_mapped: date_mappings.len(),
    })
}

pub fn create_workbook_from_template<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    pool: DbPool,
    draft: Sf2TemplateDraft,
) -> Result<Sf2ImportSummary> {
    let workbook_dir = sf2_workbook_dir(&app)?;
    create_workbook_from_template_in_dir(&workbook_dir, pool, draft)
}

fn create_workbook_from_template_in_dir(
    workbook_dir: &Path,
    pool: DbPool,
    draft: Sf2TemplateDraft,
) -> Result<Sf2ImportSummary> {
    let metadata = metadata_from_draft(&draft)?;
    let settings = SettingsRepository::new(pool.clone()).get()?;
    let class_repo = ClassRepository::new(pool.clone());
    let class =
        resolve_template_class(&class_repo, draft.class_id.as_deref(), &metadata, &settings)?;

    let student_repo = StudentRepository::new(pool.clone());
    let (students, students_created, students_reused) =
        roster_students_for_draft(&student_repo, &class.id, &draft.learner_names)?;

    let row_slots = template_roster_slots();
    let roster_assignments = template_roster_assignments(&students)?;
    if students.len() > row_slots.len() {
        return Err(AppError::InvalidInput(format!(
            "The bundled SF2 template has {} learner rows, but this class has {} learners",
            row_slots.len(),
            students.len()
        )));
    }

    let temp_template_path =
        write_temp_binary_file("sf2-template", ".xls", BUNDLED_TEMPLATE_BYTES)?;
    let analysis_result = excel::analyze_workbook(&temp_template_path);
    let _ = std::fs::remove_file(&temp_template_path);
    let _base_analysis = analysis_result?;

    let source_hash = format!(
        "bundled-{}-{}",
        hash_bytes(BUNDLED_TEMPLATE_BYTES),
        class.id
    );
    let grade_level = metadata.grade_level.clone();
    let section = metadata.section.clone();

    let sf2_repo = Sf2Repository::new(pool.clone());
    let template_id = sf2_repo
        .find_template(&source_hash, &grade_level, &section)?
        .map(|template| template.id)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let working_copy_path =
        write_bundled_template_to_dir(workbook_dir, &template_id, &grade_level, &section)?;

    excel::write_metadata(&working_copy_path, &metadata)?;
    let analysis = excel::analyze_workbook(&working_copy_path)?;
    validate_configured_calendar(&analysis, &metadata)?;
    let layout_fingerprint = layout_fingerprint(&analysis);
    let roster_marks = roster_name_marks(&analysis, &roster_assignments, &row_slots);
    excel::write_marks(&working_copy_path, &roster_marks)?;

    let mut seen_normalized_names = HashSet::new();
    let student_mappings = roster_assignments
        .iter()
        .map(|assignment| {
            let student = &assignment.student;
            let slot = assignment.slot;
            let normalized_name = unique_normalized_name(
                &mut seen_normalized_names,
                &student.name,
                &student.id.to_string(),
            );
            Sf2StudentMappingRecord {
                template_id: template_id.clone(),
                student_id: student.id.to_string(),
                workbook_name: student.name.clone(),
                normalized_name,
                row_index: slot.row_index,
                gender_block: Some(slot.gender_block.to_string()),
            }
        })
        .collect::<Vec<_>>();

    let date_mappings = analysis
        .dates
        .iter()
        .map(|date| Sf2DateMappingRecord {
            template_id: template_id.clone(),
            sheet_name: date.sheet_name.clone(),
            date: date.date.clone(),
            column_letter: date.column_letter.clone(),
            column_index: date.column_index,
        })
        .collect::<Vec<_>>();

    let template = Sf2TemplateRecord {
        id: template_id.clone(),
        source_path: working_copy_path.to_string_lossy().to_string(),
        source_hash,
        school_id: metadata.school_id,
        school_name: metadata.school_name,
        school_year: metadata.school_year,
        report_month: metadata.report_month,
        grade_level: grade_level.clone(),
        section: section.clone(),
        adviser_name: metadata.adviser_name,
        school_head_name: metadata.school_head_name,
        layout_fingerprint,
        active_class_id: class.id.clone(),
        imported_at: chrono::Utc::now().timestamp(),
    };

    sf2_repo.upsert_template_with_mappings(&template, &student_mappings, &date_mappings)?;
    let closed_days = sf2_repo.closed_days_for_class(&class.id)?;
    if let Err(error) = write_template_marks_for_days(pool, &template, &closed_days) {
        log::warn!("failed to backfill created SF2 workbook marks: {error}");
    }

    Ok(Sf2ImportSummary {
        template_id,
        class_id: class.id,
        class_name: class.name,
        source_path: working_copy_path.to_string_lossy().to_string(),
        school_year: template.school_year,
        grade_level,
        section,
        learners_found: students.len(),
        students_created,
        students_reused,
        dates_mapped: date_mappings.len(),
    })
}

pub fn workbook_settings(pool: DbPool, class_id: Option<String>) -> Result<Sf2WorkbookSettings> {
    let sf2_repo = Sf2Repository::new(pool.clone());
    let template = latest_template_for_request(&sf2_repo, class_id.as_deref())?
        .ok_or_else(|| AppError::InvalidInput("No SF2 workbook imported".to_string()))?;
    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let date_mappings = sf2_repo.date_mappings_for_template(&template.id)?;
    let class_repo = ClassRepository::new(pool);
    let class_name = class_repo
        .get(&template.active_class_id)?
        .map(|class| class.name)
        .unwrap_or_else(|| class_name(&template.grade_level, &template.section));

    Ok(Sf2WorkbookSettings {
        template_id: template.id,
        class_id: template.active_class_id,
        class_name,
        source_path: template.source_path,
        school_id: template.school_id,
        school_name: template.school_name,
        school_year: template.school_year,
        report_month: template.report_month,
        grade_level: template.grade_level,
        section: template.section,
        adviser_name: template.adviser_name,
        school_head_name: template.school_head_name,
        first_school_day: first_school_day_from_mappings(&date_mappings),
        learner_names: student_mappings
            .into_iter()
            .map(|mapping| mapping.workbook_name)
            .collect(),
        dates_mapped: date_mappings.len(),
    })
}

pub fn update_workbook_settings(pool: DbPool, draft: Sf2TemplateDraft) -> Result<Sf2ImportSummary> {
    let metadata = metadata_from_draft(&draft)?;
    let sf2_repo = Sf2Repository::new(pool.clone());
    let class_id = draft
        .class_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidInput("Class is required".to_string()))?;
    let existing = sf2_repo
        .latest_template_for_class(class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("No SF2 workbook imported for this class".to_string())
        })?;

    let workbook_path = PathBuf::from(&existing.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let class = ClassRepository::new(pool.clone())
        .get(class_id)?
        .ok_or_else(|| AppError::InvalidInput("Selected class was not found".to_string()))?;
    let student_repo = StudentRepository::new(pool.clone());
    let (students, students_created, students_reused) =
        roster_students_for_draft(&student_repo, class_id, &draft.learner_names)?;

    let row_slots = template_roster_slots();
    let roster_assignments = template_roster_assignments(&students)?;
    if students.len() > row_slots.len() {
        return Err(AppError::InvalidInput(format!(
            "The bundled SF2 template has {} learner rows, but this class has {} learners",
            row_slots.len(),
            students.len()
        )));
    }

    excel::write_metadata(&workbook_path, &metadata)?;
    let analysis = excel::analyze_workbook(&workbook_path)?;
    validate_configured_calendar(&analysis, &metadata)?;
    let layout_fingerprint = layout_fingerprint(&analysis);
    let roster_marks = roster_name_marks(&analysis, &roster_assignments, &row_slots);
    excel::write_marks(&workbook_path, &roster_marks)?;

    let mut seen_normalized_names = HashSet::new();
    let student_mappings = roster_assignments
        .iter()
        .map(|assignment| {
            let student = &assignment.student;
            let slot = assignment.slot;
            let normalized_name = unique_normalized_name(
                &mut seen_normalized_names,
                &student.name,
                &student.id.to_string(),
            );
            Sf2StudentMappingRecord {
                template_id: existing.id.clone(),
                student_id: student.id.to_string(),
                workbook_name: student.name.clone(),
                normalized_name,
                row_index: slot.row_index,
                gender_block: Some(slot.gender_block.to_string()),
            }
        })
        .collect::<Vec<_>>();

    let date_mappings = analysis
        .dates
        .iter()
        .map(|date| Sf2DateMappingRecord {
            template_id: existing.id.clone(),
            sheet_name: date.sheet_name.clone(),
            date: date.date.clone(),
            column_letter: date.column_letter.clone(),
            column_index: date.column_index,
        })
        .collect::<Vec<_>>();

    let template = Sf2TemplateRecord {
        id: existing.id.clone(),
        source_path: existing.source_path,
        source_hash: existing.source_hash,
        school_id: metadata.school_id,
        school_name: metadata.school_name,
        school_year: metadata.school_year,
        report_month: metadata.report_month,
        grade_level: metadata.grade_level,
        section: metadata.section,
        adviser_name: metadata.adviser_name,
        school_head_name: metadata.school_head_name,
        layout_fingerprint,
        active_class_id: class.id.clone(),
        imported_at: chrono::Utc::now().timestamp(),
    };

    sf2_repo.update_template_with_mappings(&template, &student_mappings, &date_mappings)?;
    let closed_days = sf2_repo.closed_days_for_class(&class.id)?;
    if let Err(error) = write_template_marks_for_days(pool, &template, &closed_days) {
        log::warn!("failed to backfill updated SF2 workbook marks: {error}");
    }

    Ok(Sf2ImportSummary {
        template_id: template.id,
        class_id: class.id,
        class_name: class.name,
        source_path: template.source_path,
        school_year: template.school_year,
        grade_level: template.grade_level,
        section: template.section,
        learners_found: students.len(),
        students_created,
        students_reused,
        dates_mapped: date_mappings.len(),
    })
}

pub fn close_day(
    pool: DbPool,
    class_id: String,
    date: Option<String>,
) -> Result<Sf2CloseDaySummary> {
    let date = date.unwrap_or_else(today_string);
    parse_date(&date)?;

    let sf2_repo = Sf2Repository::new(pool.clone());
    sf2_repo.close_day(&class_id, &date, chrono::Utc::now().timestamp())?;

    let student_repo = StudentRepository::new(pool.clone());
    let event_repo = EventRepository::new(pool.clone());
    let students = student_repo.list_by_class(Some(&class_id))?;
    let present = present_student_ids(&event_repo.list()?, &students, &class_id, &date);

    if let Some(template) = sf2_repo.latest_template_for_class(&class_id)? {
        let closed_days = sf2_repo.closed_days_for_class(&class_id)?;
        let _ = write_template_marks_for_days(pool, &template, &closed_days)?;
    }

    Ok(Sf2CloseDaySummary {
        class_id,
        date,
        present_count: present.len(),
        absent_count: students.len().saturating_sub(present.len()),
    })
}

pub fn export_readiness(pool: DbPool, class_id: Option<String>) -> Result<Sf2ExportReadiness> {
    let sf2_repo = Sf2Repository::new(pool);
    let template = match class_id {
        Some(class_id) if !class_id.is_empty() => sf2_repo.latest_template_for_class(&class_id)?,
        _ => sf2_repo
            .list_templates()?
            .into_iter()
            .next()
            .map(|summary| Sf2TemplateRecord {
                id: summary.id,
                source_path: summary.source_path,
                source_hash: String::new(),
                school_id: summary.school_id,
                school_name: summary.school_name,
                school_year: summary.school_year,
                report_month: summary.report_month,
                grade_level: summary.grade_level,
                section: summary.section,
                adviser_name: summary.adviser_name,
                school_head_name: summary.school_head_name,
                layout_fingerprint: String::new(),
                active_class_id: summary.class_id,
                imported_at: summary.imported_at,
            }),
    };

    let mut issues = Vec::new();
    let Some(template) = template else {
        return Ok(Sf2ExportReadiness {
            template: None,
            closed_days: Vec::new(),
            mapped_students: 0,
            mapped_dates: 0,
            can_export: false,
            issues: vec![
                "Import an SF2 workbook or create one from the bundled template before exporting."
                    .to_string(),
            ],
            warnings: Vec::new(),
        });
    };
    let warnings = sf2_metadata_warnings(&template_metadata(&template));

    if !Path::new(&template.source_path).exists() {
        issues.push(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again."
                .to_string(),
        );
    }

    let closed_days = sf2_closed_days_for_report_month(
        &template,
        &sf2_repo.closed_days_for_class(&template.active_class_id)?,
    );
    let mapped_students = sf2_repo.student_mappings_for_template(&template.id)?.len();

    let date_mappings = sf2_repo.date_mappings_for_template(&template.id)?;
    let mapped_dates = sf2_date_mappings_for_report_month(&template, &date_mappings).len();
    if mapped_dates == 0 {
        issues.push("No attendance dates are mapped to this SF2 report month.".to_string());
    }

    Ok(Sf2ExportReadiness {
        template: Some(template_summary(template)),
        closed_days,
        mapped_students,
        mapped_dates,
        can_export: issues.is_empty(),
        issues,
        warnings,
    })
}

pub fn export_preview(pool: DbPool, class_id: Option<String>) -> Result<Sf2ExportPreview> {
    let readiness = export_readiness(pool.clone(), class_id)?;
    let Some(summary) = readiness.template.clone() else {
        return Ok(Sf2ExportPreview {
            template: None,
            class_id: None,
            class_name: String::new(),
            source_path: None,
            dates: Vec::new(),
            students: Vec::new(),
            absent_list: Vec::new(),
            closed_days: readiness.closed_days,
            mapped_students: readiness.mapped_students,
            mapped_dates: readiness.mapped_dates,
            present_count: 0,
            absence_count: 0,
            unmapped_student_count: 0,
            unmapped_closed_day_count: 0,
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
    let closed_day_set = readiness
        .closed_days
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mapped_day_set = date_mappings
        .iter()
        .map(|mapping| mapping.date.as_str())
        .collect::<HashSet<_>>();

    let dates = date_mappings
        .iter()
        .map(|mapping| Sf2PreviewDate {
            date: mapping.date.clone(),
            sheet_name: mapping.sheet_name.clone(),
            column_letter: mapping.column_letter.clone(),
            column_index: mapping.column_index,
            closed: closed_day_set.contains(mapping.date.as_str()),
        })
        .collect::<Vec<_>>();

    let event_repo = EventRepository::new(pool.clone());
    let events = event_repo.list()?;
    let present_by_day = dates
        .iter()
        .filter(|date| date.closed)
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
                let editable = date.closed && student.is_some();
                let status = if !date.closed {
                    Sf2PreviewCellStatus::Open
                } else if present_by_day
                    .get(&date.date)
                    .is_some_and(|present| present.contains(&mapping.student_id))
                {
                    row_present_count += 1;
                    present_count += 1;
                    Sf2PreviewCellStatus::Present
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
                    status: Sf2PreviewCellStatus::Open,
                    editable: false,
                })
                .collect(),
        });
    }

    let unmapped_closed_days = readiness
        .closed_days
        .iter()
        .filter(|day| !mapped_day_set.contains(day.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    for day in &unmapped_closed_days {
        warnings.push(format!(
            "{day} is closed for attendance but is not mapped to an SF2 date column."
        ));
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
        closed_days: readiness.closed_days,
        mapped_students: readiness.mapped_students,
        mapped_dates: readiness.mapped_dates,
        present_count,
        absence_count,
        unmapped_student_count,
        unmapped_closed_day_count: unmapped_closed_days.len(),
        can_export: readiness.issues.is_empty(),
        issues: readiness.issues,
        warnings,
    })
}

pub fn set_preview_attendance(
    pool: DbPool,
    class_id: String,
    student_id: String,
    date: String,
    present: bool,
) -> Result<Sf2ExportPreview> {
    let date_value = parse_date(&date)?;
    let sf2_repo = Sf2Repository::new(pool.clone());
    let template = sf2_repo
        .latest_template_for_class(&class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("No SF2 template imported for this class".to_string())
        })?;
    let date_mappings = sf2_date_mappings_for_report_month(
        &template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    if !date_mappings
        .iter()
        .any(|mapping| mapping.date.as_str() == date.as_str())
    {
        return Err(AppError::InvalidInput(format!(
            "{date} is not mapped to an SF2 date column"
        )));
    }

    let closed_days =
        sf2_closed_days_for_report_month(&template, &sf2_repo.closed_days_for_class(&class_id)?);
    if !closed_days.iter().any(|day| day == &date) {
        return Err(AppError::InvalidInput(format!(
            "{date} is not a closed SF2 attendance day"
        )));
    }

    let class = ClassRepository::new(pool.clone())
        .get(&class_id)?
        .ok_or_else(|| AppError::InvalidInput("Selected class was not found".to_string()))?;
    let students = StudentRepository::new(pool.clone()).list_by_class(Some(&class_id))?;
    let student = students
        .iter()
        .find(|student| student.id.to_string() == student_id)
        .ok_or_else(|| AppError::InvalidInput("Selected student was not found".to_string()))?;

    set_attendance_event_for_day(
        pool.clone(),
        &student.id.to_string(),
        &class_id,
        date_value,
        &class.day_start,
        present,
    )?;
    write_template_marks_for_days(pool.clone(), &template, &closed_days)?;

    export_preview(pool, Some(class_id))
}

pub fn export_workbook(
    app: tauri::AppHandle,
    pool: DbPool,
    class_id: String,
) -> Result<Sf2ExportResult> {
    let sf2_repo = Sf2Repository::new(pool.clone());
    let template = sf2_repo
        .latest_template_for_class(&class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("No SF2 template imported for this class".to_string())
        })?;

    let output_path = save_workbook_path(&app, &template)?;
    let working_copy_path = PathBuf::from(&template.source_path);
    if working_copy_path == output_path {
        return Err(AppError::InvalidInput(
            "Choose a different output path so the app SF2 working copy is not overwritten"
                .to_string(),
        ));
    }

    if !working_copy_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let closed_days =
        sf2_closed_days_for_report_month(&template, &sf2_repo.closed_days_for_class(&class_id)?);
    let marks_written = write_template_marks_for_days(pool.clone(), &template, &closed_days)?;

    let metadata = template_metadata(&template);
    excel::write_metadata(&working_copy_path, &metadata)?;

    std::fs::copy(&working_copy_path, &output_path)
        .map_err(|error| AppError::Internal(format!("failed to export SF2 workbook: {error}")))?;

    Ok(Sf2ExportResult {
        output_path: output_path.to_string_lossy().to_string(),
        marks_written,
        closed_days: closed_days.len(),
    })
}

pub fn open_workbook(pool: DbPool, class_id: Option<String>) -> Result<String> {
    let sf2_repo = Sf2Repository::new(pool.clone());
    let template = latest_template_for_request(&sf2_repo, class_id.as_deref())?
        .ok_or_else(|| AppError::InvalidInput("No SF2 template imported".to_string()))?;
    let workbook_path = PathBuf::from(&template.source_path);

    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    open_path_in_default_app(&workbook_path)?;
    Ok(workbook_path.to_string_lossy().to_string())
}

fn latest_template_for_request(
    sf2_repo: &Sf2Repository,
    class_id: Option<&str>,
) -> Result<Option<Sf2TemplateRecord>> {
    if let Some(class_id) = class_id.filter(|value| !value.is_empty()) {
        return sf2_repo.latest_template_for_class(class_id);
    }

    Ok(sf2_repo
        .list_templates()?
        .into_iter()
        .next()
        .map(|summary| Sf2TemplateRecord {
            id: summary.id,
            source_path: summary.source_path,
            source_hash: String::new(),
            school_id: summary.school_id,
            school_name: summary.school_name,
            school_year: summary.school_year,
            report_month: summary.report_month,
            grade_level: summary.grade_level,
            section: summary.section,
            adviser_name: summary.adviser_name,
            school_head_name: summary.school_head_name,
            layout_fingerprint: String::new(),
            active_class_id: summary.class_id,
            imported_at: summary.imported_at,
        }))
}

fn export_marks(
    pool: DbPool,
    class_id: &str,
    closed_days: &[String],
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> Result<Vec<Sf2CellMark>> {
    let date_by_day: HashMap<&str, &Sf2DateMappingRecord> = date_mappings
        .iter()
        .map(|mapping| (mapping.date.as_str(), mapping))
        .collect();
    let student_repo = StudentRepository::new(pool.clone());
    let event_repo = EventRepository::new(pool);
    let students = student_repo.list_by_class(Some(class_id))?;
    let events = event_repo.list()?;

    let mut marks = Vec::new();
    for day in closed_days {
        let Some(date_mapping) = date_by_day.get(day.as_str()) else {
            continue;
        };

        let day_students: Vec<Sf2StudentMapping> = student_mappings
            .iter()
            .map(|student| Sf2StudentMapping {
                student_id: student.student_id.clone(),
                sheet_name: date_mapping.sheet_name.clone(),
                row_index: student.row_index,
            })
            .collect();
        let present_events = present_events_for_day(&events, &students, class_id, day);

        marks.extend(attendance_marks_for_closed_day(
            &day_students,
            &present_events,
            &date_mapping.column_letter,
        ));
    }

    Ok(marks)
}

fn write_template_marks_for_days(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    days: &[String],
) -> Result<usize> {
    let workbook_path = PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let sf2_repo = Sf2Repository::new(pool.clone());
    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let date_mappings = sf2_date_mappings_for_report_month(
        template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    if date_mappings.is_empty() {
        return Ok(0);
    }

    let mapped_dates: HashSet<&str> = date_mappings
        .iter()
        .map(|mapping| mapping.date.as_str())
        .collect();
    let export_days = days
        .iter()
        .filter(|day| mapped_dates.contains(day.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    let mut marks = clear_attendance_marks_for_records(&date_mappings, &student_mappings);
    let attendance_marks = if export_days.is_empty() || student_mappings.is_empty() {
        Vec::new()
    } else {
        export_marks(
            pool,
            &template.active_class_id,
            &export_days,
            &student_mappings,
            &date_mappings,
        )?
    };
    let attendance_mark_count = attendance_marks.len();

    marks.extend(attendance_marks);
    excel::write_marks(&workbook_path, &marks)?;
    Ok(attendance_mark_count)
}

fn metadata_from_analysis(analysis: &Sf2WorkbookAnalysis) -> Sf2WorkbookMetadata {
    Sf2WorkbookMetadata {
        school_id: analysis.school_id.trim().to_string(),
        school_name: analysis.school_name.trim().to_string(),
        school_year: analysis.school_year.trim().to_string(),
        report_month: analysis.report_month.trim().to_string(),
        grade_level: analysis.grade_level.trim().to_string(),
        section: analysis.section.trim().to_string(),
        adviser_name: analysis.adviser_name.trim().to_string(),
        school_head_name: analysis.school_head_name.trim().to_string(),
        configure_calendar: false,
        first_school_day: None,
    }
}

fn metadata_from_draft(draft: &Sf2TemplateDraft) -> Result<Sf2WorkbookMetadata> {
    let school_year = required_draft_text(&draft.school_year, "School Year")?;
    let report_month = required_draft_text(&draft.report_month, "Report Month")?;
    let first_school_day =
        required_first_school_day(draft.first_school_day, &report_month, &school_year)?;

    Ok(Sf2WorkbookMetadata {
        school_id: required_draft_text(&draft.school_id, "School ID")?,
        school_name: required_draft_text(&draft.school_name, "Name of School")?,
        school_year,
        report_month,
        grade_level: required_draft_text(&draft.grade_level, "Grade Level")?,
        section: required_draft_text(&draft.section, "Section")?,
        adviser_name: required_draft_text(&draft.adviser_name, "Adviser / LIS Name")?,
        school_head_name: required_draft_text(&draft.school_head_name, "School Head Name")?,
        configure_calendar: true,
        first_school_day: Some(first_school_day),
    })
}

fn required_draft_text(value: &str, label: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput(format!("{label} is required")));
    }
    Ok(trimmed.to_string())
}

fn required_first_school_day(
    day: Option<u32>,
    report_month: &str,
    school_year: &str,
) -> Result<u32> {
    let day = day.ok_or_else(|| {
        AppError::InvalidInput("First attendance day is required for SF2 templates".to_string())
    })?;
    validate_first_school_day(day, report_month, school_year)?;
    Ok(day)
}

fn validate_first_school_day(day: u32, report_month: &str, school_year: &str) -> Result<()> {
    let month = sf2_month_number(report_month).ok_or_else(|| {
        AppError::InvalidInput("Report Month must be a valid month name".to_string())
    })?;
    let year = sf2_report_year(school_year, month);
    let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        let last_day = last_day_of_month(year, month);
        AppError::InvalidInput(format!(
            "First attendance day must be between 1 and {last_day} for this report month"
        ))
    })?;

    if date.weekday().number_from_monday() > 5 {
        return Err(AppError::InvalidInput(
            "First attendance day must be a Monday-Friday school day".to_string(),
        ));
    }

    Ok(())
}

fn validate_configured_calendar(
    analysis: &Sf2WorkbookAnalysis,
    metadata: &Sf2WorkbookMetadata,
) -> Result<()> {
    if !metadata.configure_calendar {
        return Ok(());
    }

    let expected_day = metadata.first_school_day.ok_or_else(|| {
        AppError::InvalidInput("First attendance day is required for SF2 templates".to_string())
    })?;
    let month = sf2_month_number(&metadata.report_month).ok_or_else(|| {
        AppError::InvalidInput("Report Month must be a valid month name".to_string())
    })?;
    let year = sf2_report_year(&metadata.school_year, month);
    let detected_day = analysis
        .dates
        .iter()
        .filter_map(|mapping| parse_date(&mapping.date).ok())
        .filter(|date| date.year() == year && date.month() == month)
        .map(|date| date.day())
        .min();

    match detected_day {
        Some(actual_day) if actual_day == expected_day => Ok(()),
        Some(actual_day) => Err(AppError::Internal(format!(
            "SF2 calendar was not configured correctly: expected first attendance day {expected_day}, but the workbook starts at day {actual_day}"
        ))),
        None => Err(AppError::Internal(
            "SF2 calendar was not configured correctly: no attendance dates were detected"
                .to_string(),
        )),
    }
}

fn sf2_month_number(name: &str) -> Option<u32> {
    let normalized = name.trim().to_ascii_uppercase();
    if normalized.contains("JAN") {
        Some(1)
    } else if normalized.contains("FEB") {
        Some(2)
    } else if normalized.contains("MAR") {
        Some(3)
    } else if normalized.contains("APR") {
        Some(4)
    } else if normalized.contains("MAY") {
        Some(5)
    } else if normalized.contains("JUN") {
        Some(6)
    } else if normalized.contains("JUL") {
        Some(7)
    } else if normalized.contains("AUG") {
        Some(8)
    } else if normalized.contains("SEP") {
        Some(9)
    } else if normalized.contains("OCT") {
        Some(10)
    } else if normalized.contains("NOV") {
        Some(11)
    } else if normalized.contains("DEC") {
        Some(12)
    } else {
        None
    }
}

fn sf2_month_name(month: u32) -> &'static str {
    match month {
        1 => "JANUARY",
        2 => "FEBRUARY",
        3 => "MARCH",
        4 => "APRIL",
        5 => "MAY",
        6 => "JUNE",
        7 => "JULY",
        8 => "AUGUST",
        9 => "SEPTEMBER",
        10 => "OCTOBER",
        11 => "NOVEMBER",
        12 => "DECEMBER",
        _ => "",
    }
}

fn sf2_report_year(_school_year: &str, _month: u32) -> i32 {
    Local::now().year()
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|date| date.pred_opt())
        .map(|date| date.day())
        .unwrap_or(31)
}

fn template_metadata(template: &Sf2TemplateRecord) -> Sf2WorkbookMetadata {
    Sf2WorkbookMetadata {
        school_id: template.school_id.clone(),
        school_name: template.school_name.clone(),
        school_year: template.school_year.clone(),
        report_month: template.report_month.clone(),
        grade_level: template.grade_level.clone(),
        section: template.section.clone(),
        adviser_name: template.adviser_name.clone(),
        school_head_name: template.school_head_name.clone(),
        configure_calendar: false,
        first_school_day: None,
    }
}

fn sf2_date_mappings_for_report_month(
    template: &Sf2TemplateRecord,
    date_mappings: &[Sf2DateMappingRecord],
) -> Vec<Sf2DateMappingRecord> {
    let Some(month) = sf2_month_number(&template.report_month) else {
        return Vec::new();
    };
    let year = sf2_report_year(&template.school_year, month);

    date_mappings
        .iter()
        .filter_map(|mapping| {
            let date = parse_date(&mapping.date).ok()?;
            if date.month() != month {
                return None;
            }

            let normalized_date = NaiveDate::from_ymd_opt(year, month, date.day())?;
            Some(Sf2DateMappingRecord {
                template_id: mapping.template_id.clone(),
                sheet_name: mapping.sheet_name.clone(),
                date: normalized_date.format("%Y-%m-%d").to_string(),
                column_letter: mapping.column_letter.clone(),
                column_index: mapping.column_index,
            })
        })
        .collect()
}

fn sf2_closed_days_for_report_month(
    template: &Sf2TemplateRecord,
    closed_days: &[String],
) -> Vec<String> {
    closed_days
        .iter()
        .filter(|day| sf2_is_report_month_date(template, day))
        .cloned()
        .collect()
}

fn sf2_is_report_month_date(template: &Sf2TemplateRecord, date: &str) -> bool {
    let Some(month) = sf2_month_number(&template.report_month) else {
        return false;
    };
    let year = sf2_report_year(&template.school_year, month);

    parse_date(date).is_ok_and(|date| date.year() == year && date.month() == month)
}

fn resolve_template_class(
    class_repo: &ClassRepository,
    class_id: Option<&str>,
    metadata: &Sf2WorkbookMetadata,
    settings: &Settings,
) -> Result<Class> {
    if let Some(class_id) = class_id.map(str::trim).filter(|value| !value.is_empty()) {
        return class_repo
            .get(class_id)?
            .ok_or_else(|| AppError::InvalidInput("Selected class was not found".to_string()));
    }

    let class_name = class_name(&metadata.grade_level, &metadata.section);
    find_or_create_class(class_repo, &class_name, Some(settings))
}

fn roster_students_for_draft(
    student_repo: &StudentRepository,
    class_id: &str,
    learner_names: &[String],
) -> Result<(Vec<Student>, usize, usize)> {
    let existing_students = student_repo.list_by_class(Some(class_id))?;
    let mut existing_by_name: HashMap<String, Student> = existing_students
        .iter()
        .cloned()
        .map(|student| (normalize_learner_name(&student.name), student))
        .collect();

    let mut requested_names = Vec::new();
    let mut seen_names = HashSet::new();
    for name in learner_names.iter().map(|name| name.trim()) {
        if name.is_empty() || !is_learner_name(name) {
            continue;
        }

        let normalized = normalize_learner_name(name);
        if seen_names.insert(normalized) {
            requested_names.push(name.to_string());
        }
    }

    if requested_names.is_empty() {
        let reused = existing_students.len();
        return Ok((existing_students, 0, reused));
    }

    let mut students = Vec::with_capacity(requested_names.len());
    let mut students_created = 0;
    let mut students_reused = 0;

    for name in requested_names {
        let normalized = normalize_learner_name(&name);
        let student = if let Some(student) = existing_by_name.get(&normalized) {
            students_reused += 1;
            student.clone()
        } else {
            let created = student_repo.create(CreateStudentRequest {
                name: name.clone(),
                gender: None,
                card_serial: None,
                class_id: Some(class_id.to_string()),
            })?;
            existing_by_name.insert(normalized, created.clone());
            students_created += 1;
            created
        };
        students.push(student);
    }

    Ok((students, students_created, students_reused))
}

fn find_or_create_class(
    class_repo: &ClassRepository,
    class_name: &str,
    settings: Option<&Settings>,
) -> Result<Class> {
    if let Some(existing) = class_repo
        .list()?
        .into_iter()
        .find(|class| class.name.eq_ignore_ascii_case(class_name))
    {
        return Ok(existing);
    }

    let day_start = settings
        .map(|settings| settings.day_start.clone())
        .unwrap_or_else(|| "08:30".to_string());
    let day_end = settings
        .map(|settings| settings.day_end.clone())
        .unwrap_or_else(|| "15:30".to_string());
    let late_after = settings
        .map(|settings| settings.late_after.clone())
        .unwrap_or_else(|| "08:45".to_string());

    class_repo.create(CreateClassRequest {
        name: class_name.to_string(),
        room: Some("N/A".to_string()),
        day_start,
        day_end,
        late_after,
        sessions: Vec::new(),
        days: vec![1, 2, 3, 4, 5],
    })
}

#[derive(Debug, Clone, Copy)]
struct TemplateRosterSlot {
    row_index: u32,
    gender_block: &'static str,
}

#[derive(Debug, Clone)]
struct TemplateRosterAssignment {
    student: Student,
    slot: TemplateRosterSlot,
}

fn template_roster_slots() -> Vec<TemplateRosterSlot> {
    let mut slots = Vec::new();
    for row_index in SF2_MALE_ROWS {
        slots.push(TemplateRosterSlot {
            row_index,
            gender_block: "MALE",
        });
    }
    for row_index in SF2_FEMALE_ROWS {
        slots.push(TemplateRosterSlot {
            row_index,
            gender_block: "FEMALE",
        });
    }
    slots
}

fn template_roster_assignments(students: &[Student]) -> Result<Vec<TemplateRosterAssignment>> {
    let row_slots = template_roster_slots();
    let male_slots = row_slots
        .iter()
        .copied()
        .filter(|slot| slot.gender_block == StudentGender::Male.sf2_block())
        .collect::<Vec<_>>();
    let female_slots = row_slots
        .iter()
        .copied()
        .filter(|slot| slot.gender_block == StudentGender::Female.sf2_block())
        .collect::<Vec<_>>();
    let mut male_students = Vec::new();
    let mut female_students = Vec::new();
    let mut missing_gender = Vec::new();

    for student in students {
        match student.gender {
            Some(StudentGender::Male) => male_students.push(student),
            Some(StudentGender::Female) => female_students.push(student),
            None => missing_gender.push(student.name.trim().to_string()),
        }
    }

    if !missing_gender.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "Set Male/Female for these students before creating or updating the SF2 workbook: {}",
            missing_gender.join(", ")
        )));
    }
    if male_students.len() > male_slots.len() {
        return Err(AppError::InvalidInput(format!(
            "The bundled SF2 template has {} male learner rows, but this class has {} male learners",
            male_slots.len(),
            male_students.len()
        )));
    }
    if female_students.len() > female_slots.len() {
        return Err(AppError::InvalidInput(format!(
            "The bundled SF2 template has {} female learner rows, but this class has {} female learners",
            female_slots.len(),
            female_students.len()
        )));
    }

    let mut assignments = Vec::with_capacity(students.len());
    assignments.extend(
        male_students
            .into_iter()
            .zip(male_slots)
            .map(|(student, slot)| TemplateRosterAssignment {
                student: student.clone(),
                slot,
            }),
    );
    assignments.extend(
        female_students
            .into_iter()
            .zip(female_slots)
            .map(|(student, slot)| TemplateRosterAssignment {
                student: student.clone(),
                slot,
            }),
    );
    Ok(assignments)
}

fn roster_name_marks(
    analysis: &Sf2WorkbookAnalysis,
    assignments: &[TemplateRosterAssignment],
    row_slots: &[TemplateRosterSlot],
) -> Vec<Sf2CellMark> {
    let sheet_names = analysis
        .sheets
        .iter()
        .filter(|sheet| sheet.visible != 0)
        .map(|sheet| sheet.name.clone())
        .collect::<Vec<_>>();

    let names_by_row = assignments
        .iter()
        .map(|assignment| (assignment.slot.row_index, assignment.student.name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut marks = Vec::with_capacity(sheet_names.len() * row_slots.len());
    for sheet_name in sheet_names {
        for slot in row_slots {
            let value = names_by_row
                .get(&slot.row_index)
                .copied()
                .unwrap_or_default()
                .trim()
                .to_string();
            marks.push(Sf2CellMark {
                sheet_name: sheet_name.clone(),
                cell_address: format!("{SF2_NAME_COLUMN}{}", slot.row_index),
                value,
            });
        }
    }
    marks
}

fn clear_attendance_marks_for_records(
    date_mappings: &[Sf2DateMappingRecord],
    student_mappings: &[Sf2StudentMappingRecord],
) -> Vec<Sf2CellMark> {
    let row_slots = template_roster_slots();
    let row_indices = attendance_grid_rows(
        &row_slots,
        student_mappings.iter().map(|mapping| mapping.row_index),
    );

    let mut marks = Vec::with_capacity(date_mappings.len() * row_indices.len());
    for date_mapping in date_mappings {
        for row_index in &row_indices {
            marks.push(Sf2CellMark {
                sheet_name: date_mapping.sheet_name.clone(),
                cell_address: format!("{}{}", date_mapping.column_letter, row_index),
                value: String::new(),
            });
        }
    }
    marks
}

fn attendance_grid_rows<I>(row_slots: &[TemplateRosterSlot], extra_rows: I) -> Vec<u32>
where
    I: IntoIterator<Item = u32>,
{
    let mut rows = row_slots
        .iter()
        .map(|slot| slot.row_index)
        .collect::<Vec<_>>();
    rows.extend(extra_rows);
    rows.sort_unstable();
    rows.dedup();
    rows
}

fn unique_normalized_name(seen: &mut HashSet<String>, name: &str, suffix: &str) -> String {
    let normalized = normalize_learner_name(name);
    if seen.insert(normalized.clone()) {
        return normalized;
    }

    let unique = format!("{normalized}#{suffix}");
    seen.insert(unique.clone());
    unique
}

fn sf2_metadata_warnings(metadata: &Sf2WorkbookMetadata) -> Vec<String> {
    [
        ("School ID", &metadata.school_id),
        ("Name of School", &metadata.school_name),
        ("School Year", &metadata.school_year),
        ("Report for the Month of", &metadata.report_month),
        ("Grade Level", &metadata.grade_level),
        ("Section", &metadata.section),
        (
            "Signature of Adviser over Printed Name / Generated thru LIS adviser name",
            &metadata.adviser_name,
        ),
        (
            "Signature of School Head over Printed Name",
            &metadata.school_head_name,
        ),
    ]
    .into_iter()
    .filter_map(|(label, value)| {
        let missing = value.trim().is_empty();
        missing.then(|| format!("{label} is blank in this SF2 workbook."))
    })
    .collect()
}

fn copy_workbook_to_app_data<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    source_path: &Path,
    template_id: &str,
    analysis: &crate::sf2::models::Sf2WorkbookAnalysis,
) -> Result<PathBuf> {
    let dir = sf2_workbook_dir(app)?;
    let template_prefix = template_id.chars().take(8).collect::<String>();
    let file_name = format!(
        "SF2-{}-{}-{}.xls",
        sanitized_or(&analysis.grade_level, "GRADE"),
        sanitized_or(&analysis.section, "SECTION"),
        template_prefix
    );
    let working_copy_path = dir.join(file_name);

    if source_path != working_copy_path {
        std::fs::copy(source_path, &working_copy_path).map_err(|error| {
            AppError::Internal(format!(
                "failed to copy SF2 workbook into app data: {error}"
            ))
        })?;
    }

    Ok(working_copy_path)
}

fn write_bundled_template_to_dir(
    dir: &Path,
    template_id: &str,
    grade_level: &str,
    section: &str,
) -> Result<PathBuf> {
    let template_prefix = template_id.chars().take(8).collect::<String>();
    let file_name = format!(
        "SF2-{}-{}-{}.xls",
        sanitized_or(grade_level, "GRADE"),
        sanitized_or(section, "SECTION"),
        template_prefix
    );
    let working_copy_path = dir.join(file_name);
    std::fs::write(&working_copy_path, BUNDLED_TEMPLATE_BYTES).map_err(|error| {
        AppError::Internal(format!(
            "failed to create SF2 workbook from bundled template: {error}"
        ))
    })?;
    Ok(working_copy_path)
}

fn sf2_workbook_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Internal(format!("failed to get app data directory: {error}")))?
        .join("sf2-workbooks");
    std::fs::create_dir_all(&dir).map_err(|error| {
        AppError::Internal(format!("failed to create SF2 workbook directory: {error}"))
    })?;
    Ok(dir)
}

fn sanitized_or(value: &str, fallback: &str) -> String {
    let sanitized = sanitize_file_part(value);
    if sanitized.is_empty() {
        fallback.to_string()
    } else {
        sanitized
    }
}

fn present_events_for_day(
    events: &[AttendanceEvent],
    students: &[Student],
    class_id: &str,
    date: &str,
) -> Vec<Sf2AttendanceEvent> {
    let student_ids: HashSet<String> = students
        .iter()
        .map(|student| student.id.to_string())
        .collect();
    events
        .iter()
        .filter(|event| {
            event_belongs_to_class(event, &student_ids, class_id) && local_event_date(event) == date
        })
        .map(|event| Sf2AttendanceEvent {
            student_id: event.student_id.to_string(),
            event_type: "in".to_string(),
        })
        .collect()
}

fn present_student_ids(
    events: &[AttendanceEvent],
    students: &[Student],
    class_id: &str,
    date: &str,
) -> HashSet<String> {
    present_events_for_day(events, students, class_id, date)
        .into_iter()
        .map(|event| event.student_id)
        .collect()
}

fn event_belongs_to_class(
    event: &AttendanceEvent,
    class_student_ids: &HashSet<String>,
    class_id: &str,
) -> bool {
    event.class_id.as_deref() == Some(class_id)
        || class_student_ids.contains(&event.student_id.to_string())
}

fn local_event_date(event: &AttendanceEvent) -> String {
    event
        .timestamp
        .with_timezone(&Local)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
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

fn set_attendance_event_for_day(
    pool: DbPool,
    student_id: &str,
    class_id: &str,
    date: NaiveDate,
    day_start: &str,
    present: bool,
) -> Result<()> {
    let (day_start_timestamp, day_end_timestamp) = local_day_bounds_timestamps_for_date(date)?;
    let mut conn = pool.get()?;
    let transaction = conn.transaction()?;
    transaction.execute(
        "DELETE FROM events
         WHERE student_id = ?1
         AND event_type = 'in'
         AND timestamp >= ?2
         AND timestamp < ?3
         AND (class_id IS NULL OR class_id = ?4)",
        params![student_id, day_start_timestamp, day_end_timestamp, class_id],
    )?;

    if present {
        let attendance_timestamp = attendance_timestamp_for_date(date, day_start)?;
        let session_key = format!("{date}|{class_id}|day");
        transaction.execute(
            "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
             VALUES (?1, ?2, ?3, 'in', ?4, ?5, ?6, ?7, NULL)",
            params![
                uuid::Uuid::new_v4().to_string(),
                student_id,
                class_id,
                attendance_timestamp,
                "SF2 preview correction",
                session_key,
                "SF2 preview correction",
            ],
        )?;
    }

    transaction.commit()?;
    Ok(())
}

fn local_day_bounds_timestamps_for_date(date: NaiveDate) -> Result<(i64, i64)> {
    let next_day = date.succ_opt().ok_or_else(|| {
        AppError::Internal("failed to calculate local attendance date".to_string())
    })?;
    Ok((
        local_timestamp(date, 0, 0)?,
        local_timestamp(next_day, 0, 0)?,
    ))
}

fn attendance_timestamp_for_date(date: NaiveDate, day_start: &str) -> Result<i64> {
    let (hour, minute) = parse_clock(day_start).unwrap_or((8, 0));
    local_timestamp(date, hour, minute)
}

fn local_timestamp(date: NaiveDate, hour: u32, minute: u32) -> Result<i64> {
    let local_time = date
        .and_hms_opt(hour, minute, 0)
        .and_then(|time| time.and_local_timezone(Local).earliest())
        .ok_or_else(|| {
            AppError::Internal(format!(
                "failed to calculate local timestamp for {}",
                date.format("%Y-%m-%d")
            ))
        })?;
    Ok(local_time.with_timezone(&Utc).timestamp())
}

fn parse_clock(value: &str) -> Option<(u32, u32)> {
    let (hour, minute) = value.trim().split_once(':')?;
    let hour = hour.parse::<u32>().ok()?;
    let minute = minute.parse::<u32>().ok()?;
    if hour < 24 && minute < 60 {
        Some((hour, minute))
    } else {
        None
    }
}

fn pick_workbook_path(app: &tauri::AppHandle) -> Result<PathBuf> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Excel 97-2003 Workbook", &["xls"])
        .pick_file(move |result| {
            let _ = tx.send(result);
        });

    dialog_path(rx.recv().map_err(|error| {
        AppError::Internal(format!("failed to receive workbook path: {error}"))
    })?)?
    .ok_or_else(|| AppError::InvalidInput("Import cancelled".to_string()))
}

fn save_workbook_path(app: &tauri::AppHandle, template: &Sf2TemplateRecord) -> Result<PathBuf> {
    let file_name = export_workbook_file_name(template, Local::now().month());
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Excel 97-2003 Workbook", &["xls"])
        .set_file_name(file_name)
        .save_file(move |result| {
            let _ = tx.send(result);
        });

    dialog_path(
        rx.recv().map_err(|error| {
            AppError::Internal(format!("failed to receive output path: {error}"))
        })?,
    )?
    .ok_or_else(|| AppError::InvalidInput("Export cancelled".to_string()))
}

fn export_workbook_file_name(template: &Sf2TemplateRecord, current_month: u32) -> String {
    let month = sf2_export_month_file_part(&template.report_month, current_month);
    format!(
        "SF2-{}-{}-{}-generated.xls",
        sanitized_or(&template.grade_level, "GRADE"),
        sanitized_or(&template.section, "SECTION"),
        month
    )
}

fn sf2_export_month_file_part(report_month: &str, current_month: u32) -> &'static str {
    sf2_month_number(report_month)
        .or(Some(current_month))
        .map(sf2_month_name)
        .filter(|month| !month.is_empty())
        .unwrap_or("MONTH")
}

fn dialog_path(path: Option<tauri_plugin_dialog::FilePath>) -> Result<Option<PathBuf>> {
    match path {
        Some(tauri_plugin_dialog::FilePath::Path(path)) => Ok(Some(path)),
        Some(tauri_plugin_dialog::FilePath::Url(url)) => Err(AppError::InvalidInput(format!(
            "URL file paths are not supported: {url}"
        ))),
        None => Ok(None),
    }
}

#[cfg(target_os = "windows")]
fn open_path_in_default_app(path: &Path) -> Result<()> {
    let status = Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(path)
        .status()
        .map_err(|error| AppError::Internal(format!("failed to open SF2 workbook: {error}")))?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::Internal(format!(
            "failed to open SF2 workbook: default app returned {status}"
        )))
    }
}

#[cfg(target_os = "macos")]
fn open_path_in_default_app(path: &Path) -> Result<()> {
    Command::new("open")
        .arg(path)
        .spawn()
        .map_err(|error| AppError::Internal(format!("failed to open SF2 workbook: {error}")))?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_path_in_default_app(path: &Path) -> Result<()> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|error| AppError::Internal(format!("failed to open SF2 workbook: {error}")))?;
    Ok(())
}

fn class_name(grade_level: &str, section: &str) -> String {
    let grade = grade_level.trim();
    let section = section.trim();
    match (grade.is_empty(), section.is_empty()) {
        (false, false) => format!("{grade} - {section}"),
        (false, true) => grade.to_string(),
        (true, false) => section.to_string(),
        (true, true) => "SF2 Class".to_string(),
    }
}

fn sanitize_file_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn today_string() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

fn parse_date(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| AppError::InvalidInput(format!("invalid date: {date}")))
}

fn first_school_day_from_mappings(date_mappings: &[Sf2DateMappingRecord]) -> u32 {
    date_mappings
        .iter()
        .filter_map(|mapping| parse_date(&mapping.date).ok())
        .map(|date| date.day())
        .min()
        .unwrap_or(1)
}

fn write_temp_binary_file(prefix: &str, extension: &str, contents: &[u8]) -> Result<PathBuf> {
    let path = std::env::temp_dir().join(format!("{prefix}-{}{}", uuid::Uuid::new_v4(), extension));
    let mut file = std::fs::File::create(&path)
        .map_err(|error| AppError::Internal(format!("failed to create temp file: {error}")))?;
    file.write_all(contents)
        .map_err(|error| AppError::Internal(format!("failed to write temp file: {error}")))?;
    Ok(path)
}

fn file_hash(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)
        .map_err(|error| AppError::Internal(format!("failed to read SF2 workbook: {error}")))?;
    Ok(hash_bytes(&bytes))
}

fn layout_fingerprint(analysis: &crate::sf2::models::Sf2WorkbookAnalysis) -> String {
    let mut bytes = Vec::new();
    for sheet in &analysis.sheets {
        bytes.extend_from_slice(sheet.name.as_bytes());
        bytes.extend_from_slice(sheet.used_range.as_bytes());
    }
    for learner in &analysis.learners {
        bytes.extend_from_slice(learner.name.as_bytes());
        bytes.extend_from_slice(&learner.row_index.to_le_bytes());
    }
    for date in &analysis.dates {
        bytes.extend_from_slice(date.date.as_bytes());
        bytes.extend_from_slice(date.sheet_name.as_bytes());
        bytes.extend_from_slice(date.column_letter.as_bytes());
    }
    hash_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::StudentId;
    use crate::sf2::models::{Sf2WorkbookDate, Sf2WorkbookLearner, Sf2WorkbookSheet};

    fn template_draft(first_school_day: Option<u32>) -> Sf2TemplateDraft {
        Sf2TemplateDraft {
            class_id: Some("class-1".to_string()),
            school_id: "123456".to_string(),
            school_name: "Sample School".to_string(),
            school_year: "2026-2027".to_string(),
            report_month: "JUNE".to_string(),
            grade_level: "11".to_string(),
            section: "A".to_string(),
            adviser_name: "Adviser".to_string(),
            school_head_name: "Head".to_string(),
            first_school_day,
            learner_names: Vec::new(),
        }
    }

    fn workbook_analysis_with_dates(dates: &[&str]) -> Sf2WorkbookAnalysis {
        Sf2WorkbookAnalysis {
            file_format: 56,
            has_vb_project: false,
            school_id: String::new(),
            school_name: String::new(),
            school_year: "2026-2027".to_string(),
            report_month: "JUNE".to_string(),
            grade_level: "11".to_string(),
            section: "A".to_string(),
            adviser_name: String::new(),
            school_head_name: String::new(),
            learners: Vec::<Sf2WorkbookLearner>::new(),
            dates: dates
                .iter()
                .enumerate()
                .map(|(index, date)| Sf2WorkbookDate {
                    sheet_name: "JUNE 2026".to_string(),
                    date: (*date).to_string(),
                    column_letter: "F".to_string(),
                    column_index: 6 + index as u32,
                })
                .collect(),
            sheets: vec![Sf2WorkbookSheet {
                name: "JUNE 2026".to_string(),
                visible: -1,
                used_range: "A1:AP82".to_string(),
            }],
        }
    }

    fn student_with_gender(name: &str, gender: Option<StudentGender>) -> Student {
        Student {
            id: StudentId::new(),
            name: name.to_string(),
            gender,
            card_serial: None,
            class_id: Some("class-1".to_string()),
            created_at: chrono::Utc::now(),
        }
    }

    fn template_record(report_month: &str) -> Sf2TemplateRecord {
        Sf2TemplateRecord {
            id: "template-1".to_string(),
            source_path: "C:/sf2-working.xls".to_string(),
            source_hash: "hash".to_string(),
            school_id: "123456".to_string(),
            school_name: "Sample School".to_string(),
            school_year: "2026-2027".to_string(),
            report_month: report_month.to_string(),
            grade_level: "Grade 4".to_string(),
            section: "Rizal".to_string(),
            adviser_name: "Adviser".to_string(),
            school_head_name: "Head".to_string(),
            layout_fingerprint: "layout".to_string(),
            active_class_id: "class-1".to_string(),
            imported_at: 0,
        }
    }

    #[test]
    fn export_workbook_file_name_includes_saved_report_month() {
        let file_name =
            export_workbook_file_name(&template_record("Report for the Month of: June"), 7);

        assert_eq!(file_name, "SF2-GRADE-4-RIZAL-JUNE-generated.xls");
    }

    #[test]
    fn export_workbook_file_name_falls_back_to_current_month_when_report_month_is_blank() {
        let file_name = export_workbook_file_name(&template_record(""), 8);

        assert_eq!(file_name, "SF2-GRADE-4-RIZAL-AUGUST-generated.xls");
    }

    #[test]
    fn sf2_report_year_uses_current_calendar_year() {
        assert_eq!(sf2_report_year("2025-2026", 6), chrono::Local::now().year());
    }

    #[test]
    fn metadata_from_draft_requires_first_attendance_day() {
        let error = metadata_from_draft(&template_draft(None)).unwrap_err();

        assert!(error
            .to_string()
            .contains("First attendance day is required"));
    }

    #[test]
    fn metadata_from_draft_rejects_weekend_first_attendance_day() {
        let error = metadata_from_draft(&template_draft(Some(7))).unwrap_err();

        assert!(error
            .to_string()
            .contains("must be a Monday-Friday school day"));
    }

    #[test]
    fn metadata_from_draft_accepts_selected_monday_first_attendance_day() {
        let metadata = metadata_from_draft(&template_draft(Some(8))).unwrap();

        assert_eq!(metadata.first_school_day, Some(8));
    }

    #[test]
    fn configured_calendar_validation_rejects_detected_day_one() {
        let metadata = metadata_from_draft(&template_draft(Some(8))).unwrap();
        let analysis = workbook_analysis_with_dates(&["2026-06-01", "2026-06-02"]);

        let error = validate_configured_calendar(&analysis, &metadata).unwrap_err();

        assert!(error
            .to_string()
            .contains("expected first attendance day 8"));
    }

    #[test]
    fn configured_calendar_validation_accepts_detected_day_eight() {
        let metadata = metadata_from_draft(&template_draft(Some(8))).unwrap();
        let analysis = workbook_analysis_with_dates(&["2026-06-08", "2026-06-09"]);

        validate_configured_calendar(&analysis, &metadata).unwrap();
    }

    #[test]
    fn export_preview_reports_absences_and_unmapped_students() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database should be created");
        let workbook = tempfile::NamedTempFile::new().expect("workbook file should be created");
        let pool = crate::infrastructure::init_db(temp_db.path()).expect("database should init");
        let current_year = Local::now().year();
        let stale_year = current_year - 1;
        let current_june_day = format!("{current_year}-06-08");
        let stale_june_day = format!("{stale_year}-06-08");
        let stale_july_day = format!("{stale_year}-07-01");
        let class = ClassRepository::new(pool.clone())
            .create(CreateClassRequest {
                name: "7 - Rose".to_string(),
                room: Some("N/A".to_string()),
                day_start: "08:30".to_string(),
                day_end: "15:30".to_string(),
                late_after: "08:45".to_string(),
                sessions: Vec::new(),
                days: vec![1, 2, 3, 4, 5],
            })
            .expect("class should be created");
        let student_repo = StudentRepository::new(pool.clone());
        let mapped_student = student_repo
            .create(CreateStudentRequest {
                name: "Abao, Ben".to_string(),
                gender: Some(StudentGender::Male),
                card_serial: None,
                class_id: Some(class.id.clone()),
            })
            .expect("mapped student should be created");
        student_repo
            .create(CreateStudentRequest {
                name: "Zamora, Ana".to_string(),
                gender: Some(StudentGender::Female),
                card_serial: None,
                class_id: Some(class.id.clone()),
            })
            .expect("unmapped student should be created");

        let template = Sf2TemplateRecord {
            id: "template-preview".to_string(),
            source_path: workbook.path().to_string_lossy().to_string(),
            source_hash: "preview-hash".to_string(),
            school_id: "123456".to_string(),
            school_name: "Sample Integrated School".to_string(),
            school_year: format!("{stale_year}-{current_year}"),
            report_month: "JUNE".to_string(),
            grade_level: "7".to_string(),
            section: "Rose".to_string(),
            adviser_name: "Teacher Adviser".to_string(),
            school_head_name: "School Head".to_string(),
            layout_fingerprint: "preview-layout".to_string(),
            active_class_id: class.id.clone(),
            imported_at: 0,
        };
        let student_mapping = Sf2StudentMappingRecord {
            template_id: template.id.clone(),
            student_id: mapped_student.id.to_string(),
            workbook_name: mapped_student.name.clone(),
            normalized_name: normalize_learner_name(&mapped_student.name),
            row_index: 8,
            gender_block: Some("MALE".to_string()),
        };
        let date_mappings = vec![
            Sf2DateMappingRecord {
                template_id: template.id.clone(),
                sheet_name: format!("JUNE {stale_year}"),
                date: stale_june_day,
                column_letter: "F".to_string(),
                column_index: 6,
            },
            Sf2DateMappingRecord {
                template_id: template.id.clone(),
                sheet_name: format!("JULY {stale_year}"),
                date: stale_july_day,
                column_letter: "F".to_string(),
                column_index: 6,
            },
        ];
        let sf2_repo = Sf2Repository::new(pool.clone());
        sf2_repo
            .upsert_template_with_mappings(&template, &[student_mapping], &date_mappings)
            .expect("template mappings should save");
        sf2_repo
            .close_day(&class.id, &current_june_day, 0)
            .expect("closed day should save");
        sf2_repo
            .close_day(&class.id, &format!("{current_year}-07-01"), 0)
            .expect("future month closed day should save");

        let preview = export_preview(pool, Some(class.id)).expect("preview should be generated");

        assert!(preview.can_export);
        assert_eq!(preview.mapped_dates, 1);
        assert_eq!(preview.closed_days, vec![current_june_day.clone()]);
        assert_eq!(
            preview
                .dates
                .iter()
                .map(|date| date.date.as_str())
                .collect::<Vec<_>>(),
            vec![current_june_day.as_str()]
        );
        assert_eq!(preview.absence_count, 1);
        assert_eq!(preview.absent_list.len(), 1);
        assert_eq!(preview.unmapped_student_count, 1);
        assert_eq!(
            preview.students[0].cells[0].status,
            Sf2PreviewCellStatus::Absent
        );
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("not mapped to an SF2 learner row")));
    }

    #[test]
    fn clear_attendance_marks_covers_template_grid_without_students() {
        let dates = vec![Sf2DateMappingRecord {
            template_id: "template-1".to_string(),
            sheet_name: "JUNE 2026".to_string(),
            date: "2026-06-08".to_string(),
            column_letter: "F".to_string(),
            column_index: 6,
        }];

        let marks = clear_attendance_marks_for_records(&dates, &[]);

        assert_eq!(marks.len(), template_roster_slots().len());
        assert!(marks.contains(&Sf2CellMark {
            sheet_name: "JUNE 2026".to_string(),
            cell_address: "F8".to_string(),
            value: String::new(),
        }));
        assert!(marks.contains(&Sf2CellMark {
            sheet_name: "JUNE 2026".to_string(),
            cell_address: "F48".to_string(),
            value: String::new(),
        }));
    }

    #[test]
    fn clear_attendance_marks_includes_imported_rows_outside_template_grid() {
        let dates = vec![Sf2DateMappingRecord {
            template_id: "template-1".to_string(),
            sheet_name: "JUNE 2026".to_string(),
            date: "2026-06-08".to_string(),
            column_letter: "F".to_string(),
            column_index: 6,
        }];
        let students = vec![Sf2StudentMappingRecord {
            template_id: "template-1".to_string(),
            student_id: "student-1".to_string(),
            workbook_name: "Learner, Sample".to_string(),
            normalized_name: "LEARNER,SAMPLE".to_string(),
            row_index: 55,
            gender_block: Some("FEMALE".to_string()),
        }];

        let marks = clear_attendance_marks_for_records(&dates, &students);

        assert_eq!(marks.len(), template_roster_slots().len() + 1);
        assert!(marks.contains(&Sf2CellMark {
            sheet_name: "JUNE 2026".to_string(),
            cell_address: "F55".to_string(),
            value: String::new(),
        }));
    }

    #[test]
    fn template_roster_assignments_keep_female_learners_in_female_rows() {
        let students = vec![
            student_with_gender("Zamora, Ana", Some(StudentGender::Female)),
            student_with_gender("Abao, Ben", Some(StudentGender::Male)),
        ];

        let assignments = template_roster_assignments(&students).unwrap();

        let male = assignments
            .iter()
            .find(|assignment| assignment.student.name == "Abao, Ben")
            .unwrap();
        let female = assignments
            .iter()
            .find(|assignment| assignment.student.name == "Zamora, Ana")
            .unwrap();
        assert!(SF2_MALE_ROWS.contains(&male.slot.row_index));
        assert_eq!(male.slot.gender_block, "MALE");
        assert!(SF2_FEMALE_ROWS.contains(&female.slot.row_index));
        assert_eq!(female.slot.gender_block, "FEMALE");
    }

    #[test]
    fn template_roster_assignments_reject_missing_gender() {
        let students = vec![student_with_gender("Learner, Missing", None)];

        let error = template_roster_assignments(&students).unwrap_err();

        assert!(error.to_string().contains("Set Male/Female"));
    }

    #[test]
    #[ignore = "requires Microsoft Excel COM automation"]
    fn create_workbook_from_template_flow_creates_class_and_template() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database should be created");
        let workbook_dir = tempfile::tempdir().expect("workbook directory should be created");
        let pool = crate::infrastructure::init_db(temp_db.path()).expect("database should init");
        let class = ClassRepository::new(pool.clone())
            .create(CreateClassRequest {
                name: "7 - Rose".to_string(),
                room: None,
                day_start: "08:30".to_string(),
                day_end: "15:30".to_string(),
                late_after: "08:45".to_string(),
                sessions: Vec::new(),
                days: vec![1, 2, 3, 4, 5],
            })
            .expect("class should be created");
        StudentRepository::new(pool.clone())
            .create(CreateStudentRequest {
                name: "Learner, One".to_string(),
                gender: Some(StudentGender::Male),
                card_serial: None,
                class_id: Some(class.id.clone()),
            })
            .expect("student should be created");

        let summary = create_workbook_from_template_in_dir(
            workbook_dir.path(),
            pool.clone(),
            Sf2TemplateDraft {
                class_id: Some(class.id.clone()),
                school_id: "123456".to_string(),
                school_name: "Sample Integrated School".to_string(),
                school_year: "2026-2027".to_string(),
                report_month: "JUNE".to_string(),
                grade_level: "7".to_string(),
                section: "Rose".to_string(),
                adviser_name: "Teacher Adviser".to_string(),
                school_head_name: "School Head".to_string(),
                first_school_day: Some(8),
                learner_names: Vec::new(),
            },
        )
        .expect("SF2 workbook should be created from template");

        assert_eq!(summary.class_name, "7 - Rose");
        assert_eq!(summary.learners_found, 1);
        assert_eq!(summary.students_created, 0);
        assert_eq!(summary.dates_mapped, 17);
        assert!(Path::new(&summary.source_path).exists());

        let settings = workbook_settings(pool.clone(), Some(summary.class_id.clone()))
            .expect("created SF2 settings should be readable");
        assert_eq!(settings.class_name, "7 - Rose");
        assert_eq!(settings.section, "Rose");
        assert_eq!(settings.learner_names, vec!["Learner, One".to_string()]);

        let readiness = export_readiness(pool.clone(), Some(summary.class_id.clone()))
            .expect("created SF2 workbook should have export readiness");
        assert!(readiness.can_export);
        assert_eq!(readiness.mapped_students, 1);
        assert_eq!(readiness.mapped_dates, 17);

        let close_summary = close_day(pool, summary.class_id, Some("2026-06-08".to_string()))
            .expect("created SF2 workbook should accept close-day marking");
        assert_eq!(close_summary.absent_count, 1);

        let _ = std::fs::remove_file(summary.source_path);
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Fnva64::default();
    hasher.write(bytes);
    format!("{:016x}", hasher.finish())
}

#[derive(Default)]
struct Fnva64(u64);

impl Hasher for Fnva64 {
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = if self.0 == 0 {
            0xcbf29ce484222325
        } else {
            self.0
        };
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.0 = hash;
    }

    fn finish(&self) -> u64 {
        if self.0 == 0 {
            0xcbf29ce484222325
        } else {
            self.0
        }
    }
}
