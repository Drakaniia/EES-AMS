use crate::domain::error::{AppError, Result};
use crate::infrastructure::database::DbPool;
use crate::sf2::attendance_marks;
use crate::sf2::excel;
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord, Sf2TemplateRecord};
use crate::sf2::repository::Sf2Repository;
use crate::sf2::sf2_metadata::sf2_date_mappings_for_report_month;
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::Emitter;

/// Emit a progress event to the frontend during SF2 workbook operations.
pub(super) fn emit_sf2_progress<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    task: &str,
    current: u32,
    total: u32,
    message: &str,
) {
    let _ = app.emit(
        "sf2-progress",
        serde_json::json!({
            "task": task,
            "current": current,
            "total": total,
            "message": message,
        }),
    );
}

pub(super) fn write_template_marks_for_days(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    days: &[String],
) -> Result<usize> {
    let workbook_path = PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let sf2_repo = Sf2Repository::new(pool.clone());
    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let date_mappings = sf2_date_mappings_for_report_month(
        template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    if date_mappings.is_empty() {
        return Ok(0);
    }

    write_template_marks_for_mappings(pool, template, days, &student_mappings, &date_mappings)
}

pub(super) fn write_template_marks_for_mappings(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    days: &[String],
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> Result<usize> {
    let workbook_path = PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let mapped_dates: HashSet<&str> = date_mappings
        .iter()
        .map(|mapping| mapping.date.as_str())
        .collect();
    let export_days = days
        .iter()
        .filter(|day| mapped_dates.contains(day.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    // Fetch date mappings from the DB and filter to only the current report
    // month. This ensures we only clear marks on the active month's sheets,
    // preserving marks on sheets for other months that have been cached from
    // previous switches. Previously we cleared ALL date mappings (all months),
    // which would blow away marks on other months when toggling attendance.
    let sf2_repo = Sf2Repository::new(pool.clone());
    let all_date_mappings = sf2_repo.date_mappings_for_template(&template.id)?;
    let clear_date_mappings: Vec<Sf2DateMappingRecord> = if all_date_mappings.is_empty() {
        date_mappings.to_vec()
    } else {
        sf2_date_mappings_for_report_month(template, &all_date_mappings)
    };

    let mut marks = attendance_marks::clear_attendance_marks_for_records(
        template,
        &clear_date_mappings,
        student_mappings,
    );
    let attendance_marks = if export_days.is_empty() || student_mappings.is_empty() {
        Vec::new()
    } else {
        attendance_marks::export_marks(
            pool,
            &template.active_class_id,
            &export_days,
            student_mappings,
            date_mappings,
        )?
    };
    let attendance_mark_count = attendance_marks.len();

    marks.extend(attendance_marks);
    excel::write_marks_force(&workbook_path, &marks)?;

    // Ensure ABSENT/PRESENT (AM/AO) formulas exist for every student and that
    // AW5 ("TOTAL NO. OF DAYS") matches the mapped day count. The bundled
    // template ships with missing formulas on some rows and stale AW5 values;
    // rewriting them here keeps the working workbook self-healing.
    if let Err(error) = write_learner_absent_present_formulas_for_mappings(
        template,
        student_mappings,
        date_mappings,
    ) {
        log::warn!("failed to write ABSENT/PRESENT formulas: {error}");
    }

    Ok(attendance_mark_count)
}

/// Write ABSENT/PRESENT (AM/AO) formulas for every mapped student plus the
/// MALE/FEMALE/Combined subtotal cells, and correct AW5 ("TOTAL NO. OF DAYS")
/// to the actual mapped day count.
///
/// Only applies to bundled templates (the app fully owns their layout). Imported
/// templates are left untouched so their original formulas are preserved.
pub(super) fn write_learner_absent_present_formulas_for_mappings(
    template: &Sf2TemplateRecord,
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> Result<usize> {
    if !crate::sf2::roster_parser::template_owns_roster(template) {
        return Ok(0);
    }
    if student_mappings.is_empty() || date_mappings.is_empty() {
        return Ok(0);
    }

    let male_count = student_mappings
        .iter()
        .filter(|m| m.gender_block.as_deref() == Some("MALE"))
        .count();
    let female_count = student_mappings
        .iter()
        .filter(|m| m.gender_block.as_deref() == Some("FEMALE"))
        .count();
    let (male_total_row, female_total_row, combined_total_row) =
        crate::sf2::roster_parser::bundled_template_total_rows(male_count, female_count);

    let sheet_names: Vec<&str> = date_mappings
        .iter()
        .map(|m| m.sheet_name.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let (formula_marks, static_marks) = attendance_marks::learner_absent_present_formula_marks(
        student_mappings,
        male_count,
        female_count,
        date_mappings.len(),
        male_total_row,
        female_total_row,
        combined_total_row,
        &sheet_names,
    );
    if formula_marks.is_empty() && static_marks.is_empty() {
        return Ok(0);
    }

    let workbook_path = PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }
    // Write formulas and the AW5 static in a single Excel session to keep the
    // repair cheap (it also runs on every attendance toggle / workbook open).
    let marks_total = formula_marks.len() + static_marks.len();
    excel::batch_operations(&workbook_path, true, move |session| {
        if !formula_marks.is_empty() {
            session.write_formulas(&formula_marks)?;
        }
        if !static_marks.is_empty() {
            session.write_marks_force(&static_marks)?;
        }
        Ok(())
    })?;
    Ok(marks_total)
}

/// Load student/date mappings from the DB and write ABSENT/PRESENT formulas.
/// Used to self-heal existing bundled workbooks when they are opened.
pub(super) fn repair_learner_absent_present_formulas(
    pool: DbPool,
    template: &Sf2TemplateRecord,
) -> Result<usize> {
    let sf2_repo = Sf2Repository::new(pool.clone());
    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let date_mappings = sf2_date_mappings_for_report_month(
        template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    write_learner_absent_present_formulas_for_mappings(template, &student_mappings, &date_mappings)
}
