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

    Ok(attendance_mark_count)
}
