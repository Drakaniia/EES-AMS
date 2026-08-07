mod write;

use super::{rows::attendance_event_from_row, DbPool};
use crate::domain::{
    error::{AppError, Result},
    models::*,
};
use chrono::{Duration, Local, NaiveDate, TimeZone, Utc};
use rusqlite::{params, OptionalExtension};

/// Event repository
pub struct EventRepository {
    pub(crate) pool: DbPool,
}

enum DuplicateAttendancePolicy {
    Reject,
    Skip,
}

impl EventRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// List all events
    pub fn list(&self) -> Result<Vec<AttendanceEvent>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at
             FROM events 
             ORDER BY timestamp DESC",
        )?;

        let events = stmt
            .query_map([], attendance_event_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// List events filtered by class ID and date range (inclusive of both dates).
    /// Uses the local timezone to convert date strings to UTC timestamp bounds.
    pub fn list_for_class_and_date_range(
        &self,
        class_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<AttendanceEvent>> {
        let start_naive = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
            .map_err(|error| AppError::InvalidInput(format!("invalid start date: {error}")))?;
        let end_naive = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
            .map_err(|error| AppError::InvalidInput(format!("invalid end date: {error}")))?;
        // End date inclusive: add one day to cover the full end day
        let end_naive_exclusive = end_naive.succ_opt().unwrap_or(end_naive);

        let start_local = Local
            .from_local_datetime(&start_naive.and_hms_opt(0, 0, 0).unwrap())
            .earliest()
            .ok_or_else(|| AppError::InvalidInput("invalid start date".to_string()))?;
        let end_local = Local
            .from_local_datetime(&end_naive_exclusive.and_hms_opt(0, 0, 0).unwrap())
            .earliest()
            .ok_or_else(|| AppError::InvalidInput("invalid end date".to_string()))?;
        let start_timestamp = start_local.with_timezone(&Utc).timestamp();
        let end_timestamp = end_local.with_timezone(&Utc).timestamp();

        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at
             FROM events
             WHERE class_id = ?1
             AND timestamp >= ?2
             AND timestamp < ?3
             ORDER BY timestamp DESC",
        )?;

        let events = stmt
            .query_map(
                params![class_id, start_timestamp, end_timestamp],
                attendance_event_from_row,
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// List events for a local calendar date.
    pub fn list_for_local_date(&self, date: &str) -> Result<Vec<AttendanceEvent>> {
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|error| AppError::InvalidInput(format!("invalid attendance date: {error}")))?;
        let start_naive = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| AppError::InvalidInput("invalid attendance date".to_string()))?;
        let start_local = Local
            .from_local_datetime(&start_naive)
            .earliest()
            .ok_or_else(|| AppError::InvalidInput("invalid local attendance date".to_string()))?;
        let end_local = start_local + Duration::days(1);
        let start_timestamp = start_local.with_timezone(&Utc).timestamp();
        let end_timestamp = end_local.with_timezone(&Utc).timestamp();

        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at
             FROM events
             WHERE timestamp >= ?1
             AND timestamp < ?2
             ORDER BY timestamp DESC",
        )?;

        let events = stmt
            .query_map(
                params![start_timestamp, end_timestamp],
                attendance_event_from_row,
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// Get event by ID
    pub fn get(&self, id: EventId) -> Result<AttendanceEvent> {
        let conn = self.pool.get()?;
        let event = conn
            .query_row(
                "SELECT id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at
                 FROM events
                 WHERE id = ?1",
                params![id.0.to_string()],
                attendance_event_from_row,
            )
            .optional()?
            .ok_or_else(|| AppError::EventNotFound(id.0.to_string()))?;

        Ok(event)
    }

    /// List events for a specific student
    pub fn list_for_student(&self, student_id: StudentId) -> Result<Vec<AttendanceEvent>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at
             FROM events 
             WHERE student_id = ?1
             ORDER BY timestamp DESC",
        )?;

        let events = stmt
            .query_map(params![student_id.0.to_string()], attendance_event_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// Get last event for a student
    pub fn last_for_student(&self, student_id: StudentId) -> Result<Option<AttendanceEvent>> {
        let conn = self.pool.get()?;
        let event = conn
            .query_row(
                "SELECT id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at
                 FROM events 
                 WHERE student_id = ?1
                 ORDER BY timestamp DESC 
                 LIMIT 1",
                params![student_id.0.to_string()],
                attendance_event_from_row,
            )
            .optional()?;

        Ok(event)
    }

    /// Get an event within an existing transaction (no commit).
    fn get_event_inner(
        transaction: &rusqlite::Transaction<'_>,
        id: EventId,
    ) -> Result<AttendanceEvent> {
        let event = transaction
            .query_row(
                "SELECT id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at
                 FROM events
                 WHERE id = ?1",
                params![id.0.to_string()],
                attendance_event_from_row,
            )
            .optional()?
            .ok_or_else(|| AppError::EventNotFound(id.0.to_string()))?;
        Ok(event)
    }
}
