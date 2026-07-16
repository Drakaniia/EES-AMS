use crate::domain::error::Result;
use crate::domain::models::StudentGender;
use crate::infrastructure::database::{ClassRepository, DbPool};
use crate::sf2::calendar::{
    date_mappings_from_analysis, metadata_from_import_analysis,
    sf2_date_mappings_for_report_month, validate_configured_calendar,
};
use crate::sf2::excel;
use crate::sf2::models::{
    Sf2ImportSummary, Sf2ImportValidation, Sf2StudentMappingRecord, Sf2TemplateRecord,
    Sf2WorkbookAnalysis,
};
use crate::sf2::naming::class_name;
use crate::sf2::repository::Sf2Repository;
use crate::sf2::roster::{
    clear_unused_learner_marks, reject_duplicate_roster_names, roster_expansion_needed,
    roster_name_marks, template_roster_assignments, unique_normalized_name,
};
use crate::sf2::validation::{ensure_import_validation_allows, import_validation_from_analysis};
use crate::sf2::workbook_files::{
    layout_fingerprint, pick_workbook_path, sf2_workbook_dir, write_bundled_template_to_dir,
};
use std::collections::HashSet;
use std::path::PathBuf;

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

    import_workbook_with_analysis(app, pool, source_analysis)
}

fn import_workbook_with_analysis(
    app: tauri::AppHandle,
    pool: DbPool,
    source_analysis: Sf2WorkbookAnalysis,
) -> Result<Sf2ImportSummary> {
    super::attendance_service::emit_sf2_progress(
        &app,
        "import",
        5,
        7,
        "Extracting students from imported workbook",
    );

    let class_name = class_name(&source_analysis.grade_level, &source_analysis.section);
    let class_repo = ClassRepository::new(pool.clone());
    let student_repo = crate::infrastructure::database::StudentRepository::new(pool.clone());
    let sf2_repo = Sf2Repository::new(pool.clone());

    let class =
        super::calendar_service::find_or_create_class(&class_repo, &class_name, None)?;

    let source_hash = format!("bundled-import-{}", class.id);
    let grade_level = source_analysis.grade_level.clone();
    let section = source_analysis.section.clone();

    // Use existing template ID on re-import (same source_hash, grade_level, section)
    // to avoid FK violation — the UPSERT preserves the old id on conflict, so all
    // subsequent student/date mappings must reference that same id.
    let template_id = sf2_repo
        .find_template(&source_hash, &grade_level, &section)?
        .map(|template| template.id)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Fetch old template student mappings for name-update-on-reimport
    let old_mappings = sf2_repo
        .latest_template_for_class(&class.id)?
        .map(|old_template| {
            sf2_repo.student_mappings_for_template(&old_template.id)
        })
        .transpose()?
        .unwrap_or_default();

    // Step 1: Sync students from imported workbook learners to DB
    let learner_sync = super::roster::sync_workbook_learner_mappings_with_old(
        &student_repo,
        &class.id,
        &template_id,
        &source_analysis.learners,
        &old_mappings,
    )?;
    let students_created = learner_sync.students_created;
    let students_reused = learner_sync.students_reused;
    let students_updated = learner_sync.students_updated;

    // Step 2: Query all students for this class from DB
    let students = student_repo.list_by_class(Some(&class.id))?;
    reject_duplicate_roster_names(&students)?;

    // Step 3: Assign students to bundled template row slots (8-28 male, 30-48 female)
    let roster_assignments = template_roster_assignments(&students)?;
    let male_count = students
        .iter()
        .filter(|s| s.gender == Some(StudentGender::Male))
        .count();
    let female_count = students
        .iter()
        .filter(|s| s.gender == Some(StudentGender::Female))
        .count();
    let (extra_male, extra_female) = roster_expansion_needed(male_count, female_count);

    // Step 4: Create fresh working copy from the bundled automated template
    super::attendance_service::emit_sf2_progress(
        &app,
        "import",
        5,
        7,
        "Creating working copy from automated template",
    );
    let workbook_dir = sf2_workbook_dir(&app)?;
    let working_copy_path = write_bundled_template_to_dir(
        &workbook_dir,
        &template_id,
        &grade_level,
        &section,
    )?;

    // Step 5: Write metadata from imported file into the bundled template
    let metadata = metadata_from_import_analysis(&source_analysis)?;
    super::attendance_service::emit_sf2_progress(
        &app,
        "import",
        6,
        7,
        "Writing SF2 details and date layout",
    );
    excel::write_metadata(&working_copy_path, &metadata)?;

    // Step 6: Expand the workbook if more than 21 male / 19 female students
    if extra_male > 0 || extra_female > 0 {
        excel::expand_roster_rows(
            &working_copy_path,
            extra_male,
            extra_female,
            None,
            None,
        )?;
    }

    // Step 7: Analyze the bundled template to get its sheet layout
    let analysis = excel::analyze_workbook(&working_copy_path)?;
    validate_configured_calendar(&analysis, &metadata)?;
    let fingerprint = layout_fingerprint(&analysis);

    // Step 8: Write student names into the bundled template
    let roster_marks = roster_name_marks(&analysis, &roster_assignments);
    if !roster_marks.is_empty() {
        excel::write_marks(&working_copy_path, &roster_marks)?;
    }

    // Step 9: Clear unused learner rows in the template
    let mapped_rows: Vec<u32> =
        roster_assignments.iter().map(|a| a.slot.row_index).collect();
    let expanded_counts = if extra_male > 0 || extra_female > 0 {
        (Some(male_count), Some(female_count))
    } else {
        (None, None)
    };
    let clear_marks =
        clear_unused_learner_marks(&analysis, &mapped_rows, expanded_counts.0, expanded_counts.1);
    if !clear_marks.is_empty() {
        excel::write_marks(&working_copy_path, &clear_marks)?;
    }

    // Step 10: Hide empty learner rows — only rows with students should be visible.
    let occupied: HashSet<u32> =
        roster_assignments.iter().map(|a| a.slot.row_index).collect();
    let extra_male_hide = (male_count as u32).saturating_sub(21);
    let extra_female_hide = (female_count as u32).saturating_sub(19);
    let hide_male_total = 29u32 + extra_male_hide;
    let hide_female_total = 49u32 + extra_male_hide + extra_female_hide;
    excel::hide_empty_learner_rows(
        &working_copy_path,
        hide_male_total,
        hide_female_total,
        &occupied,
    )?;

    // Step 11: Create student mappings for bundled template slots
    let mut seen_normalized_names = HashSet::new();
    let student_mappings: Vec<Sf2StudentMappingRecord> = roster_assignments
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
        .collect();

    // Step 12: Date mappings from the bundled template
    let date_mappings = date_mappings_from_analysis(&template_id, &analysis);

    // Step 13: Create template record pointing to the bundled-template working copy
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
        layout_fingerprint: fingerprint,
        active_class_id: class.id.clone(),
        imported_at: chrono::Utc::now().timestamp(),
        last_synced_at: None,
    };

    // Step 14: Write Excel formulas for MALE TOTAL, FEMALE TOTAL, Combined TOTAL
    let male_total_row = 29u32 + extra_male;
    let female_total_row = 49u32 + extra_male + extra_female;
    let combined_total_row = female_total_row + 1;
    let formula_marks = super::attendance_service::total_formula_marks(
        male_count,
        female_count,
        male_total_row,
        female_total_row,
        combined_total_row,
        &date_mappings,
    );
    if let Err(error) = excel::write_formulas(&working_copy_path, &formula_marks) {
        log::warn!("failed to write TOTAL formula marks: {error}");
    }

    // Step 15: Backfill attendance marks and persist to DB
    sf2_repo.upsert_template_with_mappings(&template, &student_mappings, &date_mappings)?;

    let report_dates = sf2_date_mappings_for_report_month(&template, &date_mappings)
        .iter()
        .map(|m| m.date.clone())
        .collect::<Vec<_>>();

    if let Err(error) = super::attendance_service::write_template_marks_for_days(
        pool.clone(),
        &template,
        &report_dates,
    ) {
        log::warn!("failed to backfill imported workbook marks: {error}");
    }

    super::attendance_service::emit_sf2_progress(&app, "import", 7, 7, "SF2 import complete");

    Ok(Sf2ImportSummary {
        template_id,
        class_id: class.id,
        class_name,
        source_path: working_copy_path.to_string_lossy().to_string(),
        school_year: template.school_year,
        grade_level,
        section,
        learners_found: student_mappings.len(),
        students_created,
        students_reused,
        students_updated,
        dates_mapped: date_mappings.len(),
    })
}
