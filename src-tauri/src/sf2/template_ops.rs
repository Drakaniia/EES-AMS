use crate::domain::error::{AppError, Result};
use crate::domain::models::{Class, Settings, StudentGender};
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
    clear_unused_learner_marks, reject_duplicate_roster_names, roster_expansion_needed,
    roster_name_marks, roster_students_for_draft, student_mappings_from_roster_assignments,
    sync_workbook_learner_mappings, template_owns_roster, template_roster_assignments,
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

    // Calculate expansion needed BEFORE writing to Excel
    let roster_assignments = template_roster_assignments(&students)?;
    let male_count = students.iter().filter(|s| s.gender == Some(StudentGender::Male)).count();
    let female_count = students.iter().filter(|s| s.gender == Some(StudentGender::Female)).count();
    let (extra_male, extra_female) = roster_expansion_needed(male_count, female_count);

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

    // Compute student mappings (pure Rust, no Excel needed)
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

    // ── Phase 1: Batch all 7 Excel operations into ONE session ────────
    // Was previously 7 separate Excel startups (steps 2-7 + 9).
    let extra_male_hide = (male_count as u32).saturating_sub(21);
    let extra_female_hide = (female_count as u32).saturating_sub(19);
    let hide_male_total = 29u32 + extra_male_hide;
    let hide_female_total = 49u32 + extra_male_hide + extra_female_hide;
    let mapped_rows: Vec<u32> = roster_assignments.iter().map(|a| a.slot.row_index).collect();
    let expanded_counts = if extra_male > 0 || extra_female > 0 {
        (Some(male_count), Some(female_count))
    } else {
        (None, None)
    };
    let metadata_for_excel = metadata.clone();
    let template_id_for_excel = template_id.clone();

    let (analysis, date_mappings) = excel::batch_operations(&working_copy_path, true, move |session| {
        // Step 2: Write metadata
        session.write_metadata(&metadata_for_excel)?;

        // Step 3: Expand roster if needed
        if extra_male > 0 || extra_female > 0 {
            session.expand_roster_rows(extra_male, extra_female, None, None)?;
        }

        // Step 4: Analyze
        let analysis = session.analyze()?;

        // Compute roster marks from analysis (pure Rust, inside closure)
        let roster_marks = roster_name_marks(&analysis, &roster_assignments);
        session.write_marks(&roster_marks)?;

        // Step 6: Clear unused marks
        let clear_marks = clear_unused_learner_marks(
            &analysis, &mapped_rows, expanded_counts.0, expanded_counts.1,
        );
        if !clear_marks.is_empty() {
            session.write_marks(&clear_marks)?;
        }

        // Step 7: Hide empty learner rows
        let occupied: HashSet<u32> = roster_assignments.iter().map(|a| a.slot.row_index).collect();
        session.hide_empty_learner_rows(hide_male_total, hide_female_total, &occupied)?;

        // Compute date_mappings + formulas from analysis
        let date_mappings = date_mappings_from_analysis(&template_id_for_excel, &analysis);

        // Step 9: Write MALE/FEMALE/Combined TOTAL formulas
        let male_total_row_inner = 29u32 + extra_male;
        let female_total_row_inner = 49u32 + extra_male + extra_female;
        let combined_total_row_inner = female_total_row_inner + 1;
        let formula_marks = super::attendance_service::total_formula_marks(
            male_count,
            female_count,
            male_total_row_inner,
            female_total_row_inner,
            combined_total_row_inner,
            &date_mappings,
        );
        session.write_formulas(&formula_marks)?;

        Ok((analysis, date_mappings))
    })?;

    validate_configured_calendar(&analysis, &metadata)?;
    let layout_fingerprint = layout_fingerprint(&analysis);

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
        last_synced_at: None,
    };

    sf2_repo.upsert_template_with_mappings(&template, &student_mappings, &date_mappings)?;
    let report_dates = sf2_date_mappings_for_report_month(&template, &date_mappings)
        .iter()
        .map(|m| m.date.clone())
        .collect::<Vec<_>>();

    if let Err(error) =
        super::attendance_service::write_template_marks_for_days(pool.clone(), &template, &report_dates)
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
        students_updated: 0,
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
    let student_repo = StudentRepository::new(pool.clone());

    // Phase 1: Batch ALL Excel operations into a single session.
    // The branching logic (template_owns_roster vs imported) runs inside the
    // closure, so every Excel write happens within one Excel process.
    let pool_for_batch = pool.clone();
    let metadata_for_excel = metadata.clone();
    let existing_id_for_excel = existing.id.clone();
    let template_owns = template_owns_roster(&existing);
    let class_id_owned = class_id.to_string();
    let learner_names_owned = draft.learner_names.clone();

    let (analysis, student_mappings, students_created, students_reused, learners_found, layout_fingerprint_value) =
        excel::batch_operations(&workbook_path, true, move |session| {
            // Common: write metadata and analyze
            session.write_metadata(&metadata_for_excel)?;
            let mut analysis = session.analyze()?;

            if template_owns {
                // ── Branch 1: Bundled template (template owns roster) ─────
                let student_repo_inner = StudentRepository::new(pool_for_batch.clone());
                let (students, created, reused) = roster_students_for_draft(
                    &student_repo_inner, &class_id_owned, &learner_names_owned,
                )?;
                let roster_assignments = template_roster_assignments(&students)?;
                let male_count = students.iter()
                    .filter(|s| s.gender == Some(StudentGender::Male)).count();
                let female_count = students.iter()
                    .filter(|s| s.gender == Some(StudentGender::Female)).count();

                // Existing mappings for expansion calculation
                let sf2_repo_inner = Sf2Repository::new(pool_for_batch.clone());
                let existing_mappings = sf2_repo_inner
                    .student_mappings_for_template(&existing_id_for_excel)?;
                let existing_male_mapped = existing_mappings.iter()
                    .filter(|m| m.gender_block.as_deref() == Some("MALE")).count();
                let existing_female_mapped = existing_mappings.iter()
                    .filter(|m| m.gender_block.as_deref() == Some("FEMALE")).count();
                let extra_male = (male_count.saturating_sub(existing_male_mapped.max(21))) as u32;
                let extra_female = (female_count.saturating_sub(existing_female_mapped.max(19))) as u32;
                let male_total_row = existing_mappings.iter()
                    .filter(|m| m.gender_block.as_deref() == Some("MALE"))
                    .map(|m| m.row_index).max().map(|r| r + 1).unwrap_or(29);
                let female_total_row = existing_mappings.iter()
                    .filter(|m| m.gender_block.as_deref() == Some("FEMALE"))
                    .map(|m| m.row_index).max().map(|r| r + 1).unwrap_or(49);

                // Expand if needed
                if extra_male > 0 || extra_female > 0 {
                    session.expand_roster_rows(
                        extra_male, extra_female,
                        existing_mappings.is_empty().then_some(29).or(Some(male_total_row)),
                        existing_mappings.is_empty().then_some(49).or(Some(female_total_row)),
                    )?;
                    analysis = session.analyze()?;
                }

                // Write roster name marks
                let roster_marks = roster_name_marks(&analysis, &roster_assignments);
                session.write_marks(&roster_marks)?;

                // Clear unused learner marks
                let mapped_rows: Vec<u32> = roster_assignments.iter()
                    .map(|a| a.slot.row_index).collect();
                let expanded_counts = if extra_male > 0 || extra_female > 0 {
                    (Some(male_count), Some(female_count))
                } else {
                    (None, None)
                };
                let clear_marks = clear_unused_learner_marks(
                    &analysis, &mapped_rows, expanded_counts.0, expanded_counts.1,
                );
                if !clear_marks.is_empty() {
                    session.write_marks(&clear_marks)?;
                }

                // Hide empty learner rows
                let occupied: HashSet<u32> = roster_assignments.iter()
                    .map(|a| a.slot.row_index).collect();
                let extra_male_hide = (male_count as u32).saturating_sub(21);
                let extra_female_hide = (female_count as u32).saturating_sub(19);
                let hide_male_total = 29u32 + extra_male_hide;
                let hide_female_total = 49u32 + extra_male_hide + extra_female_hide;
                session.hide_empty_learner_rows(
                    hide_male_total, hide_female_total, &occupied,
                )?;

                // Write TOTAL formulas
                let bundle_date_mappings = date_mappings_from_analysis(
                    &existing_id_for_excel, &analysis,
                );
                let bundle_male_total_row = male_total_row + extra_male;
                let bundle_female_total_row = female_total_row + extra_male + extra_female;
                let bundle_combined_total_row = bundle_female_total_row + 1;
                let formula_marks = super::attendance_service::total_formula_marks(
                    male_count, female_count,
                    bundle_male_total_row, bundle_female_total_row, bundle_combined_total_row,
                    &bundle_date_mappings,
                );
                // Preserve best-effort semantics: formula write failures should not
                // abort the entire batch (original code used `if let Err` + warn).
                if let Err(error) = session.write_formulas(&formula_marks) {
                    log::warn!("failed to write TOTAL formula marks: {error}");
                }

                let mappings = student_mappings_from_roster_assignments(
                    &existing_id_for_excel, &roster_assignments,
                );
                let layout_fp = layout_fingerprint(&analysis);

                Ok((analysis, mappings, created, reused, students.len(), layout_fp))
            } else {
                // ── Branch 2: Imported workbook (template does NOT own roster) ─
                let student_repo_inner = StudentRepository::new(pool_for_batch.clone());
                let learner_sync = sync_workbook_learner_mappings(
                    &student_repo_inner,
                    &class_id_owned,
                    &existing_id_for_excel,
                    &analysis.learners,
                )?;

                // Clear unused learner rows
                let mapped_rows: Vec<u32> = learner_sync
                    .student_mappings.iter().map(|m| m.row_index).collect();
                let clear_marks = clear_unused_learner_marks(
                    &analysis, &mapped_rows, None, None,
                );
                if !clear_marks.is_empty() {
                    session.write_marks(&clear_marks)?;
                }

                // Hide empty learner rows
                let occupied: HashSet<u32> =
                    learner_sync.student_mappings.iter().map(|m| m.row_index).collect();
                session.hide_empty_learner_rows(29u32, 49u32, &occupied)?;

                let learners_found = learner_sync.student_mappings.len();
                let layout_fp = layout_fingerprint(&analysis);

                Ok((
                    analysis,
                    learner_sync.student_mappings,
                    learner_sync.students_created,
                    learner_sync.students_reused,
                    learners_found,
                    layout_fp,
                ))
            }
        })?;

    validate_configured_calendar(&analysis, &metadata)?;

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
        last_synced_at: None,
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
        students_updated: 0,
        dates_mapped: date_mappings.len(),
    })
}

/// Switch the active report month for an existing workbook.  This updates the
/// report month in the database, reconfigures the Excel workbook for the new
/// month (sheet visibility, date headers), re-creates the date mappings in the
/// database, and rewrites attendance marks so that past dates get an X mark
/// when no attendance was recorded.
///
/// The Excel operations are targeted at calendar/date-mappings only — much
/// lighter than a full `update_workbook_settings` call (which also handles
/// roster metadata, student name sync, and row expansion).
pub fn set_report_month(pool: DbPool, class_id: &str, report_month: &str) -> Result<()> {
    if report_month.trim().is_empty() {
        return Err(AppError::InvalidInput("Report month is required".to_string()));
    }

    let sf2_repo = Sf2Repository::new(pool.clone());

    // 1. Persist the new report month in the DB first so downstream reads see
    //    the correct value.
    let template = sf2_repo
        .latest_template_for_class(class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("No SF2 workbook imported for this class".to_string())
        })?;
    sf2_repo.set_report_month(&template.id, report_month)?;

    // 2. Reload the template so we have the current report_month.
    let updated_template = sf2_repo
        .latest_template_for_class(class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput(
                "SF2 workbook was removed while switching month".to_string(),
            )
        })?;

    // 3. Let the existing refresh function handle the heavy lifting:
    //    - Writes metadata to the Excel workbook (new report month → sheet visible)
    //    - Re-analyzes the workbook to create fresh date mappings
    //    - Persists the new mappings in the DB
    //    - Returns the fully refreshed template with current date mappings
    let refreshed = super::excel_service::refresh_template_calendar_from_saved_month(
        pool.clone(),
        &updated_template,
    )?;

    // 4. Write attendance marks (X for past dates with no attendance record)
    //    using the refreshed date mappings.
    let date_mappings = sf2_repo.date_mappings_for_template(&refreshed.id)?;
    let report_mappings = crate::sf2::calendar::sf2_date_mappings_for_report_month(
        &refreshed,
        &date_mappings,
    );
    // If the new month has no mapped dates (e.g. a month with zero school
    // days or a misconfigured workbook), log a warning and bail — not an
    // error because the user may just need to configure the calendar first.
    if report_mappings.is_empty() {
        log::warn!(
            "No date mappings found for report month {report_month} (class {class_id})"
        );
        return Ok(());
    }
    let report_dates: Vec<String> = report_mappings.iter().map(|m| m.date.clone()).collect();
    super::attendance_service::write_template_marks_for_days(
        pool,
        &refreshed,
        &report_dates,
    )?;

    Ok(())
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
