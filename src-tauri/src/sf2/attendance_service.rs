use crate::domain::error::{AppError, Result};

use crate::infrastructure::database::{
    record_audit_event, AuditEventInput, ClassRepository, DbPool, EventRepository,
    StudentRepository,
};
use crate::sf2::attendance::present_events_for_day;
use crate::sf2::calendar::sf2_date_mappings_for_report_month;
use crate::sf2::excel;
use crate::sf2::logic::{attendance_marks_for_closed_day, Sf2CellMark, Sf2StudentMapping};
use crate::sf2::models::{
    Sf2DateMappingRecord, Sf2StudentMappingRecord, Sf2TemplateRecord,
};
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
pub fn sync_attendance_to_sf2_workbook(
    pool: DbPool,
    class_id: &str,
) -> Result<()> {
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

/// Fast path: only persists the attendance event to the database without
/// touching the Excel workbook or rebuilding the full preview.
/// Used by the pre-export review toggle so the user doesn't wait on Excel I/O.
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

    let report_dates = date_mappings.iter().map(|m| m.date.clone()).collect::<Vec<_>>();

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
    let template = super::excel_service::refresh_template_calendar_from_saved_month(pool.clone(), &template)?;
    write_template_marks_for_days(pool.clone(), &template, &report_dates)?;

    super::excel_service::export_preview(pool, Some(class_id))
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

    let mut marks = clear_attendance_marks_for_records(template, date_mappings, student_mappings);
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
    excel::write_marks(&workbook_path, &marks)?;
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

    let mut marks = Vec::with_capacity(date_mappings.len() * row_indices.len());
    for date_mapping in date_mappings {
        for row_index in &row_indices {
            marks.push(Sf2CellMark {
                sheet_name: date_mapping.sheet_name.clone(),
                cell_address: format!("{}{}", date_mapping.column_letter, row_index),
                value: String::new(),
            });
        }
    }
    marks
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

fn attendance_grid_rows<I>(row_slots: &[super::calendar_service::TemplateRosterSlot], extra_rows: I) -> Vec<u32>
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
mod tests {
    use super::*;

    // ── parse_clock ──────────────────────────────────────────────────────

    #[test]
    fn parse_clock_normal_time() {
        assert_eq!(parse_clock("08:30"), Some((8, 30)));
    }

    #[test]
    fn parse_clock_afternoon_time() {
        assert_eq!(parse_clock("14:00"), Some((14, 0)));
    }

    #[test]
    fn parse_clock_midnight() {
        assert_eq!(parse_clock("00:00"), Some((0, 0)));
    }

    #[test]
    fn parse_clock_last_valid_minute() {
        assert_eq!(parse_clock("23:59"), Some((23, 59)));
    }

    #[test]
    fn parse_clock_hour_exceeds_23() {
        assert_eq!(parse_clock("24:00"), None);
    }

    #[test]
    fn parse_clock_minute_exceeds_59() {
        assert_eq!(parse_clock("08:60"), None);
    }

    #[test]
    fn parse_clock_minute_exceeds_59_high() {
        assert_eq!(parse_clock("08:99"), None);
    }

    #[test]
    fn parse_clock_empty_string() {
        assert_eq!(parse_clock(""), None);
    }

    #[test]
    fn parse_clock_non_numeric() {
        assert_eq!(parse_clock("abc"), None);
    }

    #[test]
    fn parse_clock_missing_colon() {
        assert_eq!(parse_clock("0830"), None);
    }

    #[test]
    fn parse_clock_leading_whitespace() {
        assert_eq!(parse_clock("  08:30"), Some((8, 30)));
    }

    #[test]
    fn parse_clock_trailing_whitespace() {
        assert_eq!(parse_clock("08:30  "), Some((8, 30)));
    }

    #[test]
    fn parse_clock_single_digit_hour() {
        assert_eq!(parse_clock("8:30"), Some((8, 30)));
    }

    #[test]
    fn parse_clock_invalid_hour_type() {
        assert_eq!(parse_clock("ab:30"), None);
    }

    #[test]
    fn parse_clock_invalid_minute_type() {
        assert_eq!(parse_clock("08:xy"), None);
    }

    // ── mapped_attendance_rows ────────────────────────────────────────────

    #[test]
    fn mapped_attendance_rows_empty() {
        let rows: Vec<u32> = vec![];
        let result = mapped_attendance_rows(rows.into_iter());
        assert!(result.is_empty());
    }

    #[test]
    fn mapped_attendance_rows_single() {
        let result = mapped_attendance_rows(vec![10].into_iter());
        assert_eq!(result, vec![10]);
    }

    #[test]
    fn mapped_attendance_rows_sorts() {
        let result = mapped_attendance_rows(vec![30, 10, 20].into_iter());
        assert_eq!(result, vec![10, 20, 30]);
    }

    #[test]
    fn mapped_attendance_rows_dedupes() {
        let result = mapped_attendance_rows(vec![5, 5, 10, 10].into_iter());
        assert_eq!(result, vec![5, 10]);
    }

    #[test]
    fn mapped_attendance_rows_filters_zero() {
        let result = mapped_attendance_rows(vec![0, 5, 0, 10].into_iter());
        assert_eq!(result, vec![5, 10]);
    }

    #[test]
    fn mapped_attendance_rows_all_zero() {
        let result = mapped_attendance_rows(vec![0, 0].into_iter());
        assert!(result.is_empty());
    }

    // ── attendance_grid_rows ──────────────────────────────────────────────

    #[test]
    fn attendance_grid_rows_includes_slot_rows() {
        use crate::sf2::calendar_service::TemplateRosterSlot;
        let slots = vec![
            TemplateRosterSlot { row_index: 8, gender_block: "MALE" },
            TemplateRosterSlot { row_index: 10, gender_block: "FEMALE" },
        ];
        let result = attendance_grid_rows(&slots, vec![].into_iter());
        assert_eq!(result, vec![8, 10]);
    }

    #[test]
    fn attendance_grid_rows_includes_extra_rows() {
        use crate::sf2::calendar_service::TemplateRosterSlot;
        let slots = vec![
            TemplateRosterSlot { row_index: 8, gender_block: "MALE" },
        ];
        let result = attendance_grid_rows(&slots, vec![12, 14].into_iter());
        assert_eq!(result, vec![8, 12, 14]);
    }

    #[test]
    fn attendance_grid_rows_sorts_and_dedupes() {
        use crate::sf2::calendar_service::TemplateRosterSlot;
        let slots = vec![
            TemplateRosterSlot { row_index: 10, gender_block: "MALE" },
            TemplateRosterSlot { row_index: 8, gender_block: "MALE" },
        ];
        let result = attendance_grid_rows(&slots, vec![10, 12].into_iter());
        assert_eq!(result, vec![8, 10, 12]);
    }

    #[test]
    fn attendance_grid_rows_no_extra_rows() {
        use crate::sf2::calendar_service::TemplateRosterSlot;
        let slots = vec![
            TemplateRosterSlot { row_index: 8, gender_block: "MALE" },
            TemplateRosterSlot { row_index: 9, gender_block: "MALE" },
        ];
        let result = attendance_grid_rows(&slots, vec![].into_iter());
        assert_eq!(result, vec![8, 9]);
    }
}
