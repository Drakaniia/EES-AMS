//! SQLite database infrastructure.
mod audit;
mod classes;
mod events;
mod migrations;
mod rows;
mod settings;
mod students;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

pub use audit::{record_audit_event, AuditEventInput, AuditRepository};
pub use classes::ClassRepository;
pub use events::EventRepository;
pub use migrations::{init_db, migrate_db, CURRENT_SCHEMA_VERSION};
pub use settings::SettingsRepository;
pub use students::StudentRepository;

/// Database connection pool type
pub type DbPool = Pool<SqliteConnectionManager>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{error::AppError, models::*};
    use chrono::{TimeZone, Utc};
    use rusqlite::params;

    #[test]
    fn init_db_creates_in_only_schema_without_external_student_numbers() {
        // Arrange
        let temp_db = tempfile::NamedTempFile::new().expect("test database file should be created");

        // Act
        let pool = init_db(temp_db.path()).expect("database should initialize");
        let conn = pool.get().expect("database connection should be available");

        let student_number_columns: i32 = conn
            .query_row(
                "SELECT count(*) FROM pragma_table_info('students') WHERE name='student_number'",
                [],
                |row| row.get(0),
            )
            .expect("students schema should be inspectable");
        let out_insert = conn.execute(
            "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note)
             VALUES (?1, ?2, NULL, 'out', 0, NULL)",
            params![
                uuid::Uuid::new_v4().to_string(),
                uuid::Uuid::new_v4().to_string()
            ],
        );

        // Assert
        assert_eq!(student_number_columns, 0);
        assert!(out_insert.is_err());
    }

    #[test]
    fn init_db_enables_foreign_keys_for_pooled_connections() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database file should be created");
        let pool = init_db(temp_db.path()).expect("database should initialize");
        let conn = pool.get().expect("database connection should be available");

        let foreign_keys_enabled: i32 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("foreign key pragma should be inspectable");

        assert_eq!(foreign_keys_enabled, 1);
    }

    #[test]
    fn student_gender_is_persisted_and_updated() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database file should be created");
        let pool = init_db(temp_db.path()).expect("database should initialize");
        let student_repo = StudentRepository::new(pool);

        let student = student_repo
            .create(CreateStudentRequest {
                name: "Ada Lovelace".to_string(),
                gender: Some(StudentGender::Female),
                card_serial: None,
                class_id: None,
            })
            .expect("student should be created");
        assert_eq!(student.gender, Some(StudentGender::Female));

        let updated = student_repo
            .update(
                student.id,
                UpdateStudentRequest {
                    name: None,
                    gender: Some(StudentGender::Male),
                    card_serial: None,
                    class_id: None,
                },
            )
            .expect("student should be updated");

        assert_eq!(updated.gender, Some(StudentGender::Male));
        assert_eq!(
            student_repo
                .get(student.id)
                .expect("student should be readable")
                .gender,
            Some(StudentGender::Male)
        );
    }

    #[test]
    fn init_db_creates_general_audit_events_table() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database file should be created");
        let pool = init_db(temp_db.path()).expect("database should initialize");
        let conn = pool.get().expect("database connection should be available");

        let audit_table_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'audit_events'",
                [],
                |row| row.get(0),
            )
            .expect("audit table should be inspectable");
        let schema_version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("schema version should be readable");

        assert_eq!(audit_table_count, 1);
        assert_eq!(schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn student_lifecycle_writes_general_audit_events() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database file should be created");
        let pool = init_db(temp_db.path()).expect("database should initialize");
        let student_repo = StudentRepository::new(pool.clone());
        let audit_repo = AuditRepository::new(pool);

        let student = student_repo
            .create(CreateStudentRequest {
                name: "Ada Lovelace".to_string(),
                gender: Some(StudentGender::Female),
                card_serial: Some("CARD-001".to_string()),
                class_id: None,
            })
            .expect("student should be created");
        student_repo
            .update(
                student.id,
                UpdateStudentRequest {
                    name: Some("Ada Byron".to_string()),
                    gender: Some(StudentGender::Female),
                    card_serial: Some("CARD-002".to_string()),
                    class_id: None,
                },
            )
            .expect("student should be updated");
        student_repo
            .delete(student.id)
            .expect("student should be deleted");

        let audit_events = audit_repo
            .list(Some(10))
            .expect("audit events should be readable");
        let student_events: Vec<&AuditEvent> = audit_events
            .iter()
            .filter(|event| {
                event.entity_type == "student"
                    && event.entity_id.as_deref() == Some(&student.id.to_string())
            })
            .collect();
        let actions: Vec<&str> = student_events
            .iter()
            .map(|event| event.action.as_str())
            .collect();

        assert!(actions.contains(&"create"));
        assert!(actions.contains(&"update"));
        assert!(actions.contains(&"delete"));
        assert!(student_events.iter().any(|event| event
            .after_json
            .as_deref()
            .is_some_and(|json| json.contains("Ada Lovelace"))));
        assert!(student_events.iter().any(|event| event
            .before_json
            .as_deref()
            .is_some_and(|json| json.contains("Ada Byron"))));
    }

    #[test]
    fn deleting_student_removes_attendance_events() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database file should be created");
        let pool = init_db(temp_db.path()).expect("database should initialize");
        let student_repo = StudentRepository::new(pool.clone());
        let event_repo = EventRepository::new(pool.clone());

        let student = student_repo
            .create(CreateStudentRequest {
                name: "Ada Lovelace".to_string(),
                gender: None,
                card_serial: None,
                class_id: None,
            })
            .expect("student should be created");
        event_repo
            .create(CreateEventRequest {
                student_id: student.id,
                class_id: None,
                event_type: AttendanceType::In,
                note: None,
                session_key: None,
                override_reason: None,
                timestamp: None,
            })
            .expect("event should be created");

        student_repo
            .delete(student.id)
            .expect("student should be deleted");

        let conn = pool.get().expect("database connection should be available");
        let event_count: i32 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .expect("event count should be readable");
        assert_eq!(event_count, 0);
    }

    #[test]
    fn duplicate_attendance_is_scoped_to_session_key_and_override_is_audited() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database file should be created");
        let pool = init_db(temp_db.path()).expect("database should initialize");
        let student_repo = StudentRepository::new(pool.clone());
        let event_repo = EventRepository::new(pool);

        let student = student_repo
            .create(CreateStudentRequest {
                name: "Katherine Johnson".to_string(),
                gender: None,
                card_serial: None,
                class_id: None,
            })
            .expect("student should be created");
        let timestamp = Utc
            .with_ymd_and_hms(2026, 6, 3, 8, 0, 0)
            .single()
            .expect("test timestamp should be valid");

        event_repo
            .create(CreateEventRequest {
                student_id: student.id,
                class_id: Some("class-a".to_string()),
                event_type: AttendanceType::In,
                note: None,
                session_key: Some("2026-06-03|class-a|morning".to_string()),
                override_reason: None,
                timestamp: Some(timestamp),
            })
            .expect("first session event should be created");
        event_repo
            .create(CreateEventRequest {
                student_id: student.id,
                class_id: Some("class-a".to_string()),
                event_type: AttendanceType::In,
                note: None,
                session_key: Some("2026-06-03|class-a|afternoon".to_string()),
                override_reason: None,
                timestamp: Some(timestamp),
            })
            .expect("different session event should be allowed");

        let duplicate = event_repo.create(CreateEventRequest {
            student_id: student.id,
            class_id: Some("class-a".to_string()),
            event_type: AttendanceType::In,
            note: None,
            session_key: Some("2026-06-03|class-a|morning".to_string()),
            override_reason: None,
            timestamp: Some(timestamp),
        });
        assert!(matches!(duplicate, Err(AppError::DuplicateAttendance(_))));

        let override_event = event_repo
            .create(CreateEventRequest {
                student_id: student.id,
                class_id: Some("class-a".to_string()),
                event_type: AttendanceType::In,
                note: None,
                session_key: Some("2026-06-03|class-a|morning".to_string()),
                override_reason: Some("Substitute class exception".to_string()),
                timestamp: Some(timestamp),
            })
            .expect("override event should be created");
        let audit = event_repo
            .list_audit(Some(override_event.id), None)
            .expect("audit should be readable");

        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].action, "create_override");
        assert_eq!(audit[0].reason, "Substitute class exception");
    }

    #[test]
    fn updating_and_deleting_attendance_events_writes_audit_entries() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database file should be created");
        let pool = init_db(temp_db.path()).expect("database should initialize");
        let student_repo = StudentRepository::new(pool.clone());
        let event_repo = EventRepository::new(pool);

        let student = student_repo
            .create(CreateStudentRequest {
                name: "Dorothy Vaughan".to_string(),
                gender: None,
                card_serial: None,
                class_id: None,
            })
            .expect("student should be created");
        let timestamp = Utc
            .with_ymd_and_hms(2026, 6, 3, 8, 0, 0)
            .single()
            .expect("test timestamp should be valid");
        let event = event_repo
            .create(CreateEventRequest {
                student_id: student.id,
                class_id: Some("class-a".to_string()),
                event_type: AttendanceType::In,
                note: None,
                session_key: Some("2026-06-03|class-a|day".to_string()),
                override_reason: None,
                timestamp: Some(timestamp),
            })
            .expect("event should be created");

        let updated = event_repo
            .update(
                event.id,
                UpdateEventRequest {
                    class_id: Some("class-b".to_string()),
                    note: Some("Corrected".to_string()),
                    session_key: Some("2026-06-03|class-b|day".to_string()),
                    timestamp: Some(timestamp),
                    reason: "Mistaken class selection".to_string(),
                },
            )
            .expect("event should be updated");
        event_repo
            .delete(updated.id, Some("Mistaken tap".to_string()))
            .expect("event should be deleted");

        let audit = event_repo
            .list_audit(None, Some(student.id))
            .expect("audit should be readable");
        let actions: Vec<&str> = audit.iter().map(|entry| entry.action.as_str()).collect();

        assert!(actions.contains(&"update"));
        assert!(actions.contains(&"delete"));
    }

    #[test]
    fn deleting_class_clears_student_and_event_references() {
        let temp_db = tempfile::NamedTempFile::new().expect("test database file should be created");
        let pool = init_db(temp_db.path()).expect("database should initialize");
        let class_repo = ClassRepository::new(pool.clone());
        let student_repo = StudentRepository::new(pool.clone());
        let event_repo = EventRepository::new(pool.clone());

        let class = class_repo
            .create(CreateClassRequest {
                name: "Grade 1 - A".to_string(),
                room: Some("101".to_string()),
                day_start: "08:00".to_string(),
                day_end: "15:00".to_string(),
                late_after: "08:15".to_string(),
                sessions: vec![Session {
                    name: "Full Day".to_string(),
                    start_time: "08:00".to_string(),
                    end_time: "15:00".to_string(),
                    late_after: "08:15".to_string(),
                }],
                days: vec![1, 2, 3, 4, 5],
            })
            .expect("class should be created");
        let student = student_repo
            .create(CreateStudentRequest {
                name: "Grace Hopper".to_string(),
                gender: None,
                card_serial: None,
                class_id: Some(class.id.clone()),
            })
            .expect("student should be created");
        let event = event_repo
            .create(CreateEventRequest {
                student_id: student.id,
                class_id: Some(class.id.clone()),
                event_type: AttendanceType::In,
                note: None,
                session_key: None,
                override_reason: None,
                timestamp: None,
            })
            .expect("event should be created");

        class_repo
            .delete(&class.id)
            .expect("class should be deleted");

        let student = student_repo
            .get(student.id)
            .expect("student should still exist");
        let event = event_repo
            .last_for_student(event.student_id)
            .expect("event lookup should succeed")
            .expect("event should still exist");
        assert_eq!(student.class_id, None);
        assert_eq!(event.class_id, None);
    }
}
