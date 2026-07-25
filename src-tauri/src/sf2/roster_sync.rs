use crate::domain::error::{AppError, Result};
use crate::infrastructure::database::{ClassRepository, DbPool, StudentRepository};
use crate::sf2::attendance_marks::{summary_formula_marks, total_formula_marks};
use crate::sf2::logic::Sf2CellMark;
use crate::sf2::models::{Sf2DateMappingRecord, Sf2TemplateRecord};
use crate::sf2::repository::Sf2Repository;
use crate::sf2::roster::{reject_duplicate_roster_names, template_owns_roster};
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

    let sf2_repo = Sf2Repository::new(pool.clone());
    let existing_mappings = sf2_repo.student_mappings_for_template(&template.id)?;

    if template_owns_roster(template) {
        sync_bundled_template_roster(pool, template, &students, &existing_mappings)
    } else {
        sync_imported_workbook_roster(pool, template, &students, &existing_mappings)
    }
}

mod internal;

use internal::{sync_bundled_template_roster, sync_imported_workbook_roster};

/// Compute the TOTAL Per Day formulas and summary section marks for a roster sync.
///
/// Returns `(total_formula_marks, summary_formula_marks, summary_static_marks)`.
/// These marks should be written to the workbook after syncing the roster so that
/// the MALE/FEMALE/Combined TOTAL rows and the Enrolment summary (Row 53) reflect
/// the current student count and gender distribution.
pub(super) fn roster_sync_formula_marks(
    male_count: usize,
    female_count: usize,
    male_total_row: u32,
    female_total_row: u32,
    combined_total_row: u32,
    date_mappings: &[Sf2DateMappingRecord],
) -> (Vec<Sf2CellMark>, Vec<Sf2CellMark>, Vec<Sf2CellMark>) {
    let total_marks = total_formula_marks(
        male_count,
        female_count,
        male_total_row,
        female_total_row,
        combined_total_row,
        date_mappings,
    );

    let total_students = male_count + female_count;
    let (summary_formula_marks, summary_static_marks) = summary_formula_marks(
        male_count,
        female_count,
        total_students,
        male_total_row,
        female_total_row,
        combined_total_row,
        date_mappings,
    );

    (total_marks, summary_formula_marks, summary_static_marks)
}

#[cfg(test)]
#[path = "__tests__/roster_sync_tests.rs"]
mod tests;
