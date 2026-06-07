use super::{
    audit::{insert_audit_event, AuditEventDraft},
    rows::{audit_metadata, class_from_row, normalize_optional_text, serialize_audit_payload},
    DbPool,
};
use crate::domain::{
    error::{AppError, Result},
    models::*,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};

/// Class repository
pub struct ClassRepository {
    pool: DbPool,
}

impl ClassRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// List all classes
    pub fn list(&self) -> Result<Vec<Class>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, room, day_start, day_end, late_after, created_at, sessions, days 
             FROM classes 
             ORDER BY name ASC",
        )?;

        let classes = stmt
            .query_map([], class_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(classes)
    }

    /// Get class by ID
    pub fn get(&self, id: &str) -> Result<Option<Class>> {
        let conn = self.pool.get()?;
        let class = conn
            .query_row(
                "SELECT id, name, room, day_start, day_end, late_after, created_at, sessions, days 
                 FROM classes 
                 WHERE id = ?1",
                params![id],
                class_from_row,
            )
            .optional()?;

        Ok(class)
    }

    /// Create a new class
    pub fn create(&self, req: CreateClassRequest) -> Result<Class> {
        let room = normalize_optional_text(req.room);
        let sessions_json =
            serde_json::to_string(&req.sessions).unwrap_or_else(|_| "[]".to_string());
        let days_json = serde_json::to_string(&req.days).unwrap_or_else(|_| "[]".to_string());

        let class = Class {
            id: uuid::Uuid::new_v4().to_string(),
            name: req.name,
            room,
            day_start: req.day_start,
            day_end: req.day_end,
            late_after: req.late_after,
            sessions: req.sessions,
            days: req.days,
            created_at: Utc::now(),
        };

        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        transaction.execute(
            "INSERT INTO classes (id, name, room, day_start, day_end, late_after, sessions, days, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                class.id.as_str(),
                class.name.as_str(),
                class.room.as_deref(),
                class.day_start.as_str(),
                class.day_end.as_str(),
                class.late_after.as_str(),
                sessions_json.as_str(),
                days_json.as_str(),
                class.created_at.timestamp(),
            ],
        )?;
        let after_json = serialize_audit_payload("class audit payload", &class)?;
        insert_audit_event(
            &transaction,
            AuditEventDraft::new(
                "class",
                Some(class.id.clone()),
                "create",
                format!("Created class {}", class.name),
            )
            .after_json(after_json),
        )?;
        transaction.commit()?;

        Ok(class)
    }

    /// Update a class
    pub fn update(&self, id: &str, req: UpdateClassRequest) -> Result<Class> {
        let before = self
            .get(id)?
            .ok_or_else(|| AppError::ClassNotFound(id.to_string()))?;
        let mut class = before.clone();

        if let Some(name) = req.name {
            class.name = name;
        }
        if let Some(room) = req.room {
            class.room = normalize_optional_text(Some(room));
        }
        if let Some(day_start) = req.day_start {
            class.day_start = day_start;
        }
        if let Some(day_end) = req.day_end {
            class.day_end = day_end;
        }
        if let Some(late_after) = req.late_after {
            class.late_after = late_after;
        }
        if let Some(sessions) = req.sessions {
            class.sessions = sessions;
        }
        if let Some(days) = req.days {
            class.days = days;
        }

        let sessions_json =
            serde_json::to_string(&class.sessions).unwrap_or_else(|_| "[]".to_string());
        let days_json = serde_json::to_string(&class.days).unwrap_or_else(|_| "[]".to_string());

        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        transaction.execute(
            "UPDATE classes 
             SET name = ?1, room = ?2, day_start = ?3, day_end = ?4, late_after = ?5, sessions = ?6, days = ?7 
             WHERE id = ?8",
            params![
                class.name.as_str(),
                class.room.as_deref(),
                class.day_start.as_str(),
                class.day_end.as_str(),
                class.late_after.as_str(),
                sessions_json.as_str(),
                days_json.as_str(),
                id,
            ],
        )?;
        let before_json = serialize_audit_payload("class audit before payload", &before)?;
        let after_json = serialize_audit_payload("class audit after payload", &class)?;
        insert_audit_event(
            &transaction,
            AuditEventDraft::new(
                "class",
                Some(class.id.clone()),
                "update",
                format!("Updated class {}", class.name),
            )
            .before_json(before_json)
            .after_json(after_json),
        )?;
        transaction.commit()?;

        Ok(class)
    }

    /// Delete a class
    pub fn delete(&self, id: &str) -> Result<()> {
        let before = self
            .get(id)?
            .ok_or_else(|| AppError::ClassNotFound(id.to_string()))?;
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        let affected_students: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM students WHERE class_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let affected_events: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM events WHERE class_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let deleted_templates: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM sf2_templates WHERE active_class_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        transaction.execute(
            "DELETE FROM sf2_templates WHERE active_class_id = ?1",
            params![id],
        )?;
        transaction.execute(
            "DELETE FROM attendance_day_status WHERE class_id = ?1",
            params![id],
        )?;
        transaction.execute(
            "UPDATE events SET class_id = NULL WHERE class_id = ?1",
            params![id],
        )?;
        transaction.execute(
            "UPDATE students SET class_id = NULL WHERE class_id = ?1",
            params![id],
        )?;
        let rows = transaction.execute("DELETE FROM classes WHERE id = ?1", params![id])?;

        if rows == 0 {
            return Err(AppError::ClassNotFound(id.to_string()));
        }

        let before_json = serialize_audit_payload("class audit before payload", &before)?;
        let metadata_json = audit_metadata(serde_json::json!({
            "affectedStudents": affected_students,
            "affectedEvents": affected_events,
            "deletedSf2Templates": deleted_templates,
        }))?;
        insert_audit_event(
            &transaction,
            AuditEventDraft::new(
                "class",
                Some(before.id.clone()),
                "delete",
                format!("Deleted class {}", before.name),
            )
            .before_json(before_json)
            .metadata_json(metadata_json),
        )?;
        transaction.commit()?;
        Ok(())
    }
}
