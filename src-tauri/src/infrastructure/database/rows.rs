use crate::domain::{
    error::{AppError, Result},
    models::*,
};
use chrono::{DateTime, Local, Utc};
use rusqlite::Row;
use serde::Serialize;

pub(super) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(super) fn attendance_session_key(timestamp: DateTime<Utc>, class_id: Option<&str>) -> String {
    let local_date = timestamp.with_timezone(&Local).format("%Y-%m-%d");
    let class_key = class_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unassigned");

    format!("{local_date}|{class_key}|day")
}

pub(super) fn serialize_audit_event(event: &AttendanceEvent) -> Result<String> {
    serde_json::to_string(event)
        .map_err(|error| AppError::Internal(format!("failed to serialize audit event: {error}")))
}

pub(super) fn attendance_event_from_row(row: &Row<'_>) -> rusqlite::Result<AttendanceEvent> {
    let updated_at = row
        .get::<_, Option<i64>>(8)?
        .and_then(|timestamp| DateTime::from_timestamp(timestamp, 0))
        .map(|timestamp| timestamp.with_timezone(&Utc));

    Ok(AttendanceEvent {
        id: EventId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
        student_id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(1)?).unwrap()),
        class_id: row.get(2)?,
        event_type: AttendanceType::from_db_value(&row.get::<_, String>(3)?),
        timestamp: DateTime::from_timestamp(row.get::<_, i64>(4)?, 0)
            .unwrap()
            .with_timezone(&Utc),
        note: row.get(5)?,
        session_key: row.get(6)?,
        override_reason: row.get(7)?,
        updated_at,
    })
}

pub(super) fn attendance_audit_entry_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<AttendanceAuditEntry> {
    let event_id = row
        .get::<_, Option<String>>(1)?
        .and_then(|id| uuid::Uuid::parse_str(&id).ok())
        .map(EventId);

    Ok(AttendanceAuditEntry {
        id: row.get(0)?,
        event_id,
        student_id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(2)?).unwrap()),
        class_id: row.get(3)?,
        session_key: row.get(4)?,
        action: row.get(5)?,
        reason: row.get(6)?,
        before_json: row.get(7)?,
        after_json: row.get(8)?,
        created_at: DateTime::from_timestamp(row.get::<_, i64>(9)?, 0)
            .unwrap()
            .with_timezone(&Utc),
        actor: row.get(10)?,
    })
}

pub(super) fn audit_event_from_row(row: &Row<'_>) -> rusqlite::Result<AuditEvent> {
    Ok(AuditEvent {
        id: row.get(0)?,
        entity_type: row.get(1)?,
        entity_id: row.get(2)?,
        action: row.get(3)?,
        summary: row.get(4)?,
        before_json: row.get(5)?,
        after_json: row.get(6)?,
        metadata_json: row.get(7)?,
        created_at: DateTime::from_timestamp(row.get::<_, i64>(8)?, 0)
            .unwrap()
            .with_timezone(&Utc),
        actor: row.get(9)?,
    })
}

pub(super) fn student_from_row(row: &Row<'_>) -> rusqlite::Result<Student> {
    Ok(Student {
        id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
        name: row.get(1)?,
        gender: StudentGender::from_db_value(row.get::<_, Option<String>>(2)?.as_deref()),
        card_serial: row.get(3)?,
        class_id: row.get(4)?,
        created_at: DateTime::from_timestamp(row.get::<_, i64>(5)?, 0)
            .unwrap()
            .with_timezone(&Utc),
    })
}

pub(super) fn class_from_row(row: &Row<'_>) -> rusqlite::Result<Class> {
    let room: Option<String> = row.get(2)?;
    let sessions_json: Option<String> = row.get(7)?;
    let sessions: Vec<Session> = sessions_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let days_json: Option<String> = row.get(8)?;
    let days: Vec<i32> = days_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| vec![1, 2, 3, 4, 5]);

    Ok(Class {
        id: row.get(0)?,
        name: row.get(1)?,
        room: room.filter(|r| !r.is_empty()),
        day_start: row.get(3)?,
        day_end: row.get(4)?,
        late_after: row.get(5)?,
        sessions,
        days,
        created_at: DateTime::from_timestamp(row.get::<_, i64>(6)?, 0)
            .unwrap()
            .with_timezone(&Utc),
    })
}

pub(super) fn serialize_audit_payload<T: Serialize>(label: &str, value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|error| AppError::Internal(format!("failed to serialize {label}: {error}")))
}

pub(super) fn audit_metadata(value: serde_json::Value) -> Result<String> {
    serde_json::to_string(&value)
        .map_err(|error| AppError::Internal(format!("failed to serialize audit metadata: {error}")))
}
