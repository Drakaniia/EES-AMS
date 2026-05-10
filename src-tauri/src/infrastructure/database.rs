/// Database infrastructure layer
use crate::domain::{
    error::{AppError, Result},
    models::*,
};
use chrono::{DateTime, Utc};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use std::path::Path;

/// Database connection pool type
pub type DbPool = Pool<SqliteConnectionManager>;

/// Initialize the database with schema
pub fn init_db<P: AsRef<Path>>(path: P) -> Result<DbPool> {
    let manager = SqliteConnectionManager::file(path);
    let pool = Pool::new(manager)?;

    let conn = pool.get()?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS students (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            student_number TEXT NOT NULL UNIQUE,
            card_serial TEXT UNIQUE,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_students_card ON students(card_serial);
        CREATE INDEX IF NOT EXISTS idx_students_name ON students(name);

        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY NOT NULL,
            student_id TEXT NOT NULL,
            event_type TEXT NOT NULL CHECK(event_type IN ('in', 'out')),
            timestamp TEXT NOT NULL,
            note TEXT,
            FOREIGN KEY (student_id) REFERENCES students(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_events_student ON events(student_id);
        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);

        CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            class_name TEXT NOT NULL,
            day_start TEXT NOT NULL,
            day_end TEXT NOT NULL,
            late_after TEXT NOT NULL
        );

        INSERT OR IGNORE INTO settings (id, class_name, day_start, day_end, late_after)
        VALUES (1, 'My Class', '08:30', '15:30', '08:45');
        "#,
    )?;

    Ok(pool)
}

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
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, student_number, card_serial, created_at 
             FROM students 
             ORDER BY name ASC",
        )?;

        let students = stmt
            .query_map([], |row| {
                Ok(Student {
                    id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                    name: row.get(1)?,
                    student_number: row.get(2)?,
                    card_serial: row.get(3)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(students)
    }

    /// Get student by ID
    pub fn get(&self, id: StudentId) -> Result<Student> {
        let conn = self.pool.get()?;
        let student = conn
            .query_row(
                "SELECT id, name, student_number, card_serial, created_at 
                 FROM students 
                 WHERE id = ?1",
                params![id.0.to_string()],
                |row| {
                    Ok(Student {
                        id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                        name: row.get(1)?,
                        student_number: row.get(2)?,
                        card_serial: row.get(3)?,
                        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                            .unwrap()
                            .with_timezone(&Utc),
                    })
                },
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
                "SELECT id, name, student_number, card_serial, created_at 
                 FROM students 
                 WHERE card_serial = ?1",
                params![serial],
                |row| {
                    Ok(Student {
                        id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                        name: row.get(1)?,
                        student_number: row.get(2)?,
                        card_serial: row.get(3)?,
                        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                            .unwrap()
                            .with_timezone(&Utc),
                    })
                },
            )
            .optional()?;

        Ok(student)
    }

    /// Create a new student
    pub fn create(&self, req: CreateStudentRequest) -> Result<Student> {
        // Check if card serial is already registered
        if let Some(ref serial) = req.card_serial {
            if self.find_by_card(serial)?.is_some() {
                return Err(AppError::CardAlreadyRegistered(serial.clone()));
            }
        }

        let student = Student {
            id: StudentId::new(),
            name: req.name,
            student_number: req.student_number,
            card_serial: req.card_serial,
            created_at: Utc::now(),
        };

        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO students (id, name, student_number, card_serial, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                student.id.0.to_string(),
                student.name,
                student.student_number,
                student.card_serial,
                student.created_at.to_rfc3339(),
            ],
        )?;

        Ok(student)
    }

    /// Update a student
    pub fn update(&self, id: StudentId, req: UpdateStudentRequest) -> Result<Student> {
        // Check if card serial is already registered to another student
        if let Some(ref serial) = req.card_serial {
            if let Some(existing) = self.find_by_card(serial)? {
                if existing.id != id {
                    return Err(AppError::CardAlreadyRegistered(serial.clone()));
                }
            }
        }

        let mut student = self.get(id)?;

        if let Some(name) = req.name {
            student.name = name;
        }
        if let Some(student_number) = req.student_number {
            student.student_number = student_number;
        }
        if let Some(card_serial) = req.card_serial {
            student.card_serial = Some(card_serial);
        }

        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE students 
             SET name = ?1, student_number = ?2, card_serial = ?3 
             WHERE id = ?4",
            params![
                student.name,
                student.student_number,
                student.card_serial,
                id.0.to_string(),
            ],
        )?;

        Ok(student)
    }

    /// Delete a student and all their events
    pub fn delete(&self, id: StudentId) -> Result<()> {
        let conn = self.pool.get()?;
        let rows = conn.execute(
            "DELETE FROM students WHERE id = ?1",
            params![id.0.to_string()],
        )?;

        if rows == 0 {
            return Err(AppError::StudentNotFound(id.0.to_string()));
        }

        Ok(())
    }
}

/// Event repository
pub struct EventRepository {
    pool: DbPool,
}

impl EventRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// List all events
    pub fn list(&self) -> Result<Vec<AttendanceEvent>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, student_id, event_type, timestamp, note 
             FROM events 
             ORDER BY timestamp DESC",
        )?;

        let events = stmt
            .query_map([], |row| {
                Ok(AttendanceEvent {
                    id: EventId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                    student_id: StudentId(
                        uuid::Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    ),
                    event_type: match row.get::<_, String>(2)?.as_str() {
                        "in" => AttendanceType::In,
                        "out" => AttendanceType::Out,
                        _ => unreachable!(),
                    },
                    timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    note: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// List events for a specific student
    pub fn list_for_student(&self, student_id: StudentId) -> Result<Vec<AttendanceEvent>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, student_id, event_type, timestamp, note 
             FROM events 
             WHERE student_id = ?1 
             ORDER BY timestamp DESC",
        )?;

        let events = stmt
            .query_map(params![student_id.0.to_string()], |row| {
                Ok(AttendanceEvent {
                    id: EventId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                    student_id: StudentId(
                        uuid::Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    ),
                    event_type: match row.get::<_, String>(2)?.as_str() {
                        "in" => AttendanceType::In,
                        "out" => AttendanceType::Out,
                        _ => unreachable!(),
                    },
                    timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    note: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// Get last event for a student
    pub fn last_for_student(&self, student_id: StudentId) -> Result<Option<AttendanceEvent>> {
        let conn = self.pool.get()?;
        let event = conn
            .query_row(
                "SELECT id, student_id, event_type, timestamp, note 
                 FROM events 
                 WHERE student_id = ?1 
                 ORDER BY timestamp DESC 
                 LIMIT 1",
                params![student_id.0.to_string()],
                |row| {
                    Ok(AttendanceEvent {
                        id: EventId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                        student_id: StudentId(
                            uuid::Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                        ),
                        event_type: match row.get::<_, String>(2)?.as_str() {
                            "in" => AttendanceType::In,
                            "out" => AttendanceType::Out,
                            _ => unreachable!(),
                        },
                        timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                            .unwrap()
                            .with_timezone(&Utc),
                        note: row.get(4)?,
                    })
                },
            )
            .optional()?;

        Ok(event)
    }

    /// Create an attendance event
    pub fn create(&self, req: CreateEventRequest) -> Result<AttendanceEvent> {
        let event = AttendanceEvent {
            id: EventId::new(),
            student_id: req.student_id,
            event_type: req.event_type,
            timestamp: Utc::now(),
            note: req.note,
        };

        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO events (id, student_id, event_type, timestamp, note) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                event.id.0.to_string(),
                event.student_id.0.to_string(),
                match event.event_type {
                    AttendanceType::In => "in",
                    AttendanceType::Out => "out",
                },
                event.timestamp.to_rfc3339(),
                event.note,
            ],
        )?;

        Ok(event)
    }

    /// Delete an event
    pub fn delete(&self, id: EventId) -> Result<()> {
        let conn = self.pool.get()?;
        let rows = conn.execute(
            "DELETE FROM events WHERE id = ?1",
            params![id.0.to_string()],
        )?;

        if rows == 0 {
            return Err(AppError::EventNotFound(id.0.to_string()));
        }

        Ok(())
    }
}

/// Settings repository
pub struct SettingsRepository {
    pool: DbPool,
}

impl SettingsRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get settings
    pub fn get(&self) -> Result<Settings> {
        let conn = self.pool.get()?;
        let settings = conn.query_row(
            "SELECT class_name, day_start, day_end, late_after FROM settings WHERE id = 1",
            [],
            |row| {
                Ok(Settings {
                    class_name: row.get(0)?,
                    day_start: row.get(1)?,
                    day_end: row.get(2)?,
                    late_after: row.get(3)?,
                })
            },
        )?;

        Ok(settings)
    }

    /// Update settings
    pub fn update(&self, settings: Settings) -> Result<Settings> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE settings 
             SET class_name = ?1, day_start = ?2, day_end = ?3, late_after = ?4 
             WHERE id = 1",
            params![
                settings.class_name,
                settings.day_start,
                settings.day_end,
                settings.late_after,
            ],
        )?;

        Ok(settings)
    }
}
