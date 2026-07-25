use crate::domain::error::{AppError, Result};

use crate::infrastructure::database::{ClassRepository, DbPool, EventRepository, StudentRepository};
use crate::sf2::attendance::present_events_for_day;
use crate::sf2::attendance_events;
use crate::sf2::calendar::attendance_changed_since;
use crate::sf2::sf2_metadata::sf2_date_mappings_for_report_month;
use crate::sf2::logic::day_has_attendance_taken;
use crate::sf2::progress::{emit_sf2_progress, write_template_marks_for_days};
use crate::sf2::repository::Sf2Repository;

use std::collections::HashSet;

/// Sync latest attendance events to the SF2 Excel working copy for a given class.
/// This ensures the Excel file's attendance marks reflect the current attendance state
/// after a student is marked present or absent from the attendance page.
pub fn sync_attendance_to_sf2_workbook(pool: DbPool, class_id: &str) -> Result<()> {
    let sf2_repo = Sf2Repository::new(pool.clone());
    let Some(template) = sf2_repo.latest_template_for_class(class_id)? else {
        return Ok(());
    };

    let date_mappings = sf2_date_mappings_for_report_month(
        &template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    if date_mappings.is_empty() {
        return Ok(());
    }

    let report_dates = date_mappings
        .iter()
        .map(|m| m.date.clone())
        .collect::<Vec<_>>();
    write_template_marks_for_days(pool, &template, &report_dates)?;
    Ok(())
}

/// Sync attendance and open the SF2 workbook, emitting real progress events
/// so the frontend can display a determinate progress bar with friendly messages.
/// Returns the path to the opened workbook on success.
pub fn sync_and_open_sf2_workbook<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    pool: DbPool,
    class_id: &str,
) -> Result<String> {
    // Step 1/10: Load template
    emit_sf2_progress(app, "open", 1, 10, "Loading workbook details…");
    let sf2_repo = Sf2Repository::new(pool.clone());
    let template = sf2_repo
        .latest_template_for_class(class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("No SF2 template imported for this class".to_string())
        })?;

    // Step 2/10: Load student mappings
    emit_sf2_progress(app, "open", 2, 10, "Reading student data…");
    let _student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;

    // Step 3/10: Check date mappings
    emit_sf2_progress(app, "open", 3, 10, "Checking date mappings…");
    let date_mappings = sf2_date_mappings_for_report_month(
        &template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    if date_mappings.is_empty() {
        return Err(AppError::InvalidInput(
            "No attendance dates are mapped to this SF2 report month.".to_string(),
        ));
    }
    let report_dates = date_mappings
        .iter()
        .map(|m| m.date.clone())
        .collect::<Vec<_>>();

    // Decide whether the Excel workbook actually needs to be rewritten.
    // If no attendance event was recorded after the last successful sync,
    // the workbook is already current and we can skip the slow Excel COM
    // automation entirely — opening is then instant.
    let latest_event_at = sf2_repo.latest_event_timestamp(class_id)?;
    let marks_changed = attendance_changed_since(template.last_synced_at, latest_event_at);

    // Step 4/10: Clear previous marks (only when rewriting)
    emit_sf2_progress(app, "open", 4, 10, "Clearing previous marks…");

    if marks_changed {
        // Step 5/10: Compute attendance marks
        emit_sf2_progress(app, "open", 5, 10, "Computing attendance marks…");

        // Step 6/10: Write marks to workbook
        emit_sf2_progress(app, "open", 6, 10, "Writing marks to workbook…");
        let _marks_written = write_template_marks_for_days(pool.clone(), &template, &report_dates)?;

        // Step 7/10: Save workbook changes
        emit_sf2_progress(app, "open", 7, 10, "Saving workbook changes…");
        let synced_at = chrono::Utc::now().timestamp();
        sf2_repo.set_last_synced_at(&template.id, Some(synced_at))?;
    } else {
        // Workbook already reflects the latest attendance — skip Excel I/O.
        emit_sf2_progress(app, "open", 5, 10, "Attendance already up to date…");
        emit_sf2_progress(app, "open", 6, 10, "Skipping workbook rewrite…");
        emit_sf2_progress(app, "open", 7, 10, "Workbook is current…");
    }

    // Step 8/10: Prepare to open
    emit_sf2_progress(app, "open", 8, 10, "Preparing to open…");
    let workbook_path = std::path::PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    // Step 9/10: Open in Excel
    emit_sf2_progress(app, "open", 9, 10, "Opening in Microsoft Excel…");
    crate::sf2::workbook_files::open_path_in_default_app(&workbook_path)?;

    // Step 10/10: Done
    emit_sf2_progress(app, "open", 10, 10, "Done!");

    Ok(workbook_path.to_string_lossy().to_string())
}

/// Fast path: only persists the attendance event to the database without
/// touching the Excel workbook or rebuilding the full preview.
/// Used by the pre-export review toggle so the user doesn't wait on Excel I/O.
///
/// When marking a student as absent on a day with NO existing "in" events
/// (an "Open" day), this also creates "in" events for all other mapped
/// students. This establishes the day as having attendance taken, so this
/// student correctly appears as Absent (X) in the preview instead of Present.
pub fn set_preview_attendance_lightweight(
    pool: DbPool,
    class_id: String,
    student_id: String,
    date: String,
    present: bool,
) -> Result<()> {
    use crate::sf2::calendar::parse_date;
    let date_value = parse_date(&date)?;
    let sf2_repo = Sf2Repository::new(pool.clone());
    let template = sf2_repo
        .latest_template_for_class(&class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("No SF2 template imported for this class".to_string())
        })?;
    // Removed date_mapping validation: unmapped dates should also be
    // clickable/toggleable. The toggle is DB-only — it does not write to Excel.
    // Events for unmapped dates are handled correctly during export (filtered out).

    let class = ClassRepository::new(pool.clone())
        .get(&class_id)?
        .ok_or_else(|| AppError::InvalidInput("Selected class was not found".to_string()))?;
    let students = StudentRepository::new(pool.clone()).list_by_class(Some(&class_id))?;
    let _student = students
        .iter()
        .find(|student| student.id.to_string() == student_id)
        .ok_or_else(|| AppError::InvalidInput("Selected student was not found".to_string()))?;

    // When marking a student as absent on a day where NO attendance was taken
    // (an "Open" day), we need to create "in" events for all OTHER mapped
    // students. This establishes the day as having attendance taken, and leaves
    // this specific student without an "in" event → they show as Absent (X).
    if !present {
        let event_repo = EventRepository::new(pool.clone());
        let all_events = event_repo.list()?;
        let present_events = present_events_for_day(&all_events, &students, &class_id, &date);
        if !day_has_attendance_taken(&present_events) {
            // This is an Open day → mark all other mapped students as present
            let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
            for mapping in &student_mappings {
                if mapping.student_id == student_id {
                    // Skip — this student should remain absent
                    continue;
                }
                if present_events
                    .iter()
                    .any(|e| e.student_id == mapping.student_id)
                {
                    // Already has an "in" event — skip
                    continue;
                }
                if let Err(e) = attendance_events::set_attendance_event_for_day(
                    pool.clone(),
                    &mapping.student_id,
                    &class_id,
                    date_value,
                    &class.day_start,
                    true,
                ) {
                    log::warn!(
                        "Failed to mark student {} present on {}: {}",
                        mapping.student_id,
                        date,
                        e
                    );
                }
            }
        }
    }

    attendance_events::set_attendance_event_for_day(
        pool,
        &student_id,
        &class_id,
        date_value,
        &class.day_start,
        present,
    )?;
    // Reset last_synced_at so the next SF2 open detects the change and
    // rewrites marks. Without this, the sync optimization in
    // sync_and_open_sf2_workbook would skip rewriting because all events
    // created here have past-date timestamps, making the MAX(timestamp)
    // comparison against last_synced_at return false.
    sf2_repo.set_last_synced_at(&template.id, None)?;
    Ok(())
}

pub fn set_preview_attendance(
    pool: DbPool,
    class_id: String,
    student_id: String,
    date: String,
    present: bool,
) -> Result<crate::sf2::models::Sf2ExportPreview> {
    use crate::sf2::calendar::parse_date;
    let date_value = parse_date(&date)?;
    let sf2_repo = Sf2Repository::new(pool.clone());
    let template = sf2_repo
        .latest_template_for_class(&class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("No SF2 template imported for this class".to_string())
        })?;
    let date_mappings = sf2_date_mappings_for_report_month(
        &template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    if !date_mappings
        .iter()
        .any(|mapping| mapping.date.as_str() == date.as_str())
    {
        return Err(AppError::InvalidInput(format!(
            "{date} is not mapped to an SF2 date column"
        )));
    }

    let report_dates = date_mappings
        .iter()
        .map(|m| m.date.clone())
        .collect::<Vec<_>>();

    let class = ClassRepository::new(pool.clone())
        .get(&class_id)?
        .ok_or_else(|| AppError::InvalidInput("Selected class was not found".to_string()))?;
    let students = StudentRepository::new(pool.clone()).list_by_class(Some(&class_id))?;
    let _student = students
        .iter()
        .find(|student| student.id.to_string() == student_id)
        .ok_or_else(|| AppError::InvalidInput("Selected student was not found".to_string()))?;

    attendance_events::set_attendance_event_for_day(
        pool.clone(),
        &student_id,
        &class_id,
        date_value,
        &class.day_start,
        present,
    )?;
    let template = super::excel_service::refresh_template_calendar_from_saved_month(
        pool.clone(),
        &template,
        false,
    )?;
    write_template_marks_for_days(pool.clone(), &template, &report_dates)?;

    super::excel_preview::export_preview(pool, Some(class_id))
}

/// Mark ALL absent students as present for the current report month of a class.
///
/// For each date in the report month where attendance was taken (at least one
/// "in" event exists), create "in" events for ALL mapped students who were
/// absent. This effectively "clears" all X marks, resetting to all Present.
///
/// Open days (no attendance taken at all) are left as-is.
pub fn set_all_students_present(pool: DbPool, class_id: &str) -> Result<usize> {
    use crate::sf2::calendar::parse_date;
    use crate::sf2::sf2_metadata::sf2_date_mappings_for_report_month;

    let sf2_repo = Sf2Repository::new(pool.clone());
    let template = sf2_repo
        .latest_template_for_class(class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("No SF2 template imported for this class".to_string())
        })?;

    let date_mappings = sf2_date_mappings_for_report_month(
        &template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    if date_mappings.is_empty() {
        return Ok(0);
    }

    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    if student_mappings.is_empty() {
        return Ok(0);
    }

    let class = ClassRepository::new(pool.clone())
        .get(class_id)?
        .ok_or_else(|| AppError::InvalidInput("Selected class was not found".to_string()))?;

    let student_repo = StudentRepository::new(pool.clone());
    let event_repo = EventRepository::new(pool.clone());
    let students = student_repo.list_by_class(Some(class_id))?;
    let events = event_repo.list()?;

    let report_dates: Vec<String> = date_mappings.iter().map(|m| m.date.clone()).collect();

    let mut created_count = 0usize;

    for date_str in &report_dates {
        let Ok(date) = parse_date(date_str) else {
            continue;
        };

        // Check if attendance was taken on this day
        let present_events = present_events_for_day(&events, &students, class_id, date_str);
        if !day_has_attendance_taken(&present_events) {
            // No attendance taken on this day — skip (Open day)
            continue;
        }

        // Find students who are absent (no "in" event) on this day
        let present_ids: HashSet<&str> = present_events
            .iter()
            .map(|e| e.student_id.as_str())
            .collect();

        for mapping in &student_mappings {
            if present_ids.contains(mapping.student_id.as_str()) {
                // Already present
                continue;
            }

            // Create "in" event for this absent student
            if let Err(e) = attendance_events::set_attendance_event_for_day(
                pool.clone(),
                &mapping.student_id,
                class_id,
                date,
                &class.day_start,
                true,
            ) {
                log::warn!(
                    "Failed to mark student {} present on {}: {}",
                    mapping.student_id,
                    date_str,
                    e
                );
                continue;
            }
            created_count += 1;
        }
    }

    Ok(created_count)
}

#[cfg(test)]
#[path = "__tests__/attendance_service_tests.rs"]
mod tests;
