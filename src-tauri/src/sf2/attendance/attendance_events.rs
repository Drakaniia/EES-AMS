use crate::domain::error::{AppError, Result};
use crate::domain::models::AttendanceType;
use crate::infrastructure::database::{record_audit_event, AuditEventInput};

use chrono::{Local, NaiveDate, Utc};
use rusqlite::params;

/// Set a student's attendance record for a day to the given event type.
///
/// Any existing record (of either type) for that student/day is removed first,
/// so a student never has both an 'in' and an 'absent' record for the same day.
///
/// `reason` is recorded as the note and audit `override_reason` so the trail
/// says which flow wrote the record (e.g. "SF2 preview correction").
#[allow(clippy::too_many_arguments)]
pub(crate) fn set_attendance_event_for_day(
    pool: crate::infrastructure::database::DbPool,
    student_id: &str,
    class_id: &str,
    date: NaiveDate,
    day_start: &str,
    event_type: AttendanceType,
    reason: &str,
) -> Result<()> {
    let (day_start_timestamp, day_end_timestamp) = local_day_bounds_timestamps_for_date(date)?;
    let mut conn = pool.get()?;
    let transaction = conn.transaction()?;
    let deleted_events: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM events
             WHERE student_id = ?1
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
         AND timestamp >= ?2
         AND timestamp < ?3
         AND (class_id IS NULL OR class_id = ?4)",
        params![student_id, day_start_timestamp, day_end_timestamp, class_id],
    )?;

    let attendance_timestamp = attendance_timestamp_for_date(date, day_start)?;
    let session_key = format!("{date}|{class_id}|day");
    let event_id = uuid::Uuid::new_v4().to_string();
    transaction.execute(
        "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
        params![
            event_id.as_str(),
            student_id,
            class_id,
            event_type.as_db_value(),
            attendance_timestamp,
            reason,
            session_key,
            reason,
        ],
    )?;
    let created_event_id: Option<String> = Some(event_id);

    let metadata_json = serde_json::to_string(&serde_json::json!({
        "studentId": student_id,
        "classId": class_id,
        "date": date.to_string(),
        "eventType": event_type.as_db_value(),
        "deletedEvents": deleted_events,
        "createdEventId": created_event_id.as_deref(),
    }))
    .map_err(|error| AppError::Internal(format!("failed to serialize audit metadata: {error}")))?;
    let summary = format!(
        "Set SF2 attendance for student {student_id} on {date} to {} ({reason})",
        event_type.as_db_value()
    );
    record_audit_event(
        &transaction,
        AuditEventInput {
            entity_type: "attendance_event",
            entity_id: created_event_id.as_deref(),
            action: "create",
            summary: &summary,
            before_json: None,
            after_json: None,
            metadata_json: Some(metadata_json),
        },
    )?;

    transaction.commit()?;
    Ok(())
}

/// True when the student already has an explicit `absent` record for that local
/// day. Used by the workbook→DB import so re-running it is a no-op instead of
/// rewriting the same absence (and audit-logging it) over and over.
pub(crate) fn has_absent_event_for_day(
    pool: &crate::infrastructure::database::DbPool,
    student_id: &str,
    class_id: &str,
    date: NaiveDate,
) -> Result<bool> {
    has_event_of_type_for_day(pool, student_id, class_id, date, AttendanceType::Absent)
}

/// True when the student already has an explicit `in` record for that local day.
///
/// The counterpart to [`has_absent_event_for_day`], and the reason the unattended
/// self-heal is able to be additive (D1, spec §8.3). A blank SF2 cell means
/// "present" (`SF2_PRESENT_MARK` is `""`), so a workbook `X` over a day the app
/// holds a `present` for is the app and the school disagreeing - and which of the
/// two is right is not something an unattended process is entitled to decide.
/// See `heal::import_recovered_marks`.
pub(crate) fn has_present_event_for_day(
    pool: &crate::infrastructure::database::DbPool,
    student_id: &str,
    class_id: &str,
    date: NaiveDate,
) -> Result<bool> {
    has_event_of_type_for_day(pool, student_id, class_id, date, AttendanceType::In)
}

/// True when the student has an explicit record of `event_type` for that local day.
///
/// The shared predicate behind the two questions above. An event with no
/// `class_id` counts as belonging to the class, which is how a learner marked
/// outside a class switch is stored - the same rule
/// `set_attendance_event_for_day` deletes by, so "is there a record?" and "what
/// would this write replace?" can never disagree.
pub(crate) fn has_event_of_type_for_day(
    pool: &crate::infrastructure::database::DbPool,
    student_id: &str,
    class_id: &str,
    date: NaiveDate,
    event_type: AttendanceType,
) -> Result<bool> {
    let (day_start_timestamp, day_end_timestamp) = local_day_bounds_timestamps_for_date(date)?;
    let query = HAS_EVENT_OF_TYPE_FOR_DAY.replace("{event_type}", event_type.as_db_value());
    let conn = pool.get()?;
    let count: i64 = conn.query_row(
        &query,
        params![student_id, day_start_timestamp, day_end_timestamp, class_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

const HAS_EVENT_OF_TYPE_FOR_DAY: &str = include_str!("../sql/has_event_of_type_for_day.sql");

pub(crate) fn local_day_bounds_timestamps_for_date(date: NaiveDate) -> Result<(i64, i64)> {
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

pub(crate) fn parse_clock(value: &str) -> Option<(u32, u32)> {
    let (hour, minute) = value.trim().split_once(':')?;
    let hour = hour.parse::<u32>().ok()?;
    let minute = minute.parse::<u32>().ok()?;
    if hour < 24 && minute < 60 {
        Some((hour, minute))
    } else {
        None
    }
}
