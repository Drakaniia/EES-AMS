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

/// Initialize the database with schema and migrations
pub fn init_db<P: AsRef<Path>>(path: P) -> Result<DbPool> {
    let manager = SqliteConnectionManager::file(path);
    let pool = Pool::new(manager)?;

    let conn = pool.get()?;

    // Check if we need to run migrations
    let user_version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(0);

    if user_version < 1 {
        // Initial schema creation or migration to version 1
        migrate_to_v1(&conn)?;
        conn.execute("PRAGMA user_version = 1", [])?;
    }

    if user_version < 2 {
        migrate_to_v2(&conn)?;
        conn.execute("PRAGMA user_version = 2", [])?;
    }

    if user_version < 3 {
        migrate_to_v3(&conn)?;
        conn.execute("PRAGMA user_version = 3", [])?;
    }

    if user_version < 4 {
        migrate_to_v4(&conn)?;
        conn.execute("PRAGMA user_version = 4", [])?;
    }

    if user_version < 5 {
        migrate_to_v5(&conn)?;
        conn.execute("PRAGMA user_version = 5", [])?;
    }

    if user_version < 6 {
        migrate_to_v6(&conn)?;
        conn.execute("PRAGMA user_version = 6", [])?;
    }

    Ok(pool)
}

/// Migrate database to version 1 (add class support)
fn migrate_to_v1(conn: &rusqlite::Connection) -> Result<()> {
    // Create all tables with proper schema
    conn.execute_batch(
        r#"
        -- Create classes table
        CREATE TABLE IF NOT EXISTS classes (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            day_start TEXT NOT NULL,
            day_end TEXT NOT NULL,
            late_after TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        -- Create indexes for classes
        CREATE INDEX IF NOT EXISTS idx_classes_name ON classes(name);

        -- Create students table with class support
        CREATE TABLE IF NOT EXISTS students_new (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            student_number TEXT NOT NULL UNIQUE,
            card_serial TEXT UNIQUE,
            class_id TEXT,
            created_at INTEGER NOT NULL
        );

        -- Create indexes for students
        CREATE INDEX IF NOT EXISTS idx_students_card_new ON students_new(card_serial);
        CREATE INDEX IF NOT EXISTS idx_students_name_new ON students_new(name);
        CREATE INDEX IF NOT EXISTS idx_students_class_new ON students_new(class_id);

        -- Create events table with class support
        CREATE TABLE IF NOT EXISTS events_new (
            id TEXT PRIMARY KEY NOT NULL,
            student_id TEXT NOT NULL,
            class_id TEXT,
            event_type TEXT NOT NULL CHECK(event_type IN ('in', 'out')),
            timestamp INTEGER NOT NULL,
            note TEXT,
            FOREIGN KEY (student_id) REFERENCES students_new(id) ON DELETE CASCADE
        );

        -- Create indexes for events
        CREATE INDEX IF NOT EXISTS idx_events_student_new ON events_new(student_id);
        CREATE INDEX IF NOT EXISTS idx_events_timestamp_new ON events_new(timestamp);

        -- Create settings table
        CREATE TABLE IF NOT EXISTS settings (
            id TEXT PRIMARY KEY NOT NULL,
            day_start TEXT NOT NULL,
            day_end TEXT NOT NULL,
            late_after TEXT NOT NULL,
            quarter TEXT NOT NULL DEFAULT '1st Quarter'
        );

        -- Insert default settings
        INSERT OR IGNORE INTO settings (id, day_start, day_end, late_after, quarter)
        VALUES ('app', '08:30', '15:30', '08:45', '1st Quarter');
        "#,
    )?;

    // Migrate data from old tables if they exist
    let has_old_students = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='students'")
        .and_then(|mut stmt| stmt.query_row([], |_| Ok(true)))
        .unwrap_or(false);

    if has_old_students {
        // Copy data from old students table to new one
        conn.execute(
            "INSERT INTO students_new (id, name, student_number, card_serial, created_at) 
             SELECT id, name, student_number, card_serial, created_at FROM students",
            [],
        )?;

        // Copy data from old events table to new one
        conn.execute(
            "INSERT INTO events_new (id, student_id, event_type, timestamp, note) 
             SELECT id, student_id, event_type, timestamp, note FROM events",
            [],
        )?;

        // Drop old tables
        conn.execute("DROP TABLE IF EXISTS students", [])?;
        conn.execute("DROP TABLE IF EXISTS events", [])?;

        // Rename new tables
        conn.execute("ALTER TABLE students_new RENAME TO students", [])?;
        conn.execute("ALTER TABLE events_new RENAME TO events", [])?;

        // Rename indexes
        conn.execute("DROP INDEX IF EXISTS idx_students_card_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_students_name_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_students_class_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_events_student_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_events_timestamp_new", [])?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_card ON students(card_serial)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_name ON students(name)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_class ON students(class_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_student ON events(student_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp)",
            [],
        )?;
    } else {
        // No old data, just rename the new tables
        conn.execute("ALTER TABLE students_new RENAME TO students", [])?;
        conn.execute("ALTER TABLE events_new RENAME TO events", [])?;

        // Rename indexes
        conn.execute("DROP INDEX IF EXISTS idx_students_card_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_students_name_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_students_class_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_events_student_new", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_events_timestamp_new", [])?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_card ON students(card_serial)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_name ON students(name)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_students_class ON students(class_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_student ON events(student_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp)",
            [],
        )?;
    }

    Ok(())
}

/// Migrate database to version 2 (add room to classes)
fn migrate_to_v2(conn: &rusqlite::Connection) -> Result<()> {
    // Check if room column exists
    let has_room: bool = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('classes') WHERE name='room'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_room {
        conn.execute(
            "ALTER TABLE classes ADD COLUMN room TEXT NOT NULL DEFAULT 'N/A'",
            [],
        )?;
    }
    Ok(())
}

/// Migrate database to version 3 (add quarter to settings)
fn migrate_to_v3(conn: &rusqlite::Connection) -> Result<()> {
    // Check if quarter column exists
    let has_quarter: bool = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('settings') WHERE name='quarter'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_quarter {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN quarter TEXT NOT NULL DEFAULT '1st Quarter'",
            [],
        )?;
    }
    Ok(())
}

/// Migrate database to version 4 (add quarter dates to settings)
fn migrate_to_v4(conn: &rusqlite::Connection) -> Result<()> {
    let columns = [
        "q1_start", "q1_end", "q2_start", "q2_end", "q3_start", "q3_end", "q4_start", "q4_end",
    ];

    for col in columns {
        let has_col: bool = conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM pragma_table_info('settings') WHERE name='{}'",
                    col
                ),
                [],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;

        if !has_col {
            conn.execute(&format!("ALTER TABLE settings ADD COLUMN {} TEXT", col), [])?;
        }
    }
    Ok(())
}

/// Migrate database to version 5 (add sessions to classes)
fn migrate_to_v5(conn: &rusqlite::Connection) -> Result<()> {
    // Check if sessions column exists
    let has_sessions: bool = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('classes') WHERE name='sessions'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_sessions {
        conn.execute("ALTER TABLE classes ADD COLUMN sessions TEXT", [])?;

        // Initialize sessions for existing classes based on day_start, day_end, late_after
        let mut stmt = conn.prepare("SELECT id, day_start, day_end, late_after FROM classes")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        for row in rows {
            let (id, day_start, day_end, late_after) = row?;
            let sessions = vec![Session {
                name: "Full Day".to_string(),
                start_time: day_start,
                end_time: day_end,
                late_after,
            }];
            let sessions_json =
                serde_json::to_string(&sessions).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "UPDATE classes SET sessions = ?1 WHERE id = ?2",
                params![sessions_json, id],
            )?;
        }
    }
    Ok(())
}

/// Migrate database to version 6 (add days to classes)
fn migrate_to_v6(conn: &rusqlite::Connection) -> Result<()> {
    // Check if days column exists
    let has_days: bool = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('classes') WHERE name='days'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_days {
        conn.execute("ALTER TABLE classes ADD COLUMN days TEXT", [])?;

        // Initialize days for existing classes to Monday-Friday [1, 2, 3, 4, 5]
        let days = vec![1, 2, 3, 4, 5];
        let days_json = serde_json::to_string(&days).unwrap_or_else(|_| "[]".to_string());
        conn.execute("UPDATE classes SET days = ?1", params![days_json])?;
    }
    Ok(())
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
        self.list_by_class(None)
    }

    /// List students by class
    pub fn list_by_class(&self, class_id: Option<&str>) -> Result<Vec<Student>> {
        let conn = self.pool.get()?;
        let students = if let Some(class_id) = class_id {
            let mut stmt = conn.prepare(
                "SELECT id, name, student_number, card_serial, class_id, created_at 
                 FROM students 
                 WHERE class_id = ?1 
                 ORDER BY name ASC",
            )?;
            let rows = stmt.query_map(params![class_id], |row| {
                Ok(Student {
                    id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                    name: row.get(1)?,
                    student_number: row.get(2)?,
                    card_serial: row.get(3)?,
                    class_id: row.get(4)?,
                    created_at: DateTime::from_timestamp(row.get::<_, i64>(5)?, 0)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, name, student_number, card_serial, class_id, created_at 
                 FROM students 
                 ORDER BY name ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(Student {
                    id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                    name: row.get(1)?,
                    student_number: row.get(2)?,
                    card_serial: row.get(3)?,
                    class_id: row.get(4)?,
                    created_at: DateTime::from_timestamp(row.get::<_, i64>(5)?, 0)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };

        Ok(students)
    }

    /// Get student by ID
    pub fn get(&self, id: StudentId) -> Result<Student> {
        let conn = self.pool.get()?;
        let student = conn
            .query_row(
                "SELECT id, name, student_number, card_serial, class_id, created_at 
                 FROM students 
                 WHERE id = ?1",
                params![id.0.to_string()],
                |row| {
                    Ok(Student {
                        id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                        name: row.get(1)?,
                        student_number: row.get(2)?,
                        card_serial: row.get(3)?,
                        class_id: row.get(4)?,
                        created_at: DateTime::from_timestamp(row.get::<_, i64>(5)?, 0)
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
                "SELECT id, name, student_number, card_serial, class_id, created_at 
                 FROM students 
                 WHERE card_serial = ?1",
                params![serial],
                |row| {
                    Ok(Student {
                        id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                        name: row.get(1)?,
                        student_number: row.get(2)?,
                        card_serial: row.get(3)?,
                        class_id: row.get(4)?,
                        created_at: DateTime::from_timestamp(row.get::<_, i64>(5)?, 0)
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
            class_id: req.class_id,
            created_at: Utc::now(),
        };

        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO students (id, name, student_number, card_serial, class_id, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                student.id.0.to_string(),
                student.name,
                student.student_number,
                student.card_serial,
                student.class_id,
                student.created_at.timestamp(),
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
        if let Some(class_id) = req.class_id {
            student.class_id = Some(class_id);
        }

        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE students 
             SET name = ?1, student_number = ?2, card_serial = ?3, class_id = ?4 
             WHERE id = ?5",
            params![
                student.name,
                student.student_number,
                student.card_serial,
                student.class_id,
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
            "SELECT id, student_id, class_id, event_type, timestamp, note 
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
                    class_id: row.get(2)?,
                    event_type: match row.get::<_, String>(3)?.as_str() {
                        "in" => AttendanceType::In,
                        "out" => AttendanceType::Out,
                        _ => unreachable!(),
                    },
                    timestamp: DateTime::from_timestamp(row.get::<_, i64>(4)?, 0)
                        .unwrap()
                        .with_timezone(&Utc),
                    note: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// List events for a specific student
    pub fn list_for_student(&self, student_id: StudentId) -> Result<Vec<AttendanceEvent>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, student_id, class_id, event_type, timestamp, note 
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
                    class_id: row.get(2)?,
                    event_type: match row.get::<_, String>(3)?.as_str() {
                        "in" => AttendanceType::In,
                        "out" => AttendanceType::Out,
                        _ => unreachable!(),
                    },
                    timestamp: DateTime::from_timestamp(row.get::<_, i64>(4)?, 0)
                        .unwrap()
                        .with_timezone(&Utc),
                    note: row.get(5)?,
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
                "SELECT id, student_id, class_id, event_type, timestamp, note 
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
                        class_id: row.get(2)?,
                        event_type: match row.get::<_, String>(3)?.as_str() {
                            "in" => AttendanceType::In,
                            "out" => AttendanceType::Out,
                            _ => unreachable!(),
                        },
                        timestamp: DateTime::from_timestamp(row.get::<_, i64>(4)?, 0)
                            .unwrap()
                            .with_timezone(&Utc),
                        note: row.get(5)?,
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
            class_id: req.class_id,
            event_type: req.event_type,
            timestamp: Utc::now(),
            note: req.note,
        };

        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id.0.to_string(),
                event.student_id.0.to_string(),
                event.class_id,
                match event.event_type {
                    AttendanceType::In => "in",
                    AttendanceType::Out => "out",
                },
                event.timestamp.timestamp(),
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
        let settings = conn
            .query_row(
                "SELECT id, day_start, day_end, late_after, quarter, q1_start, q1_end, q2_start, q2_end, q3_start, q3_end, q4_start, q4_end FROM settings WHERE id = 'app'",
                [],
                |row| {
                    Ok(Settings {
                        id: row.get(0)?,
                        day_start: row.get(1)?,
                        day_end: row.get(2)?,
                        late_after: row.get(3)?,
                        quarter: row.get(4)?,
                        q1_start: row.get(5)?,
                        q1_end: row.get(6)?,
                        q2_start: row.get(7)?,
                        q2_end: row.get(8)?,
                        q3_start: row.get(9)?,
                        q3_end: row.get(10)?,
                        q4_start: row.get(11)?,
                        q4_end: row.get(12)?,
                    })
                },
            )
            .optional()?;

        Ok(settings.unwrap_or_default())
    }

    /// Update settings
    pub fn update(&self, settings: Settings) -> Result<Settings> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (id, day_start, day_end, late_after, quarter, q1_start, q1_end, q2_start, q2_end, q3_start, q3_end, q4_start, q4_end) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                settings.id,
                settings.day_start,
                settings.day_end,
                settings.late_after,
                settings.quarter,
                settings.q1_start,
                settings.q1_end,
                settings.q2_start,
                settings.q2_end,
                settings.q3_start,
                settings.q3_end,
                settings.q4_start,
                settings.q4_end,
            ],
        )?;

        Ok(settings)
    }
}

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
            .query_map([], |row| {
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
            })?
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
                |row| {
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
                },
            )
            .optional()?;

        Ok(class)
    }

    /// Create a new class
    pub fn create(&self, req: CreateClassRequest) -> Result<Class> {
        let room = req.room.filter(|r| !r.trim().is_empty());
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

        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO classes (id, name, room, day_start, day_end, late_after, sessions, days, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                class.id,
                class.name,
                class.room,
                class.day_start,
                class.day_end,
                class.late_after,
                sessions_json,
                days_json,
                class.created_at.timestamp(),
            ],
        )?;

        Ok(class)
    }

    /// Update a class
    pub fn update(&self, id: &str, req: UpdateClassRequest) -> Result<Class> {
        let mut class = self
            .get(id)?
            .ok_or_else(|| AppError::ClassNotFound(id.to_string()))?;

        if let Some(name) = req.name {
            class.name = name;
        }
        if let Some(room) = req.room {
            // Empty string clears the room
            class.room = if room.trim().is_empty() {
                None
            } else {
                Some(room)
            };
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

        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE classes 
             SET name = ?1, room = ?2, day_start = ?3, day_end = ?4, late_after = ?5, sessions = ?6, days = ?7 
             WHERE id = ?8",
            params![
                class.name,
                class.room,
                class.day_start,
                class.day_end,
                class.late_after,
                sessions_json,
                days_json,
                id,
            ],
        )?;

        Ok(class)
    }

    /// Delete a class
    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let rows = conn.execute("DELETE FROM classes WHERE id = ?1", params![id])?;

        if rows == 0 {
            return Err(AppError::ClassNotFound(id.to_string()));
        }

        Ok(())
    }
}
