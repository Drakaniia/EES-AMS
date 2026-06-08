use crate::domain::error::{AppError, Result};
use crate::domain::models::{
    Class, CreateClassRequest, CreateStudentRequest, Settings, Student, StudentGender,
    UpdateStudentRequest,
};
use crate::infrastructure::database::{
    record_audit_event, AuditEventInput, ClassRepository, DbPool, EventRepository,
    SettingsRepository, StudentRepository,
};
use crate::sf2::attendance::{present_events_for_day, present_student_ids};
use crate::sf2::calendar::{
    date_mappings_are_current_for_report_month, date_mappings_from_analysis,
    first_school_day_for_report_month, first_school_day_from_mappings, metadata_from_draft,
    metadata_from_import_analysis, parse_date, sf2_closed_days_for_report_month,
    sf2_date_mappings_for_report_month, sf2_metadata_warnings, sf2_month_number, template_metadata,
    validate_configured_calendar,
};
#[cfg(test)]
use crate::sf2::calendar::{
    default_sf2_first_school_day, sf2_report_year, validate_first_school_day,
};
use crate::sf2::excel;
use crate::sf2::logic::{
    attendance_marks_for_closed_day, is_learner_name, normalize_learner_name, Sf2CellMark,
    Sf2StudentMapping,
};
#[cfg(test)]
use crate::sf2::models::Sf2PreviewCellStatus;
use crate::sf2::models::{
    Sf2CloseDaySummary, Sf2DateMappingRecord, Sf2ExportPreview, Sf2ExportReadiness,
    Sf2ExportResult, Sf2ImportSummary, Sf2ImportValidation, Sf2StudentMappingRecord,
    Sf2TemplateDraft, Sf2TemplateRecord, Sf2WorkbookAnalysis, Sf2WorkbookLearner,
    Sf2WorkbookMetadata, Sf2WorkbookSettings,
};
use crate::sf2::naming::class_name;
use crate::sf2::preview;
use crate::sf2::repository::{template_summary, Sf2Repository};
#[cfg(test)]
use crate::sf2::validation::validate_student_list;
use crate::sf2::validation::{ensure_import_validation_allows, import_validation_from_analysis};
#[cfg(test)]
use crate::sf2::workbook_files::export_workbook_file_name;
use crate::sf2::workbook_files::{
    copy_workbook_to_app_data, file_hash, hash_bytes, layout_fingerprint, open_path_in_default_app,
    pick_workbook_path, save_workbook_path, sf2_workbook_dir, write_bundled_template_to_dir,
    write_temp_binary_file, BUNDLED_TEMPLATE_BYTES,
};
use chrono::{Local, NaiveDate, Utc};
use rusqlite::params;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tauri::Emitter;

const SF2_NAME_COLUMN: &str = "C";
const SF2_MALE_ROWS: std::ops::RangeInclusive<u32> = 8..=28;
const SF2_FEMALE_ROWS: std::ops::RangeInclusive<u32> = 30..=48;

fn emit_sf2_progress<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    task: &str,
    current: u32,
    total: u32,
    message: &str,
) {
    let _ = app.emit(
        "sf2-progress",
        serde_json::json!({
            "task": task,
            "current": current,
            "total": total,
            "message": message,
        }),
    );
}

pub fn validate_workbook_import(
    app: tauri::AppHandle,
    pool: DbPool,
) -> Result<Sf2ImportValidation> {
    emit_sf2_progress(&app, "import", 1, 7, "Choosing SF2 workbook");
    let workbook_path = pick_workbook_path(&app)?;
    emit_sf2_progress(&app, "import", 2, 7, "Reading SF2 workbook");
    let source_analysis = excel::analyze_workbook(&workbook_path)?;
    emit_sf2_progress(&app, "import", 3, 7, "Validating learner list");

    import_validation_from_analysis(pool, &workbook_path, &source_analysis)
}

pub fn import_workbook(
    app: tauri::AppHandle,
    pool: DbPool,
    source_path: String,
    proceed_anyway: bool,
) -> Result<Sf2ImportSummary> {
    emit_sf2_progress(&app, "import", 4, 7, "Preparing workbook import");
    let workbook_path = PathBuf::from(source_path);
    let source_analysis = excel::analyze_workbook(&workbook_path)?;
    let validation =
        import_validation_from_analysis(pool.clone(), &workbook_path, &source_analysis)?;
    ensure_import_validation_allows(&validation, proceed_anyway)?;

    import_workbook_with_analysis(app, pool, &workbook_path, source_analysis)
}

fn import_workbook_with_analysis(
    app: tauri::AppHandle,
    pool: DbPool,
    workbook_path: &Path,
    source_analysis: Sf2WorkbookAnalysis,
) -> Result<Sf2ImportSummary> {
    emit_sf2_progress(&app, "import", 5, 7, "Copying SF2 working workbook");
    let source_hash = file_hash(workbook_path)?;
    let class_name = class_name(&source_analysis.grade_level, &source_analysis.section);

    let class_repo = ClassRepository::new(pool.clone());
    let student_repo = StudentRepository::new(pool.clone());
    let sf2_repo = Sf2Repository::new(pool.clone());

    let class = find_or_create_class(&class_repo, &class_name, None)?;
    let template_id = sf2_repo
        .find_template(
            &source_hash,
            &source_analysis.grade_level,
            &source_analysis.section,
        )?
        .map(|template| template.id)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let working_copy_path =
        copy_workbook_to_app_data(&app, workbook_path, &template_id, &source_analysis)?;
    let metadata = metadata_from_import_analysis(&source_analysis)?;
    emit_sf2_progress(&app, "import", 6, 7, "Writing SF2 details and date layout");
    excel::write_metadata(&working_copy_path, &metadata)?;
    let analysis = excel::analyze_workbook(&working_copy_path)?;
    validate_configured_calendar(&analysis, &metadata)?;
    let layout_fingerprint = layout_fingerprint(&analysis);

    let learner_sync =
        sync_workbook_learner_mappings(&student_repo, &class.id, &template_id, &analysis.learners)?;
    let student_mappings = learner_sync.student_mappings;
    let students_created = learner_sync.students_created;
    let students_reused = learner_sync.students_reused;

    let date_mappings = date_mappings_from_analysis(&template_id, &analysis);

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

    let closed_days = sf2_repo.closed_days_for_class(&class.id)?;
    write_template_marks_for_mappings(
        pool,
        &template,
        &closed_days,
        &student_mappings,
        &date_mappings,
    )?;
    sf2_repo.upsert_template_with_mappings(&template, &student_mappings, &date_mappings)?;
    emit_sf2_progress(&app, "import", 7, 7, "SF2 import complete");

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
    emit_sf2_progress(&app, "create", 1, 2, "Creating SF2 working workbook");
    let workbook_dir = sf2_workbook_dir(&app)?;
    let summary = create_workbook_from_template_in_dir(&workbook_dir, pool, draft)?;
    emit_sf2_progress(&app, "create", 2, 2, "SF2 workbook ready");
    Ok(summary)
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
    let sf2_repo = Sf2Repository::new(pool.clone());
    if sf2_repo.latest_template_for_class(&class.id)?.is_some() {
        return Err(AppError::InvalidInput(format!(
            "An SF2 workbook already exists for {}. Update the existing workbook settings instead of creating a new one",
            class.name
        )));
    }

    let student_repo = StudentRepository::new(pool.clone());
    let (students, students_created, students_reused) =
        roster_students_for_draft(&student_repo, &class.id, &draft.learner_names)?;
    reject_duplicate_roster_names(&students)?;

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

    let date_mappings = date_mappings_from_analysis(&template_id, &analysis);

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

fn reject_duplicate_roster_names(students: &[Student]) -> Result<()> {
    let mut names_by_normalized: HashMap<String, Vec<String>> = HashMap::new();
    for student in students {
        names_by_normalized
            .entry(normalize_learner_name(&student.name))
            .or_default()
            .push(student.name.clone());
    }

    let duplicates = names_by_normalized
        .into_values()
        .filter(|names| names.len() > 1)
        .map(|names| names.join(", "))
        .collect::<Vec<_>>();

    if duplicates.is_empty() {
        return Ok(());
    }

    Err(AppError::InvalidInput(format!(
        "Duplicate learner names must be corrected before creating an SF2 workbook: {}",
        duplicates.join("; ")
    )))
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
    excel::write_metadata(&workbook_path, &metadata)?;
    let analysis = excel::analyze_workbook(&workbook_path)?;
    validate_configured_calendar(&analysis, &metadata)?;
    let layout_fingerprint = layout_fingerprint(&analysis);
    let student_repo = StudentRepository::new(pool.clone());
    let (student_mappings, students_created, students_reused, learners_found) =
        if template_owns_roster(&existing) {
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
            (
                student_mappings,
                students_created,
                students_reused,
                students.len(),
            )
        } else {
            let learner_sync = sync_workbook_learner_mappings(
                &student_repo,
                &class.id,
                &existing.id,
                &analysis.learners,
            )?;
            let learners_found = learner_sync.student_mappings.len();
            (
                learner_sync.student_mappings,
                learner_sync.students_created,
                learner_sync.students_reused,
                learners_found,
            )
        };

    let date_mappings = date_mappings_from_analysis(&existing.id, &analysis);

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
        learners_found,
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
        let template = refresh_template_calendar_from_saved_month(pool.clone(), &template)?;
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

pub fn sync_workbook_roster_for_class(pool: DbPool, class_id: &str) -> Result<()> {
    let _ = sync_latest_workbook_roster_for_class(pool, class_id)?;
    Ok(())
}

fn sync_latest_workbook_roster_for_class(
    pool: DbPool,
    class_id: &str,
) -> Result<Option<Sf2TemplateRecord>> {
    let sf2_repo = Sf2Repository::new(pool.clone());
    let Some(template) = sf2_repo.latest_template_for_class(class_id)? else {
        return Ok(None);
    };

    Ok(Some(sync_template_roster_from_class(pool, &template)?))
}

fn sync_template_roster_from_class(
    pool: DbPool,
    template: &Sf2TemplateRecord,
) -> Result<Sf2TemplateRecord> {
    let workbook_path = PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let class = ClassRepository::new(pool.clone())
        .get(&template.active_class_id)?
        .ok_or_else(|| AppError::InvalidInput("Selected class was not found".to_string()))?;
    let students = StudentRepository::new(pool.clone()).list_by_class(Some(&class.id))?;
    reject_duplicate_roster_names(&students)?;

    let row_slots = template_roster_slots();
    let roster_assignments = template_roster_assignments(&students)?;
    let analysis = excel::analyze_workbook(&workbook_path)?;
    let roster_marks = roster_name_marks(&analysis, &roster_assignments, &row_slots);
    excel::write_marks(&workbook_path, &roster_marks)?;

    let refreshed_analysis = excel::analyze_workbook(&workbook_path)?;
    let student_mappings =
        student_mappings_from_roster_assignments(&template.id, &roster_assignments);
    let date_mappings = date_mappings_from_analysis(&template.id, &refreshed_analysis);
    let synced_template = Sf2TemplateRecord {
        layout_fingerprint: layout_fingerprint(&refreshed_analysis),
        ..template.clone()
    };

    let sf2_repo = Sf2Repository::new(pool.clone());
    sf2_repo.update_template_with_mappings(&synced_template, &student_mappings, &date_mappings)?;

    let closed_days = sf2_closed_days_for_report_month(
        &synced_template,
        &sf2_repo.closed_days_for_class(&class.id)?,
    );
    if let Err(error) = write_template_marks_for_mappings(
        pool,
        &synced_template,
        &closed_days,
        &student_mappings,
        &sf2_date_mappings_for_report_month(&synced_template, &date_mappings),
    ) {
        log::warn!("failed to backfill synced SF2 workbook marks: {error}");
    }

    Ok(synced_template)
}

pub fn export_readiness(pool: DbPool, class_id: Option<String>) -> Result<Sf2ExportReadiness> {
    let sf2_repo = Sf2Repository::new(pool.clone());
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
    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let mapped_students = student_mappings.len();
    let unmapped_students = unmapped_roster_student_names(pool, &template, &student_mappings)?;
    if !unmapped_students.is_empty() {
        issues.push(unmapped_roster_issue(&unmapped_students));
    }

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
    preview::export_preview(pool, readiness)
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
    let template = refresh_template_calendar_from_saved_month(pool.clone(), &template)?;
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
    let template = refresh_template_calendar_from_saved_month(pool.clone(), &template)?;
    let template = sync_template_roster_from_class(pool.clone(), &template)?;

    let working_copy_path = PathBuf::from(&template.source_path);
    if !working_copy_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let closed_days =
        sf2_closed_days_for_report_month(&template, &sf2_repo.closed_days_for_class(&class_id)?);
    let mapped_dates = sf2_date_mappings_for_report_month(
        &template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    if mapped_dates.is_empty() {
        return Err(AppError::InvalidInput(
            "No attendance dates are mapped to this SF2 report month.".to_string(),
        ));
    }
    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let unmapped_students =
        unmapped_roster_student_names(pool.clone(), &template, &student_mappings)?;
    if !unmapped_students.is_empty() {
        return Err(AppError::InvalidInput(unmapped_roster_issue(
            &unmapped_students,
        )));
    }

    let output_path = save_workbook_path(&app, &template)?;
    if working_copy_path == output_path {
        return Err(AppError::InvalidInput(
            "Choose a different output path so the app SF2 working copy is not overwritten"
                .to_string(),
        ));
    }

    let marks_written = write_template_marks_for_days(pool.clone(), &template, &closed_days)?;

    let metadata = template_metadata(&template);
    excel::write_metadata(&working_copy_path, &metadata)?;

    std::fs::copy(&working_copy_path, &output_path)
        .map_err(|error| AppError::Internal(format!("failed to export SF2 workbook: {error}")))?;
    open_path_in_default_app(&output_path)?;

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

    write_template_marks_for_mappings(pool, template, days, &student_mappings, &date_mappings)
}

fn write_template_marks_for_mappings(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    days: &[String],
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> Result<usize> {
    let workbook_path = PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
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

    let mut marks = clear_attendance_marks_for_records(template, date_mappings, student_mappings);
    let attendance_marks = if export_days.is_empty() || student_mappings.is_empty() {
        Vec::new()
    } else {
        export_marks(
            pool,
            &template.active_class_id,
            &export_days,
            student_mappings,
            date_mappings,
        )?
    };
    let attendance_mark_count = attendance_marks.len();

    marks.extend(attendance_marks);
    excel::write_marks(&workbook_path, &marks)?;
    Ok(attendance_mark_count)
}

fn refresh_template_calendar_from_saved_month(
    pool: DbPool,
    template: &Sf2TemplateRecord,
) -> Result<Sf2TemplateRecord> {
    let Some(_) = sf2_month_number(&template.report_month) else {
        return Ok(template.clone());
    };

    let sf2_repo = Sf2Repository::new(pool);
    let existing_date_mappings = sf2_repo.date_mappings_for_template(&template.id)?;
    if date_mappings_are_current_for_report_month(template, &existing_date_mappings) {
        return Ok(template.clone());
    }

    let workbook_path = PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let mut metadata = template_metadata(template);
    metadata.configure_calendar = true;
    metadata.first_school_day = Some(first_school_day_for_report_month(
        &metadata.report_month,
        &metadata.school_year,
        existing_date_mappings
            .iter()
            .map(|mapping| mapping.date.as_str()),
    )?);

    excel::write_metadata(&workbook_path, &metadata)?;
    let analysis = excel::analyze_workbook(&workbook_path)?;
    validate_configured_calendar(&analysis, &metadata)?;

    let refreshed_template = Sf2TemplateRecord {
        id: template.id.clone(),
        source_path: template.source_path.clone(),
        source_hash: template.source_hash.clone(),
        school_id: metadata.school_id,
        school_name: metadata.school_name,
        school_year: metadata.school_year,
        report_month: metadata.report_month,
        grade_level: metadata.grade_level,
        section: metadata.section,
        adviser_name: metadata.adviser_name,
        school_head_name: metadata.school_head_name,
        layout_fingerprint: layout_fingerprint(&analysis),
        active_class_id: template.active_class_id.clone(),
        imported_at: template.imported_at,
    };
    let refreshed_date_mappings = date_mappings_from_analysis(&template.id, &analysis);
    sf2_repo.update_template_with_mappings(
        &refreshed_template,
        &student_mappings,
        &refreshed_date_mappings,
    )?;

    Ok(refreshed_template)
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

#[derive(Debug)]
struct WorkbookLearnerSync {
    student_mappings: Vec<Sf2StudentMappingRecord>,
    students_created: usize,
    students_reused: usize,
}

fn sync_workbook_learner_mappings(
    student_repo: &StudentRepository,
    class_id: &str,
    template_id: &str,
    learners: &[Sf2WorkbookLearner],
) -> Result<WorkbookLearnerSync> {
    let existing_students = student_repo.list_by_class(Some(class_id))?;
    let mut existing_by_name: HashMap<String, Student> = existing_students
        .into_iter()
        .map(|student| (normalize_learner_name(&student.name), student))
        .collect();
    let mut seen_names = HashSet::new();
    let mut student_mappings = Vec::new();
    let mut students_created = 0;
    let mut students_reused = 0;

    for learner in learners
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
                class_id: Some(class_id.to_string()),
            })?;
            existing_by_name.insert(normalized_name.clone(), created.clone());
            students_created += 1;
            created
        };

        student_mappings.push(Sf2StudentMappingRecord {
            template_id: template_id.to_string(),
            student_id: student.id.to_string(),
            workbook_name: learner.name.clone(),
            normalized_name,
            row_index: learner.row_index,
            gender_block: learner.gender_block.clone(),
        });
    }

    Ok(WorkbookLearnerSync {
        student_mappings,
        students_created,
        students_reused,
    })
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

fn student_mappings_from_roster_assignments(
    template_id: &str,
    assignments: &[TemplateRosterAssignment],
) -> Vec<Sf2StudentMappingRecord> {
    let mut seen_normalized_names = HashSet::new();
    assignments
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
                template_id: template_id.to_string(),
                student_id: student.id.to_string(),
                workbook_name: student.name.clone(),
                normalized_name,
                row_index: slot.row_index,
                gender_block: Some(slot.gender_block.to_string()),
            }
        })
        .collect()
}

fn unmapped_roster_student_names(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    student_mappings: &[Sf2StudentMappingRecord],
) -> Result<Vec<String>> {
    let mapped_student_ids = student_mappings
        .iter()
        .map(|mapping| mapping.student_id.as_str())
        .collect::<HashSet<_>>();
    let students = StudentRepository::new(pool).list_by_class(Some(&template.active_class_id))?;

    Ok(students
        .into_iter()
        .filter(|student| !mapped_student_ids.contains(student.id.to_string().as_str()))
        .map(|student| student.name)
        .collect())
}

fn unmapped_roster_issue(student_names: &[String]) -> String {
    let shown = student_names
        .iter()
        .take(5)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let more = student_names.len().saturating_sub(5);
    let suffix = if more > 0 {
        format!(", and {more} more")
    } else {
        String::new()
    };

    format!(
        "{}{} {} not mapped to an SF2 learner row. Sync the SF2 workbook roster before exporting.",
        shown,
        suffix,
        if student_names.len() == 1 {
            "is"
        } else {
            "are"
        }
    )
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
    template: &Sf2TemplateRecord,
    date_mappings: &[Sf2DateMappingRecord],
    student_mappings: &[Sf2StudentMappingRecord],
) -> Vec<Sf2CellMark> {
    let row_indices = if template_owns_roster(template) {
        let row_slots = template_roster_slots();
        attendance_grid_rows(
            &row_slots,
            student_mappings.iter().map(|mapping| mapping.row_index),
        )
    } else {
        mapped_attendance_rows(student_mappings.iter().map(|mapping| mapping.row_index))
    };

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

fn template_owns_roster(template: &Sf2TemplateRecord) -> bool {
    template.source_hash.starts_with("bundled-")
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

fn mapped_attendance_rows<I>(rows: I) -> Vec<u32>
where
    I: IntoIterator<Item = u32>,
{
    let mut rows = rows
        .into_iter()
        .filter(|row_index| *row_index > 0)
        .collect::<Vec<_>>();
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
    let deleted_events: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM events
             WHERE student_id = ?1
             AND event_type = 'in'
             AND timestamp >= ?2
             AND timestamp < ?3
             AND (class_id IS NULL OR class_id = ?4)",
            params![student_id, day_start_timestamp, day_end_timestamp, class_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    transaction.execute(
        "DELETE FROM events
         WHERE student_id = ?1
         AND event_type = 'in'
         AND timestamp >= ?2
         AND timestamp < ?3
         AND (class_id IS NULL OR class_id = ?4)",
        params![student_id, day_start_timestamp, day_end_timestamp, class_id],
    )?;

    let mut created_event_id: Option<String> = None;
    if present {
        let attendance_timestamp = attendance_timestamp_for_date(date, day_start)?;
        let session_key = format!("{date}|{class_id}|day");
        let event_id = uuid::Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
             VALUES (?1, ?2, ?3, 'in', ?4, ?5, ?6, ?7, NULL)",
            params![
                event_id.as_str(),
                student_id,
                class_id,
                attendance_timestamp,
                "SF2 preview correction",
                session_key,
                "SF2 preview correction",
            ],
        )?;
        created_event_id = Some(event_id);
    }

    let metadata_json = serde_json::to_string(&serde_json::json!({
        "studentId": student_id,
        "classId": class_id,
        "date": date.to_string(),
        "present": present,
        "deletedEvents": deleted_events,
        "createdEventId": created_event_id.as_deref(),
    }))
    .map_err(|error| AppError::Internal(format!("failed to serialize audit metadata: {error}")))?;
    let summary = format!(
        "Set SF2 preview attendance for student {student_id} on {date} to {}",
        if present { "present" } else { "absent" }
    );
    record_audit_event(
        &transaction,
        AuditEventInput {
            entity_type: "attendance_event",
            entity_id: created_event_id.as_deref(),
            action: if present { "create" } else { "delete" },
            summary: &summary,
            before_json: None,
            after_json: None,
            metadata_json: Some(metadata_json),
        },
    )?;

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

fn today_string() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::StudentId;
    use crate::sf2::models::{Sf2WorkbookDate, Sf2WorkbookLearner, Sf2WorkbookSheet};
    use chrono::Datelike;

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

    fn bundled_template_record(report_month: &str) -> Sf2TemplateRecord {
        Sf2TemplateRecord {
            source_hash: "bundled-test-hash".to_string(),
            ..template_record(report_month)
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
    fn import_metadata_configures_calendar_from_report_month() {
        let expected_day = default_sf2_first_school_day("JUNE", "2025-2026").unwrap();
        let stale_year = Local::now().year() - 1;
        let stale_date = format!("{stale_year}-06-{expected_day:02}");
        let analysis = workbook_analysis_with_dates(&[&stale_date]);

        let metadata = metadata_from_import_analysis(&analysis).unwrap();

        assert!(metadata.configure_calendar);
        assert_eq!(metadata.first_school_day, Some(expected_day));
    }

    #[test]
    fn sf2_import_validation_reports_roster_mismatches_and_duplicates() {
        let current_students = vec![
            student_with_gender("Dela Cruz, Juan", Some(StudentGender::Male)),
            student_with_gender("Santos, Maria", Some(StudentGender::Female)),
            student_with_gender("Currentonly, Student", Some(StudentGender::Male)),
            student_with_gender("Duplicate, Current", Some(StudentGender::Male)),
            student_with_gender("Duplicate, Current", Some(StudentGender::Female)),
        ];
        let learners = vec![
            Sf2WorkbookLearner {
                row_index: 8,
                name: "DELA CRUZ, JUAN".to_string(),
                gender_block: Some("MALE".to_string()),
            },
            Sf2WorkbookLearner {
                row_index: 9,
                name: "Santos, Marie".to_string(),
                gender_block: Some("FEMALE".to_string()),
            },
            Sf2WorkbookLearner {
                row_index: 10,
                name: "Importedonly, Student".to_string(),
                gender_block: Some("MALE".to_string()),
            },
            Sf2WorkbookLearner {
                row_index: 11,
                name: "Duplicate, Sf2".to_string(),
                gender_block: Some("MALE".to_string()),
            },
            Sf2WorkbookLearner {
                row_index: 12,
                name: "Duplicate, Sf2".to_string(),
                gender_block: Some("MALE".to_string()),
            },
            Sf2WorkbookLearner {
                row_index: 13,
                name: String::new(),
                gender_block: Some("FEMALE".to_string()),
            },
        ];

        let validation = validate_student_list(
            "C:/official-sf2.xls",
            Some("class-1"),
            "Grade 4 - Rizal",
            &current_students,
            &learners,
        );

        assert!(validation.has_discrepancies);
        assert_eq!(validation.missing_from_sf2.len(), 4);
        assert!(validation
            .missing_from_sf2
            .iter()
            .any(|student| student.name == "Currentonly, Student"));
        assert_eq!(validation.missing_from_current.len(), 4);
        assert!(validation
            .missing_from_current
            .iter()
            .any(|learner| learner.name == "Importedonly, Student"));
        assert_eq!(validation.duplicate_current_students.len(), 1);
        assert_eq!(validation.duplicate_sf2_learners.len(), 1);
        assert_eq!(validation.missing_learner_info.len(), 1);
        assert_eq!(validation.missing_learner_info[0].row_index, 13);
        assert!(validation
            .possible_name_mismatches
            .iter()
            .any(|mismatch| mismatch.current_student.name == "Santos, Maria"
                && mismatch.sf2_learner.name == "Santos, Marie"));
    }

    #[test]
    fn sf2_import_validation_blocks_unconfirmed_mismatch_imports() {
        let current_students = vec![student_with_gender(
            "Currentonly, Student",
            Some(StudentGender::Male),
        )];
        let learners = vec![Sf2WorkbookLearner {
            row_index: 8,
            name: "Importedonly, Student".to_string(),
            gender_block: Some("MALE".to_string()),
        }];
        let validation = validate_student_list(
            "C:/official-sf2.xls",
            Some("class-1"),
            "Grade 4 - Rizal",
            &current_students,
            &learners,
        );

        let blocked = ensure_import_validation_allows(&validation, false).unwrap_err();
        assert!(blocked
            .to_string()
            .contains("Student List Mismatch Detected"));
        ensure_import_validation_allows(&validation, true).unwrap();
    }

    #[test]
    fn first_school_day_for_report_month_skips_weekend_imported_dates() {
        let year = Local::now().year();
        let weekend_day = (1..=30)
            .find(|day| {
                NaiveDate::from_ymd_opt(year, 6, *day)
                    .is_some_and(|date| date.weekday().number_from_monday() > 5)
            })
            .unwrap();
        let stale_date = format!("{}-06-{weekend_day:02}", year - 1);

        let first_day =
            first_school_day_for_report_month("JUNE", "2025-2026", [stale_date.as_str()]).unwrap();

        assert_ne!(first_day, weekend_day);
        validate_first_school_day(first_day, "JUNE", "2025-2026").unwrap();
    }

    #[test]
    fn report_month_date_mappings_use_current_year_and_skip_weekends() {
        let year = Local::now().year();
        let stale_year = year - 1;
        let weekday_day = default_sf2_first_school_day("JUNE", "2025-2026").unwrap();
        let weekend_day = (1..=30)
            .find(|day| {
                NaiveDate::from_ymd_opt(year, 6, *day)
                    .is_some_and(|date| date.weekday().number_from_monday() > 5)
            })
            .unwrap();
        let template = template_record("JUNE");
        let date_mappings = vec![
            Sf2DateMappingRecord {
                template_id: template.id.clone(),
                sheet_name: format!("JUNE {stale_year}"),
                date: format!("{stale_year}-06-{weekday_day:02}"),
                column_letter: "F".to_string(),
                column_index: 6,
            },
            Sf2DateMappingRecord {
                template_id: template.id.clone(),
                sheet_name: format!("JUNE {stale_year}"),
                date: format!("{stale_year}-06-{weekend_day:02}"),
                column_letter: "G".to_string(),
                column_index: 7,
            },
        ];

        let filtered = sf2_date_mappings_for_report_month(&template, &date_mappings);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].date, format!("{year}-06-{weekday_day:02}"));
        assert_eq!(filtered[0].column_letter, "F");
    }

    #[test]
    fn report_month_closed_days_use_current_year_month_and_skip_weekends() {
        let year = Local::now().year();
        let weekday_day = default_sf2_first_school_day("JUNE", "2025-2026").unwrap();
        let weekend_day = (1..=30)
            .find(|day| {
                NaiveDate::from_ymd_opt(year, 6, *day)
                    .is_some_and(|date| date.weekday().number_from_monday() > 5)
            })
            .unwrap();
        let weekday = format!("{year}-06-{weekday_day:02}");
        let weekend = format!("{year}-06-{weekend_day:02}");
        let other_month = format!("{year}-07-01");

        let closed_days = sf2_closed_days_for_report_month(
            &template_record("JUNE"),
            &[weekday.clone(), weekend, other_month],
        );

        assert_eq!(closed_days, vec![weekday]);
    }

    #[test]
    fn date_mappings_current_requires_current_weekday_month_year_and_sheet() {
        let year = Local::now().year();
        let weekday_day = default_sf2_first_school_day("JUNE", "2025-2026").unwrap();
        let template = template_record("JUNE");
        let current_mapping = Sf2DateMappingRecord {
            template_id: template.id.clone(),
            sheet_name: format!("JUNE {year}"),
            date: format!("{year}-06-{weekday_day:02}"),
            column_letter: "F".to_string(),
            column_index: 6,
        };
        let stale_sheet_mapping = Sf2DateMappingRecord {
            sheet_name: format!("JUNE {}", year - 1),
            ..current_mapping.clone()
        };

        assert!(date_mappings_are_current_for_report_month(
            &template,
            &[current_mapping]
        ));
        assert!(!date_mappings_are_current_for_report_month(
            &template,
            &[stale_sheet_mapping]
        ));
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

        assert!(!preview.can_export);
        assert!(preview
            .issues
            .iter()
            .any(|issue| issue.contains("not mapped to an SF2 learner row")));
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
    fn export_readiness_blocks_unmapped_roster_students() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database should be created");
        let workbook = tempfile::NamedTempFile::new().expect("workbook file should be created");
        let pool = crate::infrastructure::init_db(temp_db.path()).expect("database should init");
        let current_year = Local::now().year();
        let mapped_date = format!("{current_year}-06-08");
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
            id: "template-readiness".to_string(),
            source_path: workbook.path().to_string_lossy().to_string(),
            source_hash: "readiness-hash".to_string(),
            school_id: "123456".to_string(),
            school_name: "Sample Integrated School".to_string(),
            school_year: format!("{}-{}", current_year, current_year + 1),
            report_month: "JUNE".to_string(),
            grade_level: "7".to_string(),
            section: "Rose".to_string(),
            adviser_name: "Teacher Adviser".to_string(),
            school_head_name: "School Head".to_string(),
            layout_fingerprint: "readiness-layout".to_string(),
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
        let date_mapping = Sf2DateMappingRecord {
            template_id: template.id.clone(),
            sheet_name: format!("JUNE {current_year}"),
            date: mapped_date,
            column_letter: "F".to_string(),
            column_index: 6,
        };
        Sf2Repository::new(pool.clone())
            .upsert_template_with_mappings(&template, &[student_mapping], &[date_mapping])
            .expect("template mappings should save");

        let readiness = export_readiness(pool, Some(class.id)).expect("readiness should load");

        assert!(!readiness.can_export);
        assert!(readiness
            .issues
            .iter()
            .any(|issue| issue.contains("not mapped to an SF2 learner row")));
    }

    #[test]
    fn clear_attendance_marks_for_bundled_workbook_clears_empty_roster_rows() {
        let dates = vec![Sf2DateMappingRecord {
            template_id: "template-1".to_string(),
            sheet_name: "JUNE 2026".to_string(),
            date: "2026-06-08".to_string(),
            column_letter: "F".to_string(),
            column_index: 6,
        }];
        let template = bundled_template_record("JUNE");

        let students = vec![Sf2StudentMappingRecord {
            template_id: "template-1".to_string(),
            student_id: "student-1".to_string(),
            workbook_name: "Learner, Sample".to_string(),
            normalized_name: "LEARNER,SAMPLE".to_string(),
            row_index: 8,
            gender_block: Some("MALE".to_string()),
        }];

        let marks = clear_attendance_marks_for_records(&template, &dates, &students);

        assert_eq!(marks.len(), template_roster_slots().len());
        assert!(marks.contains(&Sf2CellMark {
            sheet_name: "JUNE 2026".to_string(),
            cell_address: "F8".to_string(),
            value: String::new(),
        }));
        assert!(marks.contains(&Sf2CellMark {
            sheet_name: "JUNE 2026".to_string(),
            cell_address: "F28".to_string(),
            value: String::new(),
        }));
        assert!(marks.contains(&Sf2CellMark {
            sheet_name: "JUNE 2026".to_string(),
            cell_address: "F30".to_string(),
            value: String::new(),
        }));
        assert!(marks.contains(&Sf2CellMark {
            sheet_name: "JUNE 2026".to_string(),
            cell_address: "F48".to_string(),
            value: String::new(),
        }));
        assert!(!marks.iter().any(|mark| mark.cell_address == "F29"));
        assert!(!marks.iter().any(|mark| mark.cell_address == "F49"));
    }

    #[test]
    fn clear_attendance_marks_for_imported_workbook_only_uses_mapped_learner_rows() {
        let dates = vec![Sf2DateMappingRecord {
            template_id: "template-1".to_string(),
            sheet_name: "JUNE 2026".to_string(),
            date: "2026-06-08".to_string(),
            column_letter: "F".to_string(),
            column_index: 6,
        }];
        let students = vec![
            Sf2StudentMappingRecord {
                template_id: "template-1".to_string(),
                student_id: "student-1".to_string(),
                workbook_name: "Learner, Sample".to_string(),
                normalized_name: "LEARNER,SAMPLE".to_string(),
                row_index: 8,
                gender_block: Some("FEMALE".to_string()),
            },
            Sf2StudentMappingRecord {
                template_id: "template-1".to_string(),
                student_id: "student-2".to_string(),
                workbook_name: "Learner, Other".to_string(),
                normalized_name: "LEARNER,OTHER".to_string(),
                row_index: 30,
                gender_block: Some("FEMALE".to_string()),
            },
        ];
        let template = template_record("JUNE");

        let marks = clear_attendance_marks_for_records(&template, &dates, &students);

        assert_eq!(marks.len(), 2);
        assert!(marks.contains(&Sf2CellMark {
            sheet_name: "JUNE 2026".to_string(),
            cell_address: "F8".to_string(),
            value: String::new(),
        }));
        assert!(marks.contains(&Sf2CellMark {
            sheet_name: "JUNE 2026".to_string(),
            cell_address: "F30".to_string(),
            value: String::new(),
        }));
        assert!(!marks.iter().any(|mark| mark.cell_address == "F29"));
        assert!(!marks.iter().any(|mark| mark.cell_address == "F49"));
    }

    #[test]
    fn workbook_learner_sync_preserves_first_dynamic_roster_row() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database should be created");
        let pool = crate::infrastructure::init_db(temp_db.path()).expect("database should init");
        let class = ClassRepository::new(pool.clone())
            .create(CreateClassRequest {
                name: "3 - Matapat".to_string(),
                room: Some("N/A".to_string()),
                day_start: "08:30".to_string(),
                day_end: "15:30".to_string(),
                late_after: "08:45".to_string(),
                sessions: Vec::new(),
                days: vec![1, 2, 3, 4, 5],
            })
            .expect("class should be created");
        let student_repo = StudentRepository::new(pool);
        let learners = vec![
            Sf2WorkbookLearner {
                row_index: 8,
                name: "CAMANIA,LIAN CARLO, SUGAY".to_string(),
                gender_block: Some("MALE".to_string()),
            },
            Sf2WorkbookLearner {
                row_index: 9,
                name: "CUARES,JAIRO, ESPIRITU".to_string(),
                gender_block: Some("MALE".to_string()),
            },
            Sf2WorkbookLearner {
                row_index: 29,
                name: "<=== MALE | TOTAL Per Day ===>".to_string(),
                gender_block: Some("MALE".to_string()),
            },
            Sf2WorkbookLearner {
                row_index: 30,
                name: "BAPTISMA,SOFIA, ESPIRITU".to_string(),
                gender_block: Some("FEMALE".to_string()),
            },
        ];

        let sync =
            sync_workbook_learner_mappings(&student_repo, &class.id, "template-1", &learners)
                .expect("learner mappings should sync");

        assert_eq!(sync.student_mappings.len(), 3);
        assert_eq!(sync.students_created, 3);
        assert_eq!(sync.student_mappings[0].row_index, 8);
        assert_eq!(
            sync.student_mappings[0].workbook_name,
            "CAMANIA,LIAN CARLO, SUGAY"
        );
        assert_eq!(
            sync.student_mappings[0].gender_block.as_deref(),
            Some("MALE")
        );
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
