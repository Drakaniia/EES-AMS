use crate::domain::error::{AppError, Result};

use crate::infrastructure::database::{
    record_audit_event, AuditEventInput, ClassRepository, DbPool, EventRepository,
    StudentRepository,
};
use crate::sf2::attendance::present_events_for_day;
use crate::sf2::calendar::{attendance_changed_since, sf2_date_mappings_for_report_month};
use crate::sf2::excel;
use crate::sf2::logic::{
    attendance_marks_for_closed_day, day_has_attendance_taken, Sf2CellMark, Sf2StudentMapping,
};
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord, Sf2TemplateRecord};
use crate::sf2::repository::Sf2Repository;

use chrono::{Local, NaiveDate, Utc};
use rusqlite::params;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tauri::Emitter;

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
        sf2_repo.set_last_synced_at(&template.id, synced_at)?;
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
        let today = Local::now().date_naive();
        if date_value <= today {
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
                    if let Err(e) = set_attendance_event_for_day(
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
    }

    set_attendance_event_for_day(
        pool,
        &student_id,
        &class_id,
        date_value,
        &class.day_start,
        present,
    )
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

    set_attendance_event_for_day(
        pool.clone(),
        &student_id,
        &class_id,
        date_value,
        &class.day_start,
        present,
    )?;
    let template =
        super::excel_service::refresh_template_calendar_from_saved_month(pool.clone(), &template)?;
    write_template_marks_for_days(pool.clone(), &template, &report_dates)?;

    super::excel_service::export_preview(pool, Some(class_id))
}

/// Mark ALL absent students as present for the current report month of a class.
///
/// For each date in the report month where attendance was taken (at least one
/// "in" event exists), create "in" events for ALL mapped students who were
/// absent. This effectively "clears" all X marks, resetting to all Present.
///
/// Open days (no attendance taken at all) are left as-is.
pub fn set_all_students_present(pool: DbPool, class_id: &str) -> Result<usize> {
    use crate::sf2::calendar::{parse_date, sf2_date_mappings_for_report_month};

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

    let today = Local::now().date_naive();
    let mut created_count = 0usize;

    for date_str in &report_dates {
        let Ok(date) = parse_date(date_str) else {
            continue;
        };

        // Skip future dates
        if date > today {
            continue;
        }

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
            if let Err(e) = set_attendance_event_for_day(
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
    // For new templates not yet persisted, fall back to the caller-supplied set.
    let sf2_repo = Sf2Repository::new(pool.clone());
    let all_date_mappings = sf2_repo.date_mappings_for_template(&template.id)?;
    let clear_date_mappings: Vec<Sf2DateMappingRecord> = if all_date_mappings.is_empty() {
        date_mappings.to_vec()
    } else {
        sf2_date_mappings_for_report_month(template, &all_date_mappings)
    };

    let mut marks =
        clear_attendance_marks_for_records(template, &clear_date_mappings, student_mappings);
    let attendance_marks = if export_days.is_empty() || student_mappings.is_empty() {
        Vec::new()
    } else {
        export_marks(
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

fn export_marks(
    pool: DbPool,
    class_id: &str,
    closed_days: &[String],
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> Result<Vec<Sf2CellMark>> {
    let today = Local::now().date_naive();
    let past_days: Vec<&String> = closed_days
        .iter()
        .filter(|day| {
            NaiveDate::parse_from_str(day, "%Y-%m-%d")
                .map(|d| d <= today)
                .unwrap_or(true)
        })
        .collect();

    let date_by_day: HashMap<&str, &Sf2DateMappingRecord> = date_mappings
        .iter()
        .map(|mapping| (mapping.date.as_str(), mapping))
        .collect();
    let student_repo = StudentRepository::new(pool.clone());
    let event_repo = EventRepository::new(pool);
    let students = student_repo.list_by_class(Some(class_id))?;
    let events = event_repo.list()?;

    let mut marks = Vec::new();
    for day in past_days {
        let Some(date_mapping) = date_by_day.get(day.as_str()) else {
            continue;
        };

        let day_students: Vec<Sf2StudentMapping> = student_mappings
            .iter()
            .map(|student| Sf2StudentMapping {
                student_id: student.student_id.clone(),
                sheet_name: date_mapping.sheet_name.clone(),
                row_index: student.row_index,
            })
            .collect();
        let present_events = present_events_for_day(&events, &students, class_id, day);

        // Skip days where NO attendance was taken for ANY student.
        // Per spec: a day with zero "in" events is an "Open" day, not "Absent".
        if !day_has_attendance_taken(&present_events) {
            continue;
        }

        marks.extend(attendance_marks_for_closed_day(
            &day_students,
            &present_events,
            &date_mapping.column_letter,
        ));
    }

    Ok(marks)
}

fn clear_attendance_marks_for_records(
    template: &Sf2TemplateRecord,
    date_mappings: &[Sf2DateMappingRecord],
    student_mappings: &[Sf2StudentMappingRecord],
) -> Vec<Sf2CellMark> {
    let row_indices = if super::calendar_service::template_owns_roster(template) {
        let row_slots = super::calendar_service::template_roster_slots();
        attendance_grid_rows(
            &row_slots,
            student_mappings.iter().map(|mapping| mapping.row_index),
        )
    } else {
        mapped_attendance_rows(student_mappings.iter().map(|mapping| mapping.row_index))
    };

    // Get the unique set of visible sheet names from the date mappings.
    let sheet_names: Vec<&str> = date_mappings
        .iter()
        .map(|m| m.sheet_name.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if sheet_names.is_empty() {
        return Vec::new();
    }

    // Generate column letters for ALL standard DepEd SF2 weekday columns (F through AL).
    // This ensures stale marks are cleared even from weekday columns that have no
    // valid date in the report month (e.g. Monday/Tuesday in the first week when
    // the month starts mid-week).
    let all_column_letters: Vec<String> = (6..=38).map(column_number_to_letter).collect();

    let mut marks =
        Vec::with_capacity(sheet_names.len() * all_column_letters.len() * row_indices.len());
    for sheet_name in &sheet_names {
        for col_letter in &all_column_letters {
            for row_index in &row_indices {
                marks.push(Sf2CellMark {
                    sheet_name: sheet_name.to_string(),
                    cell_address: format!("{col_letter}{row_index}"),
                    value: String::new(),
                });
            }
        }
    }
    marks
}

/// Convert a 1-based column index to an Excel column letter (e.g., 1 -> A, 26 -> Z, 27 -> AA).
fn column_number_to_letter(mut column: i32) -> String {
    let mut letter = String::new();
    while column > 0 {
        let modulo = (column - 1) % 26;
        letter.insert(0, (b'A' + modulo as u8) as char);
        column = (column - modulo) / 26;
    }
    letter
}

fn set_attendance_event_for_day(
    pool: DbPool,
    student_id: &str,
    class_id: &str,
    date: NaiveDate,
    day_start: &str,
    present: bool,
) -> Result<()> {
    let (day_start_timestamp, day_end_timestamp) = local_day_bounds_timestamps_for_date(date)?;
    let mut conn = pool.get()?;
    let transaction = conn.transaction()?;
    let deleted_events: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM events
             WHERE student_id = ?1
             AND event_type = 'in'
             AND timestamp >= ?2
             AND timestamp < ?3
             AND (class_id IS NULL OR class_id = ?4)",
            params![student_id, day_start_timestamp, day_end_timestamp, class_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    transaction.execute(
        "DELETE FROM events
         WHERE student_id = ?1
         AND event_type = 'in'
         AND timestamp >= ?2
         AND timestamp < ?3
         AND (class_id IS NULL OR class_id = ?4)",
        params![student_id, day_start_timestamp, day_end_timestamp, class_id],
    )?;

    let mut created_event_id: Option<String> = None;
    if present {
        let attendance_timestamp = attendance_timestamp_for_date(date, day_start)?;
        let session_key = format!("{date}|{class_id}|day");
        let event_id = uuid::Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
             VALUES (?1, ?2, ?3, 'in', ?4, ?5, ?6, ?7, NULL)",
            params![
                event_id.as_str(),
                student_id,
                class_id,
                attendance_timestamp,
                "SF2 preview correction",
                session_key,
                "SF2 preview correction",
            ],
        )?;
        created_event_id = Some(event_id);
    }

    let metadata_json = serde_json::to_string(&serde_json::json!({
        "studentId": student_id,
        "classId": class_id,
        "date": date.to_string(),
        "present": present,
        "deletedEvents": deleted_events,
        "createdEventId": created_event_id.as_deref(),
    }))
    .map_err(|error| AppError::Internal(format!("failed to serialize audit metadata: {error}")))?;
    let summary = format!(
        "Set SF2 preview attendance for student {student_id} on {date} to {}",
        if present { "present" } else { "absent" }
    );
    record_audit_event(
        &transaction,
        AuditEventInput {
            entity_type: "attendance_event",
            entity_id: created_event_id.as_deref(),
            action: if present { "create" } else { "delete" },
            summary: &summary,
            before_json: None,
            after_json: None,
            metadata_json: Some(metadata_json),
        },
    )?;

    transaction.commit()?;
    Ok(())
}

fn local_day_bounds_timestamps_for_date(date: NaiveDate) -> Result<(i64, i64)> {
    let next_day = date.succ_opt().ok_or_else(|| {
        AppError::Internal("failed to calculate local attendance date".to_string())
    })?;
    Ok((
        local_timestamp(date, 0, 0)?,
        local_timestamp(next_day, 0, 0)?,
    ))
}

fn attendance_timestamp_for_date(date: NaiveDate, day_start: &str) -> Result<i64> {
    let (hour, minute) = parse_clock(day_start).unwrap_or((8, 0));
    local_timestamp(date, hour, minute)
}

fn local_timestamp(date: NaiveDate, hour: u32, minute: u32) -> Result<i64> {
    let local_time = date
        .and_hms_opt(hour, minute, 0)
        .and_then(|time| time.and_local_timezone(Local).earliest())
        .ok_or_else(|| {
            AppError::Internal(format!(
                "failed to calculate local timestamp for {}",
                date.format("%Y-%m-%d")
            ))
        })?;
    Ok(local_time.with_timezone(&Utc).timestamp())
}

fn parse_clock(value: &str) -> Option<(u32, u32)> {
    let (hour, minute) = value.trim().split_once(':')?;
    let hour = hour.parse::<u32>().ok()?;
    let minute = minute.parse::<u32>().ok()?;
    if hour < 24 && minute < 60 {
        Some((hour, minute))
    } else {
        None
    }
}

/// Generate Excel formulas and static values for the SF2 summary section (rows 53–71).
///
/// Returns `(formula_marks, static_marks)`:
/// - `formula_marks` — Excel formulas for rows 59 (Registered Learners), 61 (Percentage of
///   Enrolment), 63 (Average Daily Attendance), and 65 (Percentage of Attendance).
///   These are written with `set_sf2_formula`.
/// - `static_marks` — Static numeric values for row 53 (Enrolment).
///   These are written with `set_sf2_mark_force`.
///
/// Marks are generated per unique sheet name found in `date_mappings`, so all visible
/// monthly sheets get the same summary formulas.
pub(super) fn summary_formula_marks(
    male_count: usize,
    female_count: usize,
    total_students: usize,
    male_total_row: u32,
    female_total_row: u32,
    combined_total_row: u32,
    date_mappings: &[Sf2DateMappingRecord],
) -> (Vec<Sf2CellMark>, Vec<Sf2CellMark>) {
    // Extract unique sheet names from date_mappings
    let sheet_names: Vec<&str> = date_mappings
        .iter()
        .map(|m| m.sheet_name.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if sheet_names.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut formula_marks = Vec::new();
    let mut static_marks = Vec::new();
    let columns = ["AR", "AS", "AT"];

    for sheet_name in &sheet_names {
        let sn = sheet_name.to_string();

        // ── Static marks (row 53: Enrolment) ─────────────────────────
        static_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AR53".to_string(),
            value: male_count.to_string(),
        });
        static_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AS53".to_string(),
            value: female_count.to_string(),
        });
        static_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AT53".to_string(),
            value: total_students.to_string(),
        });

        // ── Row 59: Registered Learners ─────────────────────────────
        // Formula: =col53+col55-col67-col69+col71
        for col in &columns {
            formula_marks.push(Sf2CellMark {
                sheet_name: sn.clone(),
                cell_address: format!("{col}59"),
                value: format!("={col}53+{col}55-{col}67-{col}69+{col}71"),
            });
        }

        // ── Row 61: Percentage of Enrolment ──────────────────────────
        // Formula: =IF(col53>0, col59/col53*100, 0)
        for col in &columns {
            formula_marks.push(Sf2CellMark {
                sheet_name: sn.clone(),
                cell_address: format!("{col}61"),
                value: format!("=IF({col}53>0,{col}59/{col}53*100,0)"),
            });
        }

        // ── Row 63: Average Daily Attendance ─────────────────────────
        // Male ADA references male_total_row, Female ADA female_total_row, Total ADA combined_total_row
        // Formula: =IFERROR(AVERAGE(F{total_row}:AL{total_row}),0)
        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AR63".to_string(),
            value: format!("=IFERROR(AVERAGE(F{male_total_row}:AL{male_total_row}),0)"),
        });
        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AS63".to_string(),
            value: format!("=IFERROR(AVERAGE(F{female_total_row}:AL{female_total_row}),0)"),
        });
        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AT63".to_string(),
            value: format!("=IFERROR(AVERAGE(F{combined_total_row}:AL{combined_total_row}),0)"),
        });

        // ── Row 65: Percentage of Attendance ─────────────────────────
        // Formula: =IF(col59>0, col63/col59*100, 0)
        for col in &columns {
            formula_marks.push(Sf2CellMark {
                sheet_name: sn.clone(),
                cell_address: format!("{col}65"),
                value: format!("=IF({col}59>0,{col}63/{col}59*100,0)"),
            });
        }
    }

    (formula_marks, static_marks)
}

/// Generate Excel formulas for the MALE TOTAL, FEMALE TOTAL, and Combined TOTAL rows.
///
/// Writes formulas that dynamically calculate present student count per day:
///   MALE TOTAL:     ={male_count}-COUNTIF({col}{first_male_row}:{col}{last_male_row},"X")
///   FEMALE TOTAL:   ={female_count}-COUNTIF({col}{first_female_row}:{col}{last_female_row},"X")
///   Combined TOTAL: ={col}{male_total}+{col}{female_total}
///
/// The formulas use "X" marks (absent) to compute PRESENT count per day.
/// Empty template rows (no student assigned) never have X marks, so they don't affect the count.
pub(super) fn total_formula_marks(
    male_count: usize,
    female_count: usize,
    male_total_row: u32,
    female_total_row: u32,
    combined_total_row: u32,
    date_mappings: &[Sf2DateMappingRecord],
) -> Vec<Sf2CellMark> {
    // Male range: first male slot (8) to last male slot before TOTAL
    let first_male_row = 8u32;
    let last_male_row = male_total_row.saturating_sub(1);
    // Female range: first female slot (after MALE TOTAL) to last female slot before FEMALE TOTAL
    let first_female_row = male_total_row + 1;
    let last_female_row = female_total_row.saturating_sub(1);

    let mut formula_marks = Vec::new();
    for date_mapping in date_mappings {
        if date_mapping.date.trim().is_empty() {
            continue;
        }
        let col = &date_mapping.column_letter;

        // MALE TOTAL: ={male_count}-COUNTIF({col}{first}:{col}{last},"X")
        formula_marks.push(Sf2CellMark {
            sheet_name: date_mapping.sheet_name.clone(),
            cell_address: format!("{col}{male_total_row}"),
            value: format!(
                "={}-COUNTIF({}{}:{}{},\"X\")",
                male_count, col, first_male_row, col, last_male_row,
            ),
        });

        // FEMALE TOTAL: ={female_count}-COUNTIF({col}{first}:{col}{last},"X")
        formula_marks.push(Sf2CellMark {
            sheet_name: date_mapping.sheet_name.clone(),
            cell_address: format!("{col}{female_total_row}"),
            value: format!(
                "={}-COUNTIF({}{}:{}{},\"X\")",
                female_count, col, first_female_row, col, last_female_row,
            ),
        });

        // Combined TOTAL: ={col}{male_total}+{col}{female_total}
        formula_marks.push(Sf2CellMark {
            sheet_name: date_mapping.sheet_name.clone(),
            cell_address: format!("{col}{combined_total_row}"),
            value: format!("={}{}+{}{}", col, male_total_row, col, female_total_row),
        });
    }
    formula_marks
}

/// Generate empty cell marks for all TOTAL PER DAY formula cells across ALL
/// weekday columns (6–38). This clears stale template values (default `0` or
/// leftover formulas) from columns that have no corresponding date in the
/// report month — e.g. columns for Monday/Tuesday in the first week when
/// the month starts mid-week.
///
/// Must be called with `write_marks_force` *before* `write_formulas` so that
/// columns WITHOUT a valid date end up clean/empty rather than showing a
/// stale value inherited from the bundled template.
pub(super) fn clear_total_cell_marks(
    male_total_row: u32,
    female_total_row: u32,
    combined_total_row: u32,
    date_mappings: &[Sf2DateMappingRecord],
) -> Vec<Sf2CellMark> {
    let sheet_names: Vec<&str> = date_mappings
        .iter()
        .map(|m| m.sheet_name.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if sheet_names.is_empty() {
        return Vec::new();
    }

    let mut marks = Vec::with_capacity(sheet_names.len() * 33 * 3);
    for sheet_name in sheet_names {
        for col in 6..=38 {
            let col_letter = column_number_to_letter(col);
            marks.push(Sf2CellMark {
                sheet_name: sheet_name.to_string(),
                cell_address: format!("{col_letter}{male_total_row}"),
                value: String::new(),
            });
            marks.push(Sf2CellMark {
                sheet_name: sheet_name.to_string(),
                cell_address: format!("{col_letter}{female_total_row}"),
                value: String::new(),
            });
            marks.push(Sf2CellMark {
                sheet_name: sheet_name.to_string(),
                cell_address: format!("{col_letter}{combined_total_row}"),
                value: String::new(),
            });
        }
    }
    marks
}

fn attendance_grid_rows<I>(
    row_slots: &[super::calendar_service::TemplateRosterSlot],
    extra_rows: I,
) -> Vec<u32>
where
    I: IntoIterator<Item = u32>,
{
    let mut rows = row_slots
        .iter()
        .map(|slot| slot.row_index)
        .collect::<Vec<_>>();
    rows.extend(extra_rows);
    rows.sort_unstable();
    rows.dedup();
    rows
}

fn mapped_attendance_rows<I>(rows: I) -> Vec<u32>
where
    I: IntoIterator<Item = u32>,
{
    let mut rows = rows
        .into_iter()
        .filter(|row_index| *row_index > 0)
        .collect::<Vec<_>>();
    rows.sort_unstable();
    rows.dedup();
    rows
}

#[cfg(test)]
#[path = "__tests__/attendance_service_tests.rs"]
mod tests;
