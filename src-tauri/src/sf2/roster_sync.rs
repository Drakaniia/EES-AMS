use crate::domain::error::{AppError, Result};
use crate::domain::models::{Student, StudentGender};
use crate::infrastructure::database::{ClassRepository, DbPool, StudentRepository};
use crate::sf2::attendance_service::{
    clear_total_cell_marks, summary_formula_marks, total_formula_marks,
};
use crate::sf2::calendar::{date_mappings_from_analysis, sf2_date_mappings_for_report_month};
use crate::sf2::excel;
use crate::sf2::excel_com::workbook::month_number;
use crate::sf2::logic::{normalize_learner_name, Sf2CellMark};
use crate::sf2::models::{
    Sf2DateMappingRecord, Sf2StudentMappingRecord, Sf2TemplateRecord, Sf2WorkbookLearner,
};
use crate::sf2::repository::Sf2Repository;
use crate::sf2::roster::{
    bundled_template_total_rows, clear_unused_learner_marks, reject_duplicate_roster_names,
    roster_name_marks, student_mappings_from_roster_assignments, template_owns_roster,
    template_roster_assignments,
};
use crate::sf2::workbook_files::layout_fingerprint;
use std::collections::HashSet;
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

/// Sync a bundled template's roster: re-assigns ALL students to dynamic row slots,
/// expanding the workbook if the student count exceeds current capacity.
fn sync_bundled_template_roster(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    students: &[Student],
    existing_mappings: &[Sf2StudentMappingRecord],
) -> Result<Sf2TemplateRecord> {
    let workbook_path = PathBuf::from(&template.source_path);
    let sf2_repo = Sf2Repository::new(pool.clone());

    let male_count = students
        .iter()
        .filter(|s| s.gender == Some(StudentGender::Male))
        .count();
    let female_count = students
        .iter()
        .filter(|s| s.gender == Some(StudentGender::Female))
        .count();

    let existing_male_mapped = existing_mappings
        .iter()
        .filter(|m| m.gender_block.as_deref() == Some("MALE"))
        .count();
    let existing_female_mapped = existing_mappings
        .iter()
        .filter(|m| m.gender_block.as_deref() == Some("FEMALE"))
        .count();

    let current_male_capacity = existing_male_mapped.max(21);
    let current_female_capacity = existing_female_mapped.max(19);
    let extra_male = male_count.saturating_sub(current_male_capacity) as u32;
    let extra_female = female_count.saturating_sub(current_female_capacity) as u32;

    let roster_assignments = template_roster_assignments(students)?;

    // Row positions derived from the slot layout — automatically adapts to any
    // expansion without hardcoding 29/49 base values.
    let (male_total_row, female_total_row, combined_total_row) =
        bundled_template_total_rows(male_count, female_count);

    let current_male_capacity = existing_male_mapped.max(21) as u32;
    let current_extra_male = current_male_capacity.saturating_sub(21);
    let current_female_capacity = existing_female_mapped.max(19) as u32;
    // Compute where the total rows CURRENTLY sit before any expansion.
    let current_male_total = 8u32 + current_male_capacity;
    let current_female_total = 30u32 + current_extra_male + current_female_capacity;

    if extra_male > 0 || extra_female > 0 {
        excel::expand_roster_rows(
            &workbook_path,
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
    }

    let analysis = excel::analyze_workbook(&workbook_path)?;
    let roster_marks = roster_name_marks(&analysis, &roster_assignments);
    excel::write_marks(&workbook_path, &roster_marks)?;

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
        excel::write_marks(&workbook_path, &clear_marks)?;
    }

    // Hide empty learner rows — only rows with students should be visible.
    // TOTAL row positions derived from slot layout via bundled_template_total_rows().
    let occupied_rows: HashSet<u32> = roster_assignments
        .iter()
        .map(|a| a.slot.row_index)
        .collect();
    excel::hide_empty_learner_rows(
        &workbook_path,
        male_total_row,
        female_total_row,
        &occupied_rows,
    )?;

    let refreshed_analysis = excel::analyze_workbook(&workbook_path)?;
    let student_mappings =
        student_mappings_from_roster_assignments(&template.id, &roster_assignments);
    let date_mappings = date_mappings_from_analysis(&template.id, &refreshed_analysis);

    // Clear stale TOTAL cell values from all weekday columns (6–38) before
    // rewriting formulas. Without this, columns without a date in the current
    // report month retain stale values inherited from the bundled template.
    let clear_marks = clear_total_cell_marks(
        male_total_row,
        female_total_row,
        combined_total_row,
        &date_mappings,
    );
    if !clear_marks.is_empty() {
        if let Err(error) = excel::write_marks_force(&workbook_path, &clear_marks) {
            log::warn!("failed to clear stale TOTAL formula cells during roster sync: {error}");
        }
    }

    // ── Write TOTAL Per Day formulas and summary section ───────────────
    // After syncing the roster (student names may have changed, counts may
    // differ), the MALE/FEMALE/Combined TOTAL rows and the Enrolment summary
    // must be rewritten so they reflect the current student gender counts.
    let (total_formulas, summary_formulas, summary_static) = roster_sync_formula_marks(
        male_count,
        female_count,
        male_total_row,
        female_total_row,
        combined_total_row,
        &date_mappings,
    );
    if !total_formulas.is_empty() {
        if let Err(error) = excel::write_formulas(&workbook_path, &total_formulas) {
            log::warn!("failed to write TOTAL formula marks during roster sync: {error}");
        }
    }
    if !summary_formulas.is_empty() {
        if let Err(error) = excel::write_formulas(&workbook_path, &summary_formulas) {
            log::warn!("failed to write summary formula marks during roster sync: {error}");
        }
    }
    if !summary_static.is_empty() {
        if let Err(error) = excel::write_marks_force(&workbook_path, &summary_static) {
            log::warn!("failed to write summary static marks during roster sync: {error}");
        }
    }

    let synced_template = Sf2TemplateRecord {
        layout_fingerprint: layout_fingerprint(&refreshed_analysis),
        ..template.clone()
    };

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

/// Sync an imported workbook's roster: map new DB students to available empty learner rows.
/// Analyses the workbook to find which learner rows are not yet mapped, then assigns
/// new students to those rows. Returns a clear error if there are more new students
/// than available rows.
fn sync_imported_workbook_roster(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    students: &[Student],
    existing_mappings: &[Sf2StudentMappingRecord],
) -> Result<Sf2TemplateRecord> {
    let workbook_path = PathBuf::from(&template.source_path);
    let sf2_repo = Sf2Repository::new(pool.clone());

    let analysis = excel::analyze_workbook(&workbook_path)?;

    // Build a set of row indices already mapped from the DB
    let mapped_row_set: std::collections::HashSet<u32> =
        existing_mappings.iter().map(|m| m.row_index).collect();

    // Find unmapped learner rows from the workbook analysis, split by gender block.
    // These are rows in the workbook that have learner names but aren't yet linked to a DB student.
    let unmapped_male_learner_rows: Vec<&Sf2WorkbookLearner> = analysis
        .learners
        .iter()
        .filter(|learner| {
            learner.gender_block.as_deref() == Some("MALE")
                && !mapped_row_set.contains(&learner.row_index)
        })
        .collect();
    let unmapped_female_learner_rows: Vec<&Sf2WorkbookLearner> = analysis
        .learners
        .iter()
        .filter(|learner| {
            learner.gender_block.as_deref() == Some("FEMALE")
                && !mapped_row_set.contains(&learner.row_index)
        })
        .collect();

    // Find DB students that don't have a mapping yet (by student_id), split by gender
    let mapped_student_set: std::collections::HashSet<String> = existing_mappings
        .iter()
        .map(|m| m.student_id.clone())
        .collect();
    let new_male_students: Vec<&Student> = students
        .iter()
        .filter(|s| {
            s.gender == Some(StudentGender::Male) && !mapped_student_set.contains(&s.id.to_string())
        })
        .collect();
    let new_female_students: Vec<&Student> = students
        .iter()
        .filter(|s| {
            s.gender == Some(StudentGender::Female)
                && !mapped_student_set.contains(&s.id.to_string())
        })
        .collect();

    if new_male_students.is_empty() && new_female_students.is_empty() {
        return Ok(template.clone());
    }

    if new_male_students.len() > unmapped_male_learner_rows.len()
        || new_female_students.len() > unmapped_female_learner_rows.len()
    {
        let total_learner_slots = existing_mappings.len()
            + unmapped_male_learner_rows.len()
            + unmapped_female_learner_rows.len();
        return Err(AppError::InvalidInput(format!(
            "The imported SF2 workbook has {} learner rows in total, but this class now has {} learners. \
             Open the workbook in Excel, add rows for the extra learners, then import the workbook again.",
            total_learner_slots,
            students.len(),
        )));
    }

    // Assign new students to available learner rows, matching gender to block
    let mut new_mappings = Vec::new();
    let mut name_marks = Vec::new();
    let mut seen_normalized_names = existing_mappings
        .iter()
        .map(|m| m.normalized_name.clone())
        .collect::<HashSet<_>>();

    let total_new = new_male_students.len() + new_female_students.len();
    let new_male_count = new_male_students.len();
    let new_female_count = new_female_students.len();

    let mut assign_to_rows = |new_students: Vec<&Student>,
                              unmapped_rows: Vec<&Sf2WorkbookLearner>,
                              new_mappings: &mut Vec<Sf2StudentMappingRecord>,
                              name_marks: &mut Vec<Sf2CellMark>|
     -> Result<()> {
        for (student, learner_row) in new_students.iter().zip(unmapped_rows.iter()) {
            let normalized_name = normalize_learner_name(&student.name);
            let unique_name = if seen_normalized_names.contains(&normalized_name) {
                format!("{normalized_name}#{}", student.id)
            } else {
                seen_normalized_names.insert(normalized_name.clone());
                normalized_name
            };

            new_mappings.push(Sf2StudentMappingRecord {
                template_id: template.id.clone(),
                student_id: student.id.to_string(),
                workbook_name: student.name.clone(),
                normalized_name: unique_name,
                row_index: learner_row.row_index,
                gender_block: learner_row.gender_block.clone(),
            });

            // Write student name to Column C of the learner row on each monthly sheet
            for sheet in &analysis.sheets {
                if sheet.visible == 0 {
                    continue;
                }
                if month_number(&sheet.name) == 0 {
                    continue;
                }
                name_marks.push(Sf2CellMark {
                    sheet_name: sheet.name.clone(),
                    cell_address: format!("C{}", learner_row.row_index),
                    value: student.name.trim().to_string(),
                });
            }
        }
        Ok(())
    };

    assign_to_rows(
        new_male_students,
        unmapped_male_learner_rows,
        &mut new_mappings,
        &mut name_marks,
    )?;
    assign_to_rows(
        new_female_students,
        unmapped_female_learner_rows,
        &mut new_mappings,
        &mut name_marks,
    )?;

    if !name_marks.is_empty() {
        excel::write_marks(&workbook_path, &name_marks)?;
    }

    // Merge all mappings (existing + new)
    let all_mappings: Vec<Sf2StudentMappingRecord> = existing_mappings
        .iter()
        .cloned()
        .chain(new_mappings)
        .collect();

    // Re-analyze and update the template
    let refreshed_analysis = excel::analyze_workbook(&workbook_path)?;
    let date_mappings = date_mappings_from_analysis(&template.id, &refreshed_analysis);
    let synced_template = Sf2TemplateRecord {
        layout_fingerprint: layout_fingerprint(&refreshed_analysis),
        ..template.clone()
    };

    sf2_repo.update_template_with_mappings(&synced_template, &all_mappings, &date_mappings)?;

    let report_dates = sf2_date_mappings_for_report_month(&synced_template, &date_mappings)
        .iter()
        .map(|m| m.date.clone())
        .collect::<Vec<_>>();

    if let Err(error) = super::attendance_service::write_template_marks_for_mappings(
        pool,
        &synced_template,
        &report_dates,
        &all_mappings,
        &date_mappings,
    ) {
        log::warn!("failed to backfill synced imported workbook marks: {error}");
    }

    log::info!(
        "Roster sync for imported workbook '{}': added {} new student(s) ({} male, {} female)",
        template.id,
        total_new,
        new_male_count,
        new_female_count
    );

    Ok(synced_template)
}

#[cfg(test)]
#[path = "__tests__/roster_sync_tests.rs"]
mod tests;
