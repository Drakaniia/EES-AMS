use super::super::{
    audit::{insert_audit_event, AuditEventDraft},
    rows::{
        attendance_audit_entry_from_row, attendance_session_key, audit_metadata,
        normalize_optional_text, serialize_audit_event, serialize_audit_payload,
    },
};
use super::{DuplicateAttendancePolicy, EventRepository};
use crate::domain::{
    error::{AppError, Result},
    models::*,
};
use chrono::Utc;
use rusqlite::params;

impl EventRepository {
    /// Create an attendance event
    pub fn create(&self, req: CreateEventRequest) -> Result<AttendanceEvent> {
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        let event =
            Self::insert_create_event(&transaction, req, DuplicateAttendancePolicy::Reject)?
                .ok_or_else(|| {
                    AppError::Internal("attendance event was unexpectedly skipped".to_string())
                })?;

        transaction.commit()?;
        Ok(event)
    }

    /// Create multiple attendance events in a single transaction.
    pub fn create_many(&self, reqs: Vec<CreateEventRequest>) -> Result<Vec<AttendanceEvent>> {
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        let mut events = Vec::with_capacity(reqs.len());

        for req in reqs {
            if let Some(event) =
                Self::insert_create_event(&transaction, req, DuplicateAttendancePolicy::Skip)?
            {
                events.push(event);
            }
        }

        transaction.commit()?;
        Ok(events)
    }

    fn insert_create_event(
        transaction: &rusqlite::Transaction<'_>,
        req: CreateEventRequest,
        duplicate_policy: DuplicateAttendancePolicy,
    ) -> Result<Option<AttendanceEvent>> {
        let timestamp = req.timestamp.unwrap_or_else(Utc::now);
        let class_id = normalize_optional_text(req.class_id);
        let session_key = normalize_optional_text(req.session_key)
            .unwrap_or_else(|| attendance_session_key(timestamp, class_id.as_deref()));
        let override_reason = normalize_optional_text(req.override_reason);

        let existing_count: i32 = transaction
            .query_row(
                "SELECT COUNT(*) FROM events 
                 WHERE student_id = ?1 
                 AND event_type = 'in' 
                 AND session_key = ?2",
                params![req.student_id.0.to_string(), session_key],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if existing_count > 0 && override_reason.is_none() {
            return match duplicate_policy {
                DuplicateAttendancePolicy::Reject => Err(AppError::DuplicateAttendance(
                    "Student already recorded for this session".to_string(),
                )),
                DuplicateAttendancePolicy::Skip => Ok(None),
            };
        }

        let event = AttendanceEvent {
            id: EventId::new(),
            student_id: req.student_id,
            class_id,
            event_type: req.event_type,
            timestamp,
            note: normalize_optional_text(req.note),
            session_key: Some(session_key),
            override_reason,
            updated_at: None,
        };

        transaction.execute(
            "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                event.id.0.to_string(),
                event.student_id.0.to_string(),
                event.class_id.as_deref(),
                "in",
                event.timestamp.timestamp(),
                event.note.as_deref(),
                event.session_key.as_deref(),
                event.override_reason.as_deref(),
                event.updated_at.map(|timestamp| timestamp.timestamp()),
            ],
        )?;

        if let Some(reason) = event.override_reason.as_ref() {
            let after_json = serialize_audit_event(&event)?;
            transaction.execute(
                "INSERT INTO attendance_event_audit (id, event_id, student_id, class_id, session_key, action, reason, before_json, after_json, created_at, actor)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'create_override', ?6, NULL, ?7, ?8, 'admin')",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    event.id.0.to_string(),
                    event.student_id.0.to_string(),
                    event.class_id.as_deref(),
                    event.session_key.as_deref(),
                    reason,
                    after_json,
                    Utc::now().timestamp(),
                ],
            )?;
        }

        let after_json = serialize_audit_payload("attendance event audit payload", &event)?;
        let metadata_json = audit_metadata(serde_json::json!({
            "overrideReason": event.override_reason.as_deref(),
        }))?;
        insert_audit_event(
            transaction,
            AuditEventDraft::new(
                "attendance_event",
                Some(event.id.0.to_string()),
                "create",
                format!("Recorded attendance for student {}", event.student_id),
            )
            .after_json(after_json)
            .metadata_json(metadata_json),
        )?;

        Ok(Some(event))
    }

    /// Update an attendance event and record the edit reason
    pub fn update(&self, id: EventId, req: UpdateEventRequest) -> Result<AttendanceEvent> {
        let reason = normalize_optional_text(Some(req.reason))
            .ok_or_else(|| AppError::InvalidInput("audit reason is required".to_string()))?;
        let has_class_update = req.class_id.is_some();
        let has_session_update = req.session_key.is_some();
        let has_timestamp_update = req.timestamp.is_some();
        let before = self.get(id)?;
        let timestamp = req.timestamp.unwrap_or(before.timestamp);
        let class_id = if has_class_update {
            normalize_optional_text(req.class_id)
        } else {
            before.class_id.clone()
        };
        let note = match req.note {
            Some(note) => normalize_optional_text(Some(note)),
            None => before.note.clone(),
        };
        let session_key = if has_session_update {
            normalize_optional_text(req.session_key)
                .unwrap_or_else(|| attendance_session_key(timestamp, class_id.as_deref()))
        } else if has_class_update || has_timestamp_update || before.session_key.is_none() {
            attendance_session_key(timestamp, class_id.as_deref())
        } else {
            before.session_key.clone().unwrap()
        };
        let updated_at = Utc::now();
        let event = AttendanceEvent {
            id: before.id,
            student_id: before.student_id,
            class_id,
            event_type: before.event_type,
            timestamp,
            note,
            session_key: Some(session_key),
            override_reason: Some(reason.clone()),
            updated_at: Some(updated_at),
        };

        let before_json = serialize_audit_event(&before)?;
        let after_json = serialize_audit_event(&event)?;
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;

        transaction.execute(
            "UPDATE events
             SET class_id = ?1, timestamp = ?2, note = ?3, session_key = ?4, override_reason = ?5, updated_at = ?6
             WHERE id = ?7",
            params![
                event.class_id.as_deref(),
                event.timestamp.timestamp(),
                event.note.as_deref(),
                event.session_key.as_deref(),
                event.override_reason.as_deref(),
                updated_at.timestamp(),
                event.id.0.to_string(),
            ],
        )?;
        transaction.execute(
            "INSERT INTO attendance_event_audit (id, event_id, student_id, class_id, session_key, action, reason, before_json, after_json, created_at, actor)
             VALUES (?1, ?2, ?3, ?4, ?5, 'update', ?6, ?7, ?8, ?9, 'admin')",
            params![
                uuid::Uuid::new_v4().to_string(),
                event.id.0.to_string(),
                event.student_id.0.to_string(),
                event.class_id.as_deref(),
                event.session_key.as_deref(),
                reason,
                before_json,
                after_json,
                updated_at.timestamp(),
            ],
        )?;
        let metadata_json = audit_metadata(serde_json::json!({
            "reason": reason,
        }))?;
        insert_audit_event(
            &transaction,
            AuditEventDraft::new(
                "attendance_event",
                Some(event.id.0.to_string()),
                "update",
                format!("Updated attendance record for student {}", event.student_id),
            )
            .before_json(before_json.clone())
            .after_json(after_json)
            .metadata_json(metadata_json),
        )?;

        transaction.commit()?;
        Ok(event)
    }

    /// Delete an event
    pub fn delete(&self, id: EventId, reason: Option<String>) -> Result<()> {
        let before = self.get(id)?;
        let audit_reason = normalize_optional_text(reason);
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        let rows = transaction.execute(
            "DELETE FROM events WHERE id = ?1",
            params![id.0.to_string()],
        )?;

        if rows == 0 {
            return Err(AppError::EventNotFound(id.0.to_string()));
        }

        if let Some(reason) = audit_reason.as_ref() {
            let before_json = serialize_audit_event(&before)?;
            transaction.execute(
                "INSERT INTO attendance_event_audit (id, event_id, student_id, class_id, session_key, action, reason, before_json, after_json, created_at, actor)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'delete', ?6, ?7, NULL, ?8, 'admin')",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    before.id.0.to_string(),
                    before.student_id.0.to_string(),
                    before.class_id.as_deref(),
                    before.session_key.as_deref(),
                    reason,
                    before_json,
                    Utc::now().timestamp(),
                ],
            )?;
        }

        let before_json = serialize_audit_payload("attendance event audit payload", &before)?;
        let metadata_json = audit_metadata(serde_json::json!({
            "reason": audit_reason.as_deref(),
        }))?;
        insert_audit_event(
            &transaction,
            AuditEventDraft::new(
                "attendance_event",
                Some(before.id.0.to_string()),
                "delete",
                format!(
                    "Deleted attendance record for student {}",
                    before.student_id
                ),
            )
            .before_json(before_json)
            .metadata_json(metadata_json),
        )?;

        transaction.commit()?;
        Ok(())
    }

    /// Delete multiple events in a single transaction.
    /// Returns the list of event IDs that were successfully deleted.
    pub fn delete_many(&self, ids: &[EventId], reason: Option<String>) -> Result<Vec<EventId>> {
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        let mut deleted = Vec::new();
        let audit_reason = normalize_optional_text(reason);

        for &id in ids {
            let before = match Self::get_event_inner(&transaction, id) {
                Ok(event) => event,
                Err(_) => continue,
            };

            let rows = transaction.execute(
                "DELETE FROM events WHERE id = ?1",
                params![id.0.to_string()],
            )?;
            if rows == 0 {
                continue;
            }

            deleted.push(id);

            if let Some(ref reason) = audit_reason {
                let before_json = serialize_audit_event(&before)?;
                transaction.execute(
                    "INSERT INTO attendance_event_audit (id, event_id, student_id, class_id, session_key, action, reason, before_json, after_json, created_at, actor)
                     VALUES (?1, ?2, ?3, ?4, ?5, 'delete', ?6, ?7, NULL, ?8, 'admin')",
                    params![
                        uuid::Uuid::new_v4().to_string(),
                        before.id.0.to_string(),
                        before.student_id.0.to_string(),
                        before.class_id.as_deref(),
                        before.session_key.as_deref(),
                        reason,
                        before_json,
                        Utc::now().timestamp(),
                    ],
                )?;
            }

            let before_json = serialize_audit_payload("attendance event audit payload", &before)?;
            let metadata_json = audit_metadata(serde_json::json!({
                "reason": audit_reason.as_deref(),
            }))?;
            insert_audit_event(
                &transaction,
                AuditEventDraft::new(
                    "attendance_event",
                    Some(before.id.0.to_string()),
                    "delete",
                    format!(
                        "Deleted attendance record for student {}",
                        before.student_id
                    ),
                )
                .before_json(before_json)
                .metadata_json(metadata_json),
            )?;
        }

        transaction.commit()?;
        Ok(deleted)
    }

    /// List attendance audit entries
    pub fn list_audit(
        &self,
        event_id: Option<EventId>,
        student_id: Option<StudentId>,
    ) -> Result<Vec<AttendanceAuditEntry>> {
        let conn = self.pool.get()?;
        let sql = "SELECT id, event_id, student_id, class_id, session_key, action, reason, before_json, after_json, created_at, actor
                   FROM attendance_event_audit";

        let entries = match (event_id, student_id) {
            (Some(event_id), Some(student_id)) => {
                let mut stmt = conn.prepare(&format!(
                    "{sql} WHERE event_id = ?1 OR student_id = ?2 ORDER BY created_at DESC"
                ))?;
                let entries = stmt
                    .query_map(
                        params![event_id.0.to_string(), student_id.0.to_string()],
                        attendance_audit_entry_from_row,
                    )?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                entries
            }
            (Some(event_id), None) => {
                let mut stmt = conn.prepare(&format!(
                    "{sql} WHERE event_id = ?1 ORDER BY created_at DESC"
                ))?;
                let entries = stmt
                    .query_map(
                        params![event_id.0.to_string()],
                        attendance_audit_entry_from_row,
                    )?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                entries
            }
            (None, Some(student_id)) => {
                let mut stmt = conn.prepare(&format!(
                    "{sql} WHERE student_id = ?1 ORDER BY created_at DESC"
                ))?;
                let entries = stmt
                    .query_map(
                        params![student_id.0.to_string()],
                        attendance_audit_entry_from_row,
                    )?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                entries
            }
            (None, None) => {
                let mut stmt =
                    conn.prepare(&format!("{sql} ORDER BY created_at DESC LIMIT 200"))?;
                let entries = stmt
                    .query_map([], attendance_audit_entry_from_row)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                entries
            }
        };

        Ok(entries)
    }
}
