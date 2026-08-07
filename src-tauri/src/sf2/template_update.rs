use crate::domain::error::{AppError, Result};
use crate::domain::models::StudentGender;
use crate::infrastructure::database::{ClassRepository, DbPool, StudentRepository};
use crate::sf2::attendance_marks::{summary_formula_marks, total_formula_marks};
use crate::sf2::calendar::validate_configured_calendar;
use crate::sf2::excel;
use crate::sf2::models::{Sf2ImportSummary, Sf2TemplateDraft, Sf2TemplateRecord};
use crate::sf2::repository::Sf2Repository;
use crate::sf2::roster::{
    bundled_template_total_rows, clear_unused_learner_marks, roster_name_marks,
    roster_students_for_draft, student_mappings_from_roster_assignments,
    sync_workbook_learner_mappings, template_owns_roster, template_roster_assignments,
};
use crate::sf2::sf2_metadata::{
    date_mappings_from_analysis, metadata_from_draft, sf2_date_mappings_for_report_month,
};
use crate::sf2::workbook_files::layout_fingerprint;
use std::collections::HashSet;
use std::path::PathBuf;

/// Update an existing workbook's settings, optionally recreating the roster
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

    // Phase 1: Batch ALL Excel operations into a single session.
    // The branching logic (template_owns_roster vs imported) runs inside the
    // closure, so every Excel write occurs within one Excel process.
    let pool_for_batch = pool.clone();
    let metadata_for_excel = metadata.clone();
    let existing_id_for_excel = existing.id.clone();
    let template_owns = template_owns_roster(&existing);
    let class_id_owned = class_id.to_string();
    let learner_names_owned = draft.learner_names.clone();

    let (
        analysis,
        student_mappings,
        students_created,
        students_reused,
        learners_found,
        layout_fingerprint_value,
    ) = excel::batch_operations(&workbook_path, true, move |session| {
        // Common: write metadata and analyze
        session.write_metadata(&metadata_for_excel)?;
        let mut analysis = session.analyze()?;

        if template_owns {
            // ── Branch 1: Bundled template (template owns roster) ─────
            let student_repo_inner = StudentRepository::new(pool_for_batch.clone());
            let (students, created, reused) = roster_students_for_draft(
                &student_repo_inner,
                &class_id_owned,
                &learner_names_owned,
            )?;
            let roster_assignments = template_roster_assignments(&students)?;
            let male_count = students
                .iter()
                .filter(|s| s.gender == Some(StudentGender::Male))
                .count();
            let female_count = students
                .iter()
                .filter(|s| s.gender == Some(StudentGender::Female))
                .count();

            // Existing mappings for expansion calculation
            let sf2_repo_inner = Sf2Repository::new(pool_for_batch.clone());
            let existing_mappings =
                sf2_repo_inner.student_mappings_for_template(&existing_id_for_excel)?;
            let existing_male_mapped = existing_mappings
                .iter()
                .filter(|m| m.gender_block.as_deref() == Some("MALE"))
                .count();
            let existing_female_mapped = existing_mappings
                .iter()
                .filter(|m| m.gender_block.as_deref() == Some("FEMALE"))
                .count();
            let extra_male = (male_count.saturating_sub(existing_male_mapped.max(21))) as u32;
            let extra_female = (female_count.saturating_sub(existing_female_mapped.max(19))) as u32;

            // Row positions derived from the slot layout — adapts to any expansion.
            let (male_total_row, female_total_row, combined_total_row) =
                bundled_template_total_rows(male_count, female_count);

            let current_male_capacity = existing_male_mapped.max(21) as u32;
            let current_extra_male = current_male_capacity.saturating_sub(21);
            let current_female_capacity = existing_female_mapped.max(19) as u32;
            let current_male_total = 8u32 + current_male_capacity;
            let current_female_total = 30u32 + current_extra_male + current_female_capacity;

            // Expand if needed
            if extra_male > 0 || extra_female > 0 {
                session.expand_roster_rows(
                    extra_male,
                    extra_female,
                    existing_mappings
                        .is_empty()
                        .then_some(29)
                        .or(Some(current_male_total)),
                    existing_mappings
                        .is_empty()
                        .then_some(49)
                        .or(Some(current_female_total)),
                )?;
                analysis = session.analyze()?;
            }

            // Write roster name marks
            let roster_marks = roster_name_marks(&analysis, &roster_assignments);
            session.write_marks(&roster_marks)?;

            // Clear unused learner marks
            let mapped_rows: Vec<u32> = roster_assignments
                .iter()
                .map(|a| a.slot.row_index)
                .collect();
            let expanded_counts = if extra_male > 0 || extra_female > 0 {
                (Some(male_count), Some(female_count))
            } else {
                (None, None)
            };
            let clear_marks = clear_unused_learner_marks(
                &analysis,
                &mapped_rows,
                expanded_counts.0,
                expanded_counts.1,
            );
            if !clear_marks.is_empty() {
                session.write_marks(&clear_marks)?;
            }

            // Hide empty learner rows
            let occupied: HashSet<u32> = roster_assignments
                .iter()
                .map(|a| a.slot.row_index)
                .collect();
            session.hide_empty_learner_rows(male_total_row, female_total_row, &occupied)?;

            // Prepare date mappings and TOTAL row positions (needed for both
            // the clear step and the formula writes that follow).
            let bundle_date_mappings =
                date_mappings_from_analysis(&existing_id_for_excel, &analysis);
            let bundle_male_total_row = male_total_row;
            let bundle_female_total_row = female_total_row;
            let bundle_combined_total_row = combined_total_row;

            // Clear stale TOTAL cell values from all weekday columns (6–38)
            // so columns without dates in this month end up clean/empty rather
            // than showing a stale value inherited from the bundled template.
            let total_sheet_names: Vec<&str> = bundle_date_mappings
                .iter()
                .map(|m| m.sheet_name.as_str())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            for sheet_name in &total_sheet_names {
                session.clear_total_rows(
                    sheet_name,
                    bundle_male_total_row,
                    bundle_female_total_row,
                    bundle_combined_total_row,
                )?;
            }

            // Write TOTAL formulas
            let formula_marks = total_formula_marks(
                male_count,
                female_count,
                bundle_male_total_row,
                bundle_female_total_row,
                bundle_combined_total_row,
                &bundle_date_mappings,
            );
            // Preserve best-effort semantics: formula write failures should not
            // abort the entire batch (original code used `if let Err` + warn).
            if let Err(error) = session.write_formulas(&formula_marks) {
                log::warn!("failed to write TOTAL formula marks: {error}");
            }

            // Write summary section formulas (rows 53-65: Enrolment, Registered Learners, % of Enrolment, ADA, % of Attendance)
            let total_bundle = male_count + female_count;
            let (summary_marks_bundle, summary_static_bundle) = summary_formula_marks(
                male_count,
                female_count,
                total_bundle,
                bundle_male_total_row,
                bundle_female_total_row,
                bundle_combined_total_row,
                &bundle_date_mappings,
            );
            if let Err(error) = session.write_formulas(&summary_marks_bundle) {
                log::warn!("failed to write summary formula marks: {error}");
            }
            if let Err(error) = session.write_marks_force(&summary_static_bundle) {
                log::warn!("failed to write summary static marks: {error}");
            }

            let mappings = student_mappings_from_roster_assignments(
                &existing_id_for_excel,
                &roster_assignments,
            );
            let layout_fp = layout_fingerprint(&analysis);

            Ok((
                analysis,
                mappings,
                created,
                reused,
                students.len(),
                layout_fp,
            ))
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
                .student_mappings
                .iter()
                .map(|m| m.row_index)
                .collect();
            let clear_marks = clear_unused_learner_marks(&analysis, &mapped_rows, None, None);
            if !clear_marks.is_empty() {
                session.write_marks(&clear_marks)?;
            }

            // Hide empty learner rows
            let occupied: HashSet<u32> = learner_sync
                .student_mappings
                .iter()
                .map(|m| m.row_index)
                .collect();
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
        super::progress::write_template_marks_for_days(pool, &template, &report_dates)
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
