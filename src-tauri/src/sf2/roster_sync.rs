use crate::domain::error::{AppError, Result};
use crate::infrastructure::database::{ClassRepository, DbPool, StudentRepository};
use crate::sf2::calendar::{date_mappings_from_analysis, sf2_date_mappings_for_report_month};
use crate::sf2::excel;
use crate::sf2::models::Sf2TemplateRecord;
use crate::sf2::repository::Sf2Repository;
use crate::sf2::roster::{
    reject_duplicate_roster_names, roster_name_marks, student_mappings_from_roster_assignments,
    template_owns_roster, template_roster_assignments,
};
use crate::sf2::workbook_files::layout_fingerprint;
use std::path::PathBuf;

/// Sync workbook roster for a given class ID (public entry point, swallows errors as warnings)
pub fn sync_workbook_roster_for_class(pool: DbPool, class_id: &str) -> Result<()> {
    let _ = sync_latest_workbook_roster_for_class(pool, class_id)?;
    Ok(())
}

pub(crate) fn sync_latest_workbook_roster_for_class(
    pool: DbPool,
    class_id: &str,
) -> Result<Option<Sf2TemplateRecord>> {
    let sf2_repo = Sf2Repository::new(pool.clone());
    let Some(template) = sf2_repo.latest_template_for_class(class_id)? else {
        return Ok(None);
    };

    Ok(Some(sync_template_roster_from_class(pool, &template)?))
}

pub(crate) fn sync_template_roster_from_class(
    pool: DbPool,
    template: &Sf2TemplateRecord,
) -> Result<Sf2TemplateRecord> {
    if !template_owns_roster(template) {
        return Ok(template.clone());
    }

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

    let roster_assignments = template_roster_assignments(&students)?;
    let analysis = excel::analyze_workbook(&workbook_path)?;
    let roster_marks = roster_name_marks(&analysis, &roster_assignments);
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

    let report_dates = sf2_date_mappings_for_report_month(&synced_template, &date_mappings)
        .iter()
        .map(|m| m.date.clone())
        .collect::<Vec<_>>();

    if let Err(error) = super::attendance_service::write_template_marks_for_mappings(
        pool,
        &synced_template,
        &report_dates,
        &student_mappings,
        &date_mappings,
    ) {
        log::warn!("failed to backfill synced SF2 workbook marks: {error}");
    }

    Ok(synced_template)
}
