use crate::domain::error::{AppError, Result};
use crate::domain::models::{Class, Settings};
use crate::infrastructure::database::{ClassRepository, DbPool, StudentRepository, SettingsRepository};
use crate::sf2::calendar::{
    date_mappings_from_analysis, metadata_from_draft, sf2_date_mappings_for_report_month,
    validate_configured_calendar,
};
use crate::sf2::excel;
use crate::sf2::models::{
    Sf2StudentMappingRecord, Sf2TemplateDraft, Sf2TemplateRecord, Sf2ImportSummary,
};
use crate::sf2::naming::class_name;
use crate::sf2::repository::Sf2Repository;
use crate::sf2::roster::{
    reject_duplicate_roster_names, roster_name_marks, roster_students_for_draft,
    student_mappings_from_roster_assignments, sync_workbook_learner_mappings,
    template_owns_roster, template_roster_assignments, template_roster_slots,
};
use crate::sf2::workbook_files::{
    hash_bytes, layout_fingerprint, sf2_workbook_dir, write_bundled_template_to_dir,
    write_temp_binary_file, BUNDLED_TEMPLATE_BYTES,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Create a new SF2 workbook from the bundled template
pub fn create_workbook_from_template<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    pool: DbPool,
    draft: Sf2TemplateDraft,
) -> Result<Sf2ImportSummary> {
    super::attendance_service::emit_sf2_progress(
        &app,
        "create",
        1,
        2,
        "Creating SF2 working workbook",
    );
    let workbook_dir = sf2_workbook_dir(&app)?;
    let summary = create_workbook_from_template_in_dir(&workbook_dir, pool, draft)?;
    super::attendance_service::emit_sf2_progress(&app, "create", 2, 2, "SF2 workbook ready");
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

    let roster_assignments = template_roster_assignments(&students)?;
    let slot_count = template_roster_slots().len();
    if students.len() > slot_count {
        return Err(AppError::InvalidInput(format!(
            "The bundled SF2 template has {} learner rows, but this class has {} learners",
            slot_count,
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
    let roster_marks = roster_name_marks(&analysis, &roster_assignments);
    excel::write_marks(&working_copy_path, &roster_marks)?;

    let mut seen_normalized_names = HashSet::new();
    let student_mappings = roster_assignments
        .iter()
        .map(|assignment| {
            let student = &assignment.student;
            let slot = assignment.slot;
            let normalized_name = crate::sf2::roster::unique_normalized_name(
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
    let report_dates = sf2_date_mappings_for_report_month(&template, &date_mappings)
        .iter()
        .map(|m| m.date.clone())
        .collect::<Vec<_>>();

    if let Err(error) =
        super::attendance_service::write_template_marks_for_days(pool, &template, &report_dates)
    {
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

/// Update an existing workbook's settings, optionally recreating the roster
pub fn update_workbook_settings(
    pool: DbPool,
    draft: Sf2TemplateDraft,
) -> Result<Sf2ImportSummary> {
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
    let layout_fingerprint_value = layout_fingerprint(&analysis);
    let student_repo = StudentRepository::new(pool.clone());
    let (student_mappings, students_created, students_reused, learners_found) =
        if template_owns_roster(&existing) {
            let (students, created, reused) =
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

            let roster_marks = roster_name_marks(&analysis, &roster_assignments);
            excel::write_marks(&workbook_path, &roster_marks)?;

            let mappings = student_mappings_from_roster_assignments(
                &existing.id,
                &roster_assignments,
            );
            (mappings, created, reused, students.len())
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
        layout_fingerprint: layout_fingerprint_value,
        active_class_id: class.id.clone(),
        imported_at: chrono::Utc::now().timestamp(),
    };

    sf2_repo.update_template_with_mappings(&template, &student_mappings, &date_mappings)?;
    let report_dates = sf2_date_mappings_for_report_month(&template, &date_mappings)
        .iter()
        .map(|m| m.date.clone())
        .collect::<Vec<_>>();

    if let Err(error) =
        super::attendance_service::write_template_marks_for_days(pool, &template, &report_dates)
    {
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

fn resolve_template_class(
    class_repo: &ClassRepository,
    class_id: Option<&str>,
    metadata: &crate::sf2::models::Sf2WorkbookMetadata,
    settings: &Settings,
) -> Result<Class> {
    if let Some(class_id) = class_id.map(str::trim).filter(|value| !value.is_empty()) {
        return class_repo
            .get(class_id)?
            .ok_or_else(|| AppError::InvalidInput("Selected class was not found".to_string()));
    }

    let class_name_value = class_name(&metadata.grade_level, &metadata.section);
    crate::sf2::roster::find_or_create_class(class_repo, &class_name_value, Some(settings))
}
