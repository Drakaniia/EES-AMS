use super::{
    audit::{insert_audit_event, AuditEventDraft},
    rows::{audit_metadata, normalize_optional_text, serialize_audit_payload, student_from_row},
    DbPool,
};
use crate::domain::{
    error::{AppError, Result},
    models::*,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use std::collections::HashSet;

/// Student repository
pub struct StudentRepository {
    pool: DbPool,
}

impl StudentRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// List all students
    pub fn list(&self) -> Result<Vec<Student>> {
        self.list_by_class(None)
    }

    /// List students by class
    pub fn list_by_class(&self, class_id: Option<&str>) -> Result<Vec<Student>> {
        let conn = self.pool.get()?;
        let students = if let Some(class_id) = class_id {
            let mut stmt = conn.prepare(
                "SELECT id, name, gender, card_serial, class_id, created_at
                 FROM students 
                 WHERE class_id = ?1 
                 ORDER BY name ASC",
            )?;
            let rows = stmt.query_map(params![class_id], student_from_row)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, name, gender, card_serial, class_id, created_at
                 FROM students 
                 ORDER BY name ASC",
            )?;
            let rows = stmt.query_map([], student_from_row)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };

        Ok(students)
    }

    /// Get student by ID
    pub fn get(&self, id: StudentId) -> Result<Student> {
        let conn = self.pool.get()?;
        let student = conn
            .query_row(
                "SELECT id, name, gender, card_serial, class_id, created_at
                 FROM students 
                 WHERE id = ?1",
                params![id.0.to_string()],
                student_from_row,
            )
            .optional()?
            .ok_or_else(|| AppError::StudentNotFound(id.0.to_string()))?;

        Ok(student)
    }

    /// Find student by card serial
    pub fn find_by_card(&self, serial: &str) -> Result<Option<Student>> {
        let conn = self.pool.get()?;
        let student = conn
            .query_row(
                "SELECT id, name, gender, card_serial, class_id, created_at
                 FROM students 
                 WHERE card_serial = ?1",
                params![serial],
                student_from_row,
            )
            .optional()?;

        Ok(student)
    }

    /// Create a new student
    pub fn create(&self, req: CreateStudentRequest) -> Result<Student> {
        let card_serial = normalize_optional_text(req.card_serial);
        let class_id = normalize_optional_text(req.class_id);

        // Check if card serial is already registered
        if let Some(ref serial) = card_serial {
            if self.find_by_card(serial)?.is_some() {
                return Err(AppError::CardAlreadyRegistered(serial.clone()));
            }
        }

        let student = Student {
            id: StudentId::new(),
            name: req.name,
            gender: req.gender,
            card_serial,
            class_id,
            created_at: Utc::now(),
        };

        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        transaction.execute(
            "INSERT INTO students (id, name, gender, card_serial, class_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                student.id.0.to_string(),
                student.name.as_str(),
                student.gender.map(StudentGender::as_db_value),
                student.card_serial.as_deref(),
                student.class_id.as_deref(),
                student.created_at.timestamp(),
            ],
        )?;
        let after_json = serialize_audit_payload("student audit payload", &student)?;
        insert_audit_event(
            &transaction,
            AuditEventDraft::new(
                "student",
                Some(student.id.to_string()),
                "create",
                format!("Created student {}", student.name),
            )
            .after_json(after_json),
        )?;
        transaction.commit()?;

        Ok(student)
    }

    /// Create multiple students in a single transaction.
    pub fn create_many(&self, reqs: Vec<CreateStudentRequest>) -> Result<Vec<Student>> {
        let mut normalized = Vec::with_capacity(reqs.len());
        let mut seen_card_serials = HashSet::new();

        for req in reqs {
            let card_serial = normalize_optional_text(req.card_serial);
            let class_id = normalize_optional_text(req.class_id);

            if let Some(serial) = card_serial.as_ref() {
                if !seen_card_serials.insert(serial.clone()) {
                    return Err(AppError::CardAlreadyRegistered(serial.clone()));
                }
            }

            normalized.push((req.name, req.gender, card_serial, class_id));
        }

        let mut conn = self.pool.get()?;
        for (_, _, card_serial, _) in &normalized {
            if let Some(serial) = card_serial.as_ref() {
                let exists = conn
                    .query_row(
                        "SELECT 1 FROM students WHERE card_serial = ?1 LIMIT 1",
                        params![serial],
                        |_| Ok(true),
                    )
                    .optional()?
                    .unwrap_or(false);

                if exists {
                    return Err(AppError::CardAlreadyRegistered(serial.clone()));
                }
            }
        }

        let transaction = conn.transaction()?;
        let mut students = Vec::with_capacity(normalized.len());

        for (name, gender, card_serial, class_id) in normalized {
            let student = Student {
                id: StudentId::new(),
                name,
                gender,
                card_serial,
                class_id,
                created_at: Utc::now(),
            };

            transaction.execute(
                "INSERT INTO students (id, name, gender, card_serial, class_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    student.id.0.to_string(),
                    student.name.as_str(),
                    student.gender.map(StudentGender::as_db_value),
                    student.card_serial.as_deref(),
                    student.class_id.as_deref(),
                    student.created_at.timestamp(),
                ],
            )?;
            let after_json = serialize_audit_payload("student audit payload", &student)?;
            insert_audit_event(
                &transaction,
                AuditEventDraft::new(
                    "student",
                    Some(student.id.to_string()),
                    "create",
                    format!("Created student {}", student.name),
                )
                .after_json(after_json),
            )?;
            students.push(student);
        }

        transaction.commit()?;

        Ok(students)
    }

    /// Update a student
    pub fn update(&self, id: StudentId, req: UpdateStudentRequest) -> Result<Student> {
        let card_serial = req
            .card_serial
            .map(|value| normalize_optional_text(Some(value)));
        let class_id = req
            .class_id
            .map(|value| normalize_optional_text(Some(value)));

        // Check if card serial is already registered to another student
        if let Some(Some(ref serial)) = card_serial {
            if let Some(existing) = self.find_by_card(serial)? {
                if existing.id != id {
                    return Err(AppError::CardAlreadyRegistered(serial.clone()));
                }
            }
        }

        let before = self.get(id)?;
        let mut student = before.clone();

        if let Some(name) = req.name {
            student.name = name;
        }
        if let Some(gender) = req.gender {
            student.gender = Some(gender);
        }
        if let Some(card_serial) = card_serial {
            student.card_serial = card_serial;
        }
        if let Some(class_id) = class_id {
            student.class_id = class_id;
        }

        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        transaction.execute(
            "UPDATE students 
             SET name = ?1, gender = ?2, card_serial = ?3, class_id = ?4
             WHERE id = ?5",
            params![
                student.name.as_str(),
                student.gender.map(StudentGender::as_db_value),
                student.card_serial.as_deref(),
                student.class_id.as_deref(),
                id.0.to_string(),
            ],
        )?;
        let before_json = serialize_audit_payload("student audit before payload", &before)?;
        let after_json = serialize_audit_payload("student audit after payload", &student)?;
        insert_audit_event(
            &transaction,
            AuditEventDraft::new(
                "student",
                Some(student.id.to_string()),
                "update",
                format!("Updated student {}", student.name),
            )
            .before_json(before_json)
            .after_json(after_json),
        )?;
        transaction.commit()?;

        Ok(student)
    }

    /// Delete a student and all their events
    pub fn delete(&self, id: StudentId) -> Result<()> {
        let before = self.get(id)?;
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        let deleted_events: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM events WHERE student_id = ?1",
                params![id.0.to_string()],
                |row| row.get(0),
            )
            .unwrap_or(0);

        transaction.execute(
            "DELETE FROM sf2_student_mappings WHERE student_id = ?1",
            params![id.0.to_string()],
        )?;
        transaction.execute(
            "DELETE FROM events WHERE student_id = ?1",
            params![id.0.to_string()],
        )?;
        let rows = transaction.execute(
            "DELETE FROM students WHERE id = ?1",
            params![id.0.to_string()],
        )?;

        if rows == 0 {
            return Err(AppError::StudentNotFound(id.0.to_string()));
        }

        let before_json = serialize_audit_payload("student audit before payload", &before)?;
        let metadata_json = audit_metadata(serde_json::json!({
            "deletedEvents": deleted_events,
        }))?;
        insert_audit_event(
            &transaction,
            AuditEventDraft::new(
                "student",
                Some(before.id.to_string()),
                "delete",
                format!("Deleted student {}", before.name),
            )
            .before_json(before_json)
            .metadata_json(metadata_json),
        )?;
        transaction.commit()?;
        Ok(())
    }
}
