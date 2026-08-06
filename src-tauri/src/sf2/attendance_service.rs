use crate::domain::error::{AppError, Result};
use crate::domain::models::AttendanceType;

use crate::infrastructure::database::{ClassRepository, DbPool, StudentRepository};

use crate::sf2::attendance_events;
#[cfg(test)]
use crate::sf2::attendance_events::parse_clock;
use rusqlite::params;
#[cfg(test)]
use crate::sf2::attendance_marks::{
    attendance_grid_rows, clear_attendance_marks_for_records, learner_absent_present_formula_marks,
    mapped_attendance_rows, summary_formula_marks, total_formula_marks,
};
use crate::sf2::calendar::attendance_changed_since;
#[cfg(test)]
use crate::sf2::logic::Sf2CellMark;
#[cfg(test)]
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord, Sf2TemplateRecord};
use crate::sf2::progress::{
    emit_sf2_progress, write_template_marks_for_days, write_template_marks_for_days_with_progress,
};
use crate::sf2::repository::Sf2Repository;
use crate::sf2::sf2_metadata::sf2_date_mappings_for_report_month;

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

        // Step 6/10: Write marks to workbook (fine-grained progress events
        // are emitted from inside the Excel write phase by the progress sink).
        emit_sf2_progress(app, "open", 6, 10, "Writing marks to workbook…");
        let _marks_written =
            write_template_marks_for_days_with_progress(app, pool.clone(), &template, &report_dates)?;

        // Step 7/10: Save workbook changes
        emit_sf2_progress(app, "open", 7, 10, "Saving workbook changes…");
        let synced_at = chrono::Utc::now().timestamp();
        sf2_repo.set_last_synced_at(&template.id, Some(synced_at))?;
    } else {
        // Workbook already reflects the latest attendance — skip Excel I/O.
        emit_sf2_progress(app, "open", 5, 10, "Attendance already up to date…");
        emit_sf2_progress(app, "open", 6, 10, "Skipping workbook rewrite…");
        emit_sf2_progress(app, "open", 7, 10, "Workbook is current…");

        // Self-heal: the bundled template ships with missing ABSENT/PRESENT
        // (AM/AO) formulas on some rows and a stale AW5 day count. Repair them
        // now so the opened workbook always shows live formulas.
        if crate::sf2::roster_parser::template_owns_roster(&template) {
            if let Err(error) = super::progress::repair_learner_absent_present_formulas(
                pool.clone(),
                &template,
            ) {
                log::warn!("failed to repair ABSENT/PRESENT formulas: {error}");
            }
        }
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
/// Only the selected student's record is changed: marking absent stores an
/// explicit 'absent' record for that one student (no other student is
/// auto-recorded), and marking present replaces it with an 'in' record.
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

    let event_type = if present {
        AttendanceType::In
    } else {
        AttendanceType::Absent
    };
    attendance_events::set_attendance_event_for_day(
        pool,
        &student_id,
        &class_id,
        date_value,
        &class.day_start,
        event_type,
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

    let event_type = if present {
        AttendanceType::In
    } else {
        AttendanceType::Absent
    };
    attendance_events::set_attendance_event_for_day(
        pool.clone(),
        &student_id,
        &class_id,
        date_value,
        &class.day_start,
        event_type,
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
/// Deletes every explicit 'absent' event for the class within the report month.
/// With the present-by-default model, removing the absent records leaves every
/// learner blank (present) - clearing all X marks without touching 'in' records.
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

    let student_repo = StudentRepository::new(pool.clone());
    let students = student_repo.list_by_class(Some(class_id))?;
    let roster_ids: HashSet<String> = students.iter().map(|s| s.id.to_string()).collect();

    let mut conn = pool.get()?;
    let transaction = conn.transaction()?;
    let mut deleted = 0usize;
    {
        let mut stmt = transaction.prepare(
            "SELECT id, student_id, class_id FROM events
             WHERE event_type = 'absent' AND timestamp >= ?1 AND timestamp < ?2",
        )?;
        for mapping in &date_mappings {
            let Ok(date) = parse_date(&mapping.date) else {
                continue;
            };
            let (start_ts, end_ts) =
                attendance_events::local_day_bounds_timestamps_for_date(date)?;
            let rows = stmt.query_map(params![start_ts, end_ts], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?;
            for row in rows {
                let (id, student_id, event_class_id) = row?;
                let belongs = event_class_id.as_deref() == Some(class_id)
                    || roster_ids.contains(&student_id);
                if !belongs {
                    continue;
                }
                transaction.execute("DELETE FROM events WHERE id = ?1", params![id])?;
                deleted += 1;
            }
        }
    }
    transaction.commit()?;

    // Reset last_synced_at so the next SF2 open detects the change and
    // rewrites the cleared marks.
    sf2_repo.set_last_synced_at(&template.id, None)?;
    Ok(deleted)
}

#[cfg(test)]
#[path = "__tests__/attendance_service_tests.rs"]
mod tests;
