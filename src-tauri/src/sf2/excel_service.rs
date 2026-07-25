use crate::domain::error::{AppError, Result};
use crate::infrastructure::database::{ClassRepository, DbPool};
use crate::sf2::sf2_metadata::{sf2_date_mappings_for_report_month, sf2_metadata_warnings, template_metadata};
use crate::sf2::excel;
use crate::sf2::models::{
    Sf2ExportReadiness, Sf2ExportResult, Sf2TemplateRecord, Sf2WorkbookSettings,
};
use crate::sf2::naming::class_name;
use crate::sf2::repository::{template_summary, Sf2Repository};
use crate::sf2::workbook_files::{open_path_in_default_app, save_workbook_path};
use std::path::{Path, PathBuf};

pub fn workbook_settings(pool: DbPool, class_id: Option<String>) -> Result<Sf2WorkbookSettings> {
    use crate::sf2::sf2_metadata::first_school_day_from_mappings;
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

pub fn export_readiness(pool: DbPool, class_id: Option<String>) -> Result<Sf2ExportReadiness> {
    let sf2_repo = Sf2Repository::new(pool.clone());
    let template = match class_id {
        Some(ref class_id) if !class_id.is_empty() => {
            sf2_repo.latest_template_for_class(class_id)?
        }
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
                last_synced_at: None,
            }),
    };

    let mut issues = Vec::new();
    let Some(ref template) = template else {
        return Ok(Sf2ExportReadiness {
            template: None,
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
    let warnings = sf2_metadata_warnings(&template_metadata(template));

    if !Path::new(&template.source_path).exists() {
        issues.push(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again."
                .to_string(),
        );
    }

    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let mapped_students = student_mappings.len();
    let unmapped_students =
        unmapped_roster_student_names(pool.clone(), template, &student_mappings)?;
    if !unmapped_students.is_empty() {
        issues.push(unmapped_roster_issue(&unmapped_students));
    }

    let date_mappings = sf2_repo.date_mappings_for_template(&template.id)?;
    let mapped_dates = sf2_date_mappings_for_report_month(template, &date_mappings).len();
    if mapped_dates == 0 {
        issues.push("No attendance dates are mapped to this SF2 report month.".to_string());
    }

    Ok(Sf2ExportReadiness {
        template: Some(template_summary(
            // Take ownership by reconstructing
            Sf2TemplateRecord {
                id: template.id.clone(),
                source_path: template.source_path.clone(),
                source_hash: template.source_hash.clone(),
                school_id: template.school_id.clone(),
                school_name: template.school_name.clone(),
                school_year: template.school_year.clone(),
                report_month: template.report_month.clone(),
                grade_level: template.grade_level.clone(),
                section: template.section.clone(),
                adviser_name: template.adviser_name.clone(),
                school_head_name: template.school_head_name.clone(),
                layout_fingerprint: template.layout_fingerprint.clone(),
                active_class_id: template.active_class_id.clone(),
                imported_at: template.imported_at,
                last_synced_at: template.last_synced_at,
            },
        )),
        mapped_students,
        mapped_dates,
        can_export: issues.is_empty(),
        issues,
        warnings,
    })
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
    let template = refresh_template_calendar_from_saved_month(pool.clone(), &template, false)?;
    let template =
        super::calendar_service::sync_template_roster_from_class(pool.clone(), &template)?;

    let working_copy_path = PathBuf::from(&template.source_path);
    if !working_copy_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let mapped_dates = sf2_date_mappings_for_report_month(
        &template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    if mapped_dates.is_empty() {
        return Err(AppError::InvalidInput(
            "No attendance dates are mapped to this SF2 report month.".to_string(),
        ));
    }
    let report_dates = mapped_dates
        .iter()
        .map(|m| m.date.clone())
        .collect::<Vec<_>>();

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

    let marks_written = super::progress::write_template_marks_for_days(
        pool.clone(),
        &template,
        &report_dates,
    )?;

    let metadata = template_metadata(&template);
    excel::write_metadata(&working_copy_path, &metadata)?;

    std::fs::copy(&working_copy_path, &output_path)
        .map_err(|error| AppError::Internal(format!("failed to export SF2 workbook: {error}")))?;
    open_path_in_default_app(&output_path)?;

    Ok(Sf2ExportResult {
        output_path: output_path.to_string_lossy().to_string(),
        marks_written,
    })
}

pub(super) fn refresh_template_calendar_from_saved_month(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    force_refresh: bool,
) -> Result<Sf2TemplateRecord> {
use crate::sf2::attendance_marks;
use crate::sf2::calendar::{first_school_day_for_report_month, sf2_month_number};
use crate::sf2::sf2_metadata::{
    date_mappings_are_current_for_report_month, date_mappings_from_analysis,
};
    use crate::sf2::workbook_files::layout_fingerprint;

    let Some(_) = sf2_month_number(&template.report_month) else {
        return Ok(template.clone());
    };

    let sf2_repo = Sf2Repository::new(pool);
    let existing_date_mappings = sf2_repo.date_mappings_for_template(&template.id)?;
    if !force_refresh
        && date_mappings_are_current_for_report_month(template, &existing_date_mappings)
    {
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

    // Pre-compute values needed inside the move closure — avoids capturing
    // student_mappings (needed again after the closure) while still having
    // ownership of the data the closure needs.
    let owns_roster = crate::sf2::roster_parser::template_owns_roster(template);
    let male_count = student_mappings
        .iter()
        .filter(|m| m.gender_block.as_deref() == Some("MALE"))
        .count();
    let female_count = student_mappings
        .iter()
        .filter(|m| m.gender_block.as_deref() == Some("FEMALE"))
        .count();
    let template_id_for_closure = template.id.clone();
    let student_mappings_for_update = student_mappings.clone();

    let metadata_for_excel = metadata.clone();
    let analysis = excel::batch_operations(&workbook_path, true, move |session| {
        session.write_metadata(&metadata_for_excel)?;
        let analysis = session.analyze()?;

        // Regenerate TOTAL Per Day formulas for bundled templates when switching
        // months. The calendar dates change, so stale static values inherited from
        // the original template must be cleared first, then new formulas written.
        if owns_roster {
            let (male_total_row, female_total_row, combined_total_row) =
                crate::sf2::roster_parser::bundled_template_total_rows(male_count, female_count);
            let inner_date_mappings =
                date_mappings_from_analysis(&template_id_for_closure, &analysis);

            // Clear stale template defaults from all weekday columns (6–38)
            // so columns without dates in this month show nothing.
            let clear_marks = attendance_marks::clear_total_cell_marks(
                male_total_row,
                female_total_row,
                combined_total_row,
                &inner_date_mappings,
            );
            if !clear_marks.is_empty() {
                session.write_marks_force(&clear_marks)?;
            }

            // Write fresh formulas only for columns that have valid dates.
            let formula_marks = attendance_marks::total_formula_marks(
                male_count,
                female_count,
                male_total_row,
                female_total_row,
                combined_total_row,
                &inner_date_mappings,
            );
            if !formula_marks.is_empty() {
                session.write_formulas(&formula_marks)?;
            }

            // Rewrite summary section (Enrolment, Registered Learners, ADA, etc.)
            // to match current student counts.
            let total = male_count + female_count;
            let (summary_formulas, summary_static) =
                attendance_marks::summary_formula_marks(
                    male_count,
                    female_count,
                    total,
                    male_total_row,
                    female_total_row,
                    combined_total_row,
                    &inner_date_mappings,
                );
            session.write_formulas(&summary_formulas)?;
            session.write_marks_force(&summary_static)?;
        }

        Ok(analysis)
    })?;
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
        last_synced_at: template.last_synced_at,
    };
    let refreshed_date_mappings = date_mappings_from_analysis(&template.id, &analysis);
    sf2_repo.update_template_with_mappings(
        &refreshed_template,
        &student_mappings_for_update,
        &refreshed_date_mappings,
    )?;

    Ok(refreshed_template)
}

use crate::sf2::excel_service_helpers::{
    latest_template_for_request, unmapped_roster_issue, unmapped_roster_student_names,
};

#[cfg(test)]
#[path = "__tests__/excel_service_tests.rs"]
mod tests;
