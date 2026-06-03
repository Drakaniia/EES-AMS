/// Database infrastructure layer
use crate::domain::{
    error::{AppError, Result},
    models::*,
};
use chrono::{DateTime, Local, Utc};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension, Row};
use std::path::Path;

/// Database connection pool type
pub type DbPool = Pool<SqliteConnectionManager>;

/// Initialize the database with schema and migrations
pub fn init_db<P: AsRef<Path>>(path: P) -> Result<DbPool> {
    let manager = SqliteConnectionManager::file(path)
        .with_init(|conn| conn.execute_batch("PRAGMA foreign_keys = ON;"));
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

    if user_version < 7 {
        migrate_to_v7(&conn)?;
        conn.execute("PRAGMA user_version = 7", [])?;
    }

    if user_version < 8 {
        migrate_to_v8(&conn)?;
        conn.execute("PRAGMA user_version = 8", [])?;
    }

    if user_version < 9 {
        migrate_to_v9(&conn)?;
        conn.execute("PRAGMA user_version = 9", [])?;
    }

    if user_version < 10 {
        migrate_to_v10(&conn)?;
        conn.execute("PRAGMA user_version = 10", [])?;
    }

    if user_version < 11 {
        migrate_to_v11(&conn)?;
        conn.execute("PRAGMA user_version = 11", [])?;
    }

    if user_version < 12 {
        migrate_to_v12(&conn)?;
        conn.execute("PRAGMA user_version = 12", [])?;
    }

    if user_version < 13 {
        migrate_to_v13(&conn)?;
        conn.execute("PRAGMA user_version = 13", [])?;
    }

    if user_version < 14 {
        migrate_to_v14(&conn)?;
        conn.execute("PRAGMA user_version = 14", [])?;
    }

    Ok(pool)
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn attendance_session_key(timestamp: DateTime<Utc>, class_id: Option<&str>) -> String {
    let local_date = timestamp.with_timezone(&Local).format("%Y-%m-%d");
    let class_key = class_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unassigned");

    format!("{local_date}|{class_key}|day")
}

fn serialize_audit_event(event: &AttendanceEvent) -> Result<String> {
    serde_json::to_string(event)
        .map_err(|error| AppError::Internal(format!("failed to serialize audit event: {error}")))
}

fn attendance_event_from_row(row: &Row<'_>) -> rusqlite::Result<AttendanceEvent> {
    let updated_at = row
        .get::<_, Option<i64>>(8)?
        .and_then(|timestamp| DateTime::from_timestamp(timestamp, 0))
        .map(|timestamp| timestamp.with_timezone(&Utc));

    Ok(AttendanceEvent {
        id: EventId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
        student_id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(1)?).unwrap()),
        class_id: row.get(2)?,
        event_type: AttendanceType::In,
        timestamp: DateTime::from_timestamp(row.get::<_, i64>(4)?, 0)
            .unwrap()
            .with_timezone(&Utc),
        note: row.get(5)?,
        session_key: row.get(6)?,
        override_reason: row.get(7)?,
        updated_at,
    })
}

fn attendance_audit_entry_from_row(row: &Row<'_>) -> rusqlite::Result<AttendanceAuditEntry> {
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
            event_type TEXT NOT NULL CHECK(event_type IN ('in')),
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
            "INSERT INTO students_new (id, name, card_serial, created_at) 
             SELECT id, name, card_serial, created_at FROM students",
            [],
        )?;

        // Copy data from old events table to new one
        conn.execute(
            "INSERT INTO events_new (id, student_id, event_type, timestamp, note) 
             SELECT id, student_id, event_type, timestamp, note FROM events
             WHERE event_type = 'in'",
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
        "q1_start", "q1_end", "q2_start", "q2_end", "q3_start", "q3_end",
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

/// Migrate database to version 7 (limit active quarter to three periods)
fn migrate_to_v7(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute(
        "UPDATE settings
         SET quarter = '3rd Quarter'
         WHERE quarter NOT IN ('1st Quarter', '2nd Quarter', '3rd Quarter')",
        [],
    )?;

    Ok(())
}

/// Migrate database to version 8 (add attendance mode setting)
fn migrate_to_v8(conn: &rusqlite::Connection) -> Result<()> {
    let has_attendance_mode: bool = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('settings') WHERE name='attendance_mode'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
        > 0;

    if !has_attendance_mode {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN attendance_mode TEXT NOT NULL DEFAULT 'manual'",
            [],
        )?;
    }

    conn.execute(
        "UPDATE settings
         SET attendance_mode = 'manual'
         WHERE attendance_mode NOT IN ('manual', 'card_reader')",
        [],
    )?;

    Ok(())
}

/// Migrate database to version 9 (add DepEd SF2 workbook mappings)
fn migrate_to_v9(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sql/sf2/migrate_to_v9.sql"))?;
    Ok(())
}

/// Migrate database to version 10 (add SF2 form metadata to settings)
fn migrate_to_v10(conn: &rusqlite::Connection) -> Result<()> {
    let columns = [
        "school_id",
        "school_name",
        "school_year",
        "report_month",
        "grade_level",
        "section",
        "adviser_name",
        "school_head_name",
    ];

    for column in columns {
        let has_column: bool = conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM pragma_table_info('settings') WHERE name='{}'",
                    column
                ),
                [],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;

        if !has_column {
            conn.execute(
                &format!("ALTER TABLE settings ADD COLUMN {} TEXT", column),
                [],
            )?;
        }
    }

    Ok(())
}

/// Migrate database to version 11 (single IN attendance and no external student number)
fn migrate_to_v11(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sql/migrate_to_v11.sql"))?;
    Ok(())
}

/// Migrate database to version 12 (store SF2 metadata per workbook template)
fn migrate_to_v12(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sql/sf2/migrate_to_v12.sql"))?;
    Ok(())
}

/// Migrate database to version 13 (add student gender for SF2 roster sections)
fn migrate_to_v13(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sql/migrate_to_v13.sql"))?;
    Ok(())
}

/// Migrate database to version 14 (attendance exception workflow)
fn migrate_to_v14(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("../../sql/migrate_to_v14.sql"))?;
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
                "SELECT id, name, gender, card_serial, class_id, created_at
                 FROM students 
                 WHERE class_id = ?1 
                 ORDER BY name ASC",
            )?;
            let rows = stmt.query_map(params![class_id], |row| {
                Ok(Student {
                    id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                    name: row.get(1)?,
                    gender: StudentGender::from_db_value(
                        row.get::<_, Option<String>>(2)?.as_deref(),
                    ),
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
                "SELECT id, name, gender, card_serial, class_id, created_at
                 FROM students 
                 ORDER BY name ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(Student {
                    id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                    name: row.get(1)?,
                    gender: StudentGender::from_db_value(
                        row.get::<_, Option<String>>(2)?.as_deref(),
                    ),
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
                "SELECT id, name, gender, card_serial, class_id, created_at
                 FROM students 
                 WHERE id = ?1",
                params![id.0.to_string()],
                |row| {
                    Ok(Student {
                        id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                        name: row.get(1)?,
                        gender: StudentGender::from_db_value(
                            row.get::<_, Option<String>>(2)?.as_deref(),
                        ),
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
                "SELECT id, name, gender, card_serial, class_id, created_at
                 FROM students 
                 WHERE card_serial = ?1",
                params![serial],
                |row| {
                    Ok(Student {
                        id: StudentId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap()),
                        name: row.get(1)?,
                        gender: StudentGender::from_db_value(
                            row.get::<_, Option<String>>(2)?.as_deref(),
                        ),
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

        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO students (id, name, gender, card_serial, class_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                student.id.0.to_string(),
                student.name,
                student.gender.map(StudentGender::as_db_value),
                student.card_serial,
                student.class_id,
                student.created_at.timestamp(),
            ],
        )?;

        Ok(student)
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

        let mut student = self.get(id)?;

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

        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE students 
             SET name = ?1, gender = ?2, card_serial = ?3, class_id = ?4
             WHERE id = ?5",
            params![
                student.name,
                student.gender.map(StudentGender::as_db_value),
                student.card_serial,
                student.class_id,
                id.0.to_string(),
            ],
        )?;

        Ok(student)
    }

    /// Delete a student and all their events
    pub fn delete(&self, id: StudentId) -> Result<()> {
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;

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

        transaction.commit()?;
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
            "SELECT id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at
             FROM events 
             WHERE event_type = 'in'
             ORDER BY timestamp DESC",
        )?;

        let events = stmt
            .query_map([], attendance_event_from_row)?
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
             AND event_type = 'in'
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
                 AND event_type = 'in'
                 ORDER BY timestamp DESC 
                 LIMIT 1",
                params![student_id.0.to_string()],
                attendance_event_from_row,
            )
            .optional()?;

        Ok(event)
    }

    /// Create an attendance event
    pub fn create(&self, req: CreateEventRequest) -> Result<AttendanceEvent> {
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
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
            return Err(AppError::DuplicateAttendance(
                "Student already recorded for this session".to_string(),
            ));
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

        transaction.commit()?;
        Ok(event)
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

        if let Some(reason) = audit_reason {
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

        transaction.commit()?;
        Ok(())
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
        let mut settings = conn
            .query_row(
                "SELECT id, day_start, day_end, late_after, quarter, q1_start, q1_end, q2_start, q2_end, q3_start, q3_end, attendance_mode, school_id, school_name, school_year, report_month, grade_level, section, adviser_name, school_head_name FROM settings WHERE id = 'app'",
                [],
                |row| {
                    let attendance_mode = row.get::<_, String>(11)?;
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
                        attendance_mode: AttendanceMode::from_db(&attendance_mode),
                        school_id: row.get(12)?,
                        school_name: row.get(13)?,
                        school_year: row.get(14)?,
                        report_month: row.get(15)?,
                        grade_level: row.get(16)?,
                        section: row.get(17)?,
                        adviser_name: row.get(18)?,
                        school_head_name: row.get(19)?,
                    })
                },
            )
            .optional()?
            .unwrap_or_default();

        if !matches!(
            settings.quarter.as_str(),
            "1st Quarter" | "2nd Quarter" | "3rd Quarter"
        ) {
            settings.quarter = "3rd Quarter".to_string();
        }
        settings.attendance_mode = settings.attendance_mode.normalize();

        Ok(settings)
    }

    /// Update settings
    pub fn update(&self, settings: Settings) -> Result<Settings> {
        let mut settings = settings;
        if !matches!(
            settings.quarter.as_str(),
            "1st Quarter" | "2nd Quarter" | "3rd Quarter"
        ) {
            settings.quarter = "3rd Quarter".to_string();
        }
        settings.attendance_mode = settings.attendance_mode.normalize();

        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (id, day_start, day_end, late_after, quarter, q1_start, q1_end, q2_start, q2_end, q3_start, q3_end, attendance_mode, school_id, school_name, school_year, report_month, grade_level, section, adviser_name, school_head_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
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
                settings.attendance_mode.as_str(),
                settings.school_id,
                settings.school_name,
                settings.school_year,
                settings.report_month,
                settings.grade_level,
                settings.section,
                settings.adviser_name,
                settings.school_head_name,
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
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;

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

        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

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
