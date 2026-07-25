use crate::domain::error::{AppError, Result};
use crate::domain::models::{Student, StudentGender};
use crate::infrastructure::database::DbPool;
use crate::sf2::attendance_marks::clear_total_cell_marks;
use crate::sf2::sf2_metadata::{date_mappings_from_analysis, sf2_date_mappings_for_report_month};
use crate::sf2::excel;
use crate::sf2::excel_com::workbook_utils::month_number;
use crate::sf2::logic::{normalize_learner_name, Sf2CellMark};
use crate::sf2::models::{
    Sf2StudentMappingRecord, Sf2TemplateRecord, Sf2WorkbookLearner,
};
use crate::sf2::repository::Sf2Repository;
use crate::sf2::roster::{
    bundled_template_total_rows, clear_unused_learner_marks, roster_name_marks,
    student_mappings_from_roster_assignments, template_roster_assignments,
};
use crate::sf2::workbook_files::layout_fingerprint;
use crate::sf2::roster_sync::roster_sync_formula_marks;
use std::collections::HashSet;
use std::path::PathBuf;

/// Sync a bundled template's roster: re-assigns ALL students to dynamic row slots,
/// expanding the workbook if the student count exceeds current capacity.
pub(super) fn sync_bundled_template_roster(
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

    let (male_total_row, female_total_row, combined_total_row) =
        bundled_template_total_rows(male_count, female_count);

    let current_male_capacity = existing_male_mapped.max(21) as u32;
    let current_extra_male = current_male_capacity.saturating_sub(21);
    let current_female_capacity = existing_female_mapped.max(19) as u32;
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

    if let Err(error) = super::super::progress::write_template_marks_for_mappings(
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

/// Sync an imported workbook's roster: map new DB students to available empty learner rows.
pub(super) fn sync_imported_workbook_roster(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    students: &[Student],
    existing_mappings: &[Sf2StudentMappingRecord],
) -> Result<Sf2TemplateRecord> {
    let workbook_path = PathBuf::from(&template.source_path);
    let sf2_repo = Sf2Repository::new(pool.clone());
    let analysis = excel::analyze_workbook(&workbook_path)?;

    let mapped_row_set: HashSet<u32> =
        existing_mappings.iter().map(|m| m.row_index).collect();

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

    let mapped_student_set: HashSet<String> = existing_mappings
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
            total_learner_slots, students.len(),
        )));
    }

    let mut new_mappings = Vec::new();
    let mut name_marks = Vec::new();
    let mut seen_normalized_names = existing_mappings
        .iter()
        .map(|m| m.normalized_name.clone())
        .collect::<HashSet<_>>();

    let total_new = new_male_students.len() + new_female_students.len();
    let new_male_count = new_male_students.len();
    let new_female_count = new_female_students.len();

    assign_students_to_rows(
        &analysis, &template, &mut seen_normalized_names,
        new_male_students, unmapped_male_learner_rows,
        &mut new_mappings, &mut name_marks,
    )?;
    assign_students_to_rows(
        &analysis, &template, &mut seen_normalized_names,
        new_female_students, unmapped_female_learner_rows,
        &mut new_mappings, &mut name_marks,
    )?;

    if !name_marks.is_empty() {
        excel::write_marks(&workbook_path, &name_marks)?;
    }

    let all_mappings: Vec<Sf2StudentMappingRecord> = existing_mappings
        .iter()
        .cloned()
        .chain(new_mappings)
        .collect();

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

    if let Err(error) = super::super::progress::write_template_marks_for_mappings(
        pool, &synced_template, &report_dates, &all_mappings, &date_mappings,
    ) {
        log::warn!("failed to backfill synced imported workbook marks: {error}");
    }

    log::info!(
        "Roster sync for imported workbook '{}': added {} new student(s) ({} male, {} female)",
        template.id, total_new, new_male_count, new_female_count
    );

    Ok(synced_template)
}

/// Assign new students to unmapped learner rows, creating mappings and name marks.
fn assign_students_to_rows(
    analysis: &crate::sf2::models::Sf2WorkbookAnalysis,
    template: &Sf2TemplateRecord,
    seen_normalized_names: &mut HashSet<String>,
    new_students: Vec<&Student>,
    unmapped_rows: Vec<&Sf2WorkbookLearner>,
    new_mappings: &mut Vec<Sf2StudentMappingRecord>,
    name_marks: &mut Vec<Sf2CellMark>,
) -> Result<()> {
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

        for sheet in &analysis.sheets {
            if sheet.visible == 0 { continue; }
            if month_number(&sheet.name) == 0 { continue; }
            name_marks.push(Sf2CellMark {
                sheet_name: sheet.name.clone(),
                cell_address: format!("C{}", learner_row.row_index),
                value: student.name.trim().to_string(),
            });
        }
    }
    Ok(())
}
