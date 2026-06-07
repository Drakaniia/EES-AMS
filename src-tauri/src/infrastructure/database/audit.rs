use super::{rows::audit_event_from_row, DbPool};
use crate::domain::{error::Result, models::AuditEvent};
use chrono::Utc;
use rusqlite::params;

pub(super) struct AuditEventDraft {
    entity_type: String,
    entity_id: Option<String>,
    action: String,
    summary: String,
    before_json: Option<String>,
    after_json: Option<String>,
    metadata_json: Option<String>,
    actor: String,
}

impl AuditEventDraft {
    pub(super) fn new(
        entity_type: impl Into<String>,
        entity_id: Option<String>,
        action: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            entity_type: entity_type.into(),
            entity_id,
            action: action.into(),
            summary: summary.into(),
            before_json: None,
            after_json: None,
            metadata_json: None,
            actor: "admin".to_string(),
        }
    }

    pub(super) fn before_json(mut self, value: String) -> Self {
        self.before_json = Some(value);
        self
    }

    pub(super) fn after_json(mut self, value: String) -> Self {
        self.after_json = Some(value);
        self
    }

    pub(super) fn metadata_json(mut self, value: String) -> Self {
        self.metadata_json = Some(value);
        self
    }
}

pub(super) fn insert_audit_event(
    conn: &rusqlite::Connection,
    draft: AuditEventDraft,
) -> Result<AuditEvent> {
    let event = AuditEvent {
        id: uuid::Uuid::new_v4().to_string(),
        entity_type: draft.entity_type,
        entity_id: draft.entity_id,
        action: draft.action,
        summary: draft.summary,
        before_json: draft.before_json,
        after_json: draft.after_json,
        metadata_json: draft.metadata_json,
        created_at: Utc::now(),
        actor: draft.actor,
    };

    conn.execute(
        "INSERT INTO audit_events (id, entity_type, entity_id, action, summary, before_json, after_json, metadata_json, created_at, actor)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            event.id,
            event.entity_type,
            event.entity_id,
            event.action,
            event.summary,
            event.before_json,
            event.after_json,
            event.metadata_json,
            event.created_at.timestamp(),
            event.actor,
        ],
    )?;

    Ok(event)
}

pub struct AuditEventInput<'a> {
    pub entity_type: &'a str,
    pub entity_id: Option<&'a str>,
    pub action: &'a str,
    pub summary: &'a str,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub metadata_json: Option<String>,
}

pub fn record_audit_event(
    conn: &rusqlite::Connection,
    input: AuditEventInput<'_>,
) -> Result<AuditEvent> {
    let mut draft = AuditEventDraft::new(
        input.entity_type,
        input.entity_id.map(str::to_string),
        input.action,
        input.summary,
    );
    draft.before_json = input.before_json;
    draft.after_json = input.after_json;
    draft.metadata_json = input.metadata_json;

    insert_audit_event(conn, draft)
}

/// General audit repository
pub struct AuditRepository {
    pool: DbPool,
}

impl AuditRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn record(&self, input: AuditEventInput<'_>) -> Result<AuditEvent> {
        let conn = self.pool.get()?;
        record_audit_event(&conn, input)
    }

    pub fn list(&self, limit: Option<i64>) -> Result<Vec<AuditEvent>> {
        let conn = self.pool.get()?;
        let limit = limit.unwrap_or(200).clamp(1, 1_000);
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, entity_id, action, summary, before_json, after_json, metadata_json, created_at, actor
             FROM audit_events
             ORDER BY created_at DESC, id DESC
             LIMIT ?1",
        )?;
        let events = stmt
            .query_map(params![limit], audit_event_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(events)
    }

    pub fn list_all(&self) -> Result<Vec<AuditEvent>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, entity_id, action, summary, before_json, after_json, metadata_json, created_at, actor
             FROM audit_events
             ORDER BY created_at ASC, id ASC",
        )?;
        let events = stmt
            .query_map([], audit_event_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(events)
    }
}
