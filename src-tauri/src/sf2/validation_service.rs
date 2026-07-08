use crate::domain::error::Result;
use crate::infrastructure::database::{ClassRepository, DbPool};
use crate::sf2::calendar::{
    date_mappings_from_analysis, metadata_from_import_analysis,
    sf2_date_mappings_for_report_month, validate_configured_calendar,
};
use crate::sf2::excel;
use crate::sf2::models::{
    Sf2ImportSummary, Sf2ImportValidation, Sf2TemplateRecord, Sf2WorkbookAnalysis,
};
use crate::sf2::naming::class_name;
use crate::sf2::repository::Sf2Repository;
use crate::sf2::roster::clear_unused_learner_marks;
use crate::sf2::validation::{ensure_import_validation_allows, import_validation_from_analysis};
use crate::sf2::workbook_files::{
    copy_workbook_to_app_data, file_hash, layout_fingerprint, pick_workbook_path,
};
use std::path::{Path, PathBuf};

pub fn validate_workbook_import(
    app: tauri::AppHandle,
    pool: DbPool,
) -> Result<Sf2ImportValidation> {
    super::attendance_service::emit_sf2_progress(&app, "import", 1, 7, "Choosing SF2 workbook");
    let workbook_path = pick_workbook_path(&app)?;
    super::attendance_service::emit_sf2_progress(&app, "import", 2, 7, "Reading SF2 workbook");
    let source_analysis = excel::analyze_workbook(&workbook_path)?;
    super::attendance_service::emit_sf2_progress(&app, "import", 3, 7, "Validating learner list");

    import_validation_from_analysis(pool, &workbook_path, &source_analysis)
}

pub fn import_workbook(
    app: tauri::AppHandle,
    pool: DbPool,
    source_path: String,
    proceed_anyway: bool,
) -> Result<Sf2ImportSummary> {
    super::attendance_service::emit_sf2_progress(&app, "import", 4, 7, "Preparing workbook import");
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
    super::attendance_service::emit_sf2_progress(&app, "import", 5, 7, "Copying SF2 working workbook");
    let source_hash = file_hash(workbook_path)?;
    let class_name = class_name(&source_analysis.grade_level, &source_analysis.section);

    let class_repo = ClassRepository::new(pool.clone());
    let student_repo = crate::infrastructure::database::StudentRepository::new(pool.clone());
    let sf2_repo = Sf2Repository::new(pool.clone());

    let class = super::calendar_service::find_or_create_class(&class_repo, &class_name, None)?;
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
    super::attendance_service::emit_sf2_progress(&app, "import", 6, 7, "Writing SF2 details and date layout");
    excel::write_metadata(&working_copy_path, &metadata)?;
    let analysis = excel::analyze_workbook(&working_copy_path)?;
    validate_configured_calendar(&analysis, &metadata)?;
    let layout_fingerprint = layout_fingerprint(&analysis);

    // Fetch old template student mappings for name-update-on-reimport
    let old_mappings = sf2_repo
        .latest_template_for_class(&class.id)?
        .map(|old_template| {
            sf2_repo.student_mappings_for_template(&old_template.id)
        })
        .transpose()?
        .unwrap_or_default();

    let learner_sync =
        super::roster::sync_workbook_learner_mappings_with_old(
            &student_repo, &class.id, &template_id, &analysis.learners, &old_mappings,
        )?;
    let student_mappings = learner_sync.student_mappings;
    let students_created = learner_sync.students_created;
    let students_reused = learner_sync.students_reused;
    let students_updated = learner_sync.students_updated;

    // Clear unused learner rows (columns A, B, C) in the imported workbook
    let mapped_rows: Vec<u32> = student_mappings.iter().map(|m| m.row_index).collect();
    let clear_marks = clear_unused_learner_marks(&analysis, &mapped_rows, None, None);
    if !clear_marks.is_empty() {
        excel::write_marks(&working_copy_path, &clear_marks)?;
    }

    // Hide empty learner rows — only rows with students should be visible.
    // Standard DepEd SF2: MALE TOTAL at row 29, FEMALE TOTAL at row 49.
    {
        let occupied: std::collections::HashSet<u32> =
            student_mappings.iter().map(|m| m.row_index).collect();
        excel::hide_empty_learner_rows(
            &working_copy_path,
            29u32,
            49u32,
            &occupied,
        )?;
    }

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

    let report_dates = sf2_date_mappings_for_report_month(&template, &date_mappings)
        .iter()
        .map(|m| m.date.clone())
        .collect::<Vec<_>>();

    super::attendance_service::write_template_marks_for_mappings(
        pool.clone(),
        &template,
        &report_dates,
        &student_mappings,
        &date_mappings,
    )?;
    sf2_repo.upsert_template_with_mappings(&template, &student_mappings, &date_mappings)?;
    super::attendance_service::emit_sf2_progress(&app, "import", 7, 7, "SF2 import complete");

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
        students_updated,
        dates_mapped: date_mappings.len(),
    })
}
