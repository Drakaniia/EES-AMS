use crate::domain::error::{AppError, Result};
use crate::infrastructure::database::{record_audit_event, AuditEventInput};

use chrono::{Local, NaiveDate, Utc};
use rusqlite::params;

pub(super) fn set_attendance_event_for_day(
    pool: crate::infrastructure::database::DbPool,
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
