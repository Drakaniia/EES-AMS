/// Tauri commands
use crate::backup::models::{BackupPreview, BackupStatus, BackupSummary, RestoreResult};
use crate::backup::service as backup_service;
use crate::domain::models::*;
use crate::infrastructure::database::{
    record_audit_event, AuditEventInput, AuditRepository, ClassRepository, EventRepository,
    SettingsRepository, StudentRepository,
};
use crate::sf2::models::{
    Sf2CloseDaySummary, Sf2ExportPreview, Sf2ExportReadiness, Sf2ExportResult, Sf2ImportSummary,
    Sf2ImportValidation, Sf2TemplateDraft, Sf2WorkbookSettings,
};
use crate::sf2::service;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serde::Serialize;
use std::{fs, path::PathBuf};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_updater::UpdaterExt;

fn audit_metadata_json(value: serde_json::Value) -> std::result::Result<String, String> {
    serde_json::to_string(&value)
        .map_err(|error| format!("Failed to serialize audit metadata: {error}"))
}

fn record_command_audit(
    pool: &Pool<SqliteConnectionManager>,
    entity_type: &str,
    entity_id: Option<&str>,
    action: &str,
    summary: &str,
    metadata_json: Option<String>,
) -> std::result::Result<(), String> {
    AuditRepository::new(pool.clone())
        .record(AuditEventInput {
            entity_type,
            entity_id,
            action,
            summary,
            before_json: None,
            after_json: None,
            metadata_json,
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn collect_export_data(
    pool: &Pool<SqliteConnectionManager>,
) -> std::result::Result<ExportData, String> {
    let student_repo = StudentRepository::new(pool.clone());
    let class_repo = ClassRepository::new(pool.clone());
    let event_repo = EventRepository::new(pool.clone());
    let settings_repo = SettingsRepository::new(pool.clone());
    let audit_repo = AuditRepository::new(pool.clone());

    let students = student_repo.list().map_err(|e| e.to_string())?;
    let classes = class_repo.list().map_err(|e| e.to_string())?;
    let events = event_repo.list().map_err(|e| e.to_string())?;
    let settings = vec![settings_repo.get().map_err(|e| e.to_string())?];
    let audit_events = audit_repo.list_all().map_err(|e| e.to_string())?;

    Ok(ExportData {
        students,
        classes,
        events,
        settings,
        audit_events,
        exported_at: chrono::Utc::now().timestamp(),
    })
}

fn insert_imported_audit_event(
    transaction: &rusqlite::Transaction<'_>,
    event: &AuditEvent,
) -> std::result::Result<usize, String> {
    transaction
        .execute(
            "INSERT OR IGNORE INTO audit_events (id, entity_type, entity_id, action, summary, before_json, after_json, metadata_json, created_at, actor)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                event.id.as_str(),
                event.entity_type.as_str(),
                event.entity_id.as_deref(),
                event.action.as_str(),
                event.summary.as_str(),
                event.before_json.as_deref(),
                event.after_json.as_deref(),
                event.metadata_json.as_deref(),
                event.created_at.timestamp(),
                event.actor.as_str(),
            ],
        )
        .map_err(|error| error.to_string())
}

// ── Student Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn list_students(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<Vec<Student>, String> {
    let repo = StudentRepository::new(pool.inner().clone());
    repo.list_by_class(class_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<Student, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    repo.get(student_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn find_student_by_card(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    serial: String,
) -> std::result::Result<Option<Student>, String> {
    let repo = StudentRepository::new(pool.inner().clone());
    repo.find_by_card(&serial).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    req: CreateStudentRequest,
) -> std::result::Result<Student, String> {
    let repo = StudentRepository::new(pool.inner().clone());
    repo.create(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
    req: UpdateStudentRequest,
) -> std::result::Result<Student, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    repo.update(student_id, req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<(), String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    repo.delete(student_id).map_err(|e| e.to_string())
}

// ── Class Commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_classes(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<Vec<Class>, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_class(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<Option<Class>, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.get(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_class(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    req: CreateClassRequest,
) -> std::result::Result<Class, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.create(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_class(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
    req: UpdateClassRequest,
) -> std::result::Result<Class, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.update(&id, req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_class(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<(), String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.delete(&id).map_err(|e| e.to_string())
}

// ── Event Commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_events(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<Vec<AttendanceEvent>, String> {
    let repo = EventRepository::new(pool.inner().clone());
    repo.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_events_for_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    student_id: String,
) -> std::result::Result<Vec<AttendanceEvent>, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&student_id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());
    repo.list_for_student(student_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn last_event_for_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    student_id: String,
) -> std::result::Result<Option<AttendanceEvent>, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&student_id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());
    repo.last_for_student(student_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_event(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    req: CreateEventRequest,
) -> std::result::Result<AttendanceEvent, String> {
    let repo = EventRepository::new(pool.inner().clone());
    repo.create(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_event(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
    reason: Option<String>,
) -> std::result::Result<(), String> {
    let event_id = EventId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());
    repo.delete(event_id, reason).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_event(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
    req: UpdateEventRequest,
) -> std::result::Result<AttendanceEvent, String> {
    let event_id = EventId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());
    repo.update(event_id, req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_attendance_audit(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    event_id: Option<String>,
    student_id: Option<String>,
) -> std::result::Result<Vec<AttendanceAuditEntry>, String> {
    let event_id = event_id
        .map(|id| uuid::Uuid::parse_str(&id).map(EventId))
        .transpose()
        .map_err(|e| e.to_string())?;
    let student_id = student_id
        .map(|id| uuid::Uuid::parse_str(&id).map(StudentId))
        .transpose()
        .map_err(|e| e.to_string())?;
    let repo = EventRepository::new(pool.inner().clone());
    repo.list_audit(event_id, student_id)
        .map_err(|e| e.to_string())
}

// ── Settings Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn list_audit_events(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    limit: Option<i64>,
) -> std::result::Result<Vec<AuditEvent>, String> {
    let repo = AuditRepository::new(pool.inner().clone());
    repo.list(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<Settings, String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.get().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    settings: Settings,
) -> std::result::Result<Settings, String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.update(settings).map_err(|e| e.to_string())
}

// ── Export/Import Commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn export_all(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<ExportData, String> {
    let export_data = collect_export_data(pool.inner())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "format": "json",
        "students": export_data.students.len(),
        "classes": export_data.classes.len(),
        "events": export_data.events.len(),
        "settings": export_data.settings.len(),
        "auditEvents": export_data.audit_events.len(),
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        None,
        "export",
        "Exported application data snapshot",
        Some(metadata_json),
    )?;

    Ok(export_data)
}

#[tauri::command]
pub fn import_all(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    payload: ExportData,
) -> std::result::Result<(), String> {
    let ExportData {
        students,
        classes,
        events,
        settings,
        audit_events,
        exported_at,
    } = payload;
    let student_count = students.len();
    let class_count = classes.len();
    let event_count = events.len();
    let settings_count = settings.len();
    let imported_audit_count = audit_events.len();

    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let transaction = conn.transaction().map_err(|e| e.to_string())?;

    for class in classes {
        let sessions_json = serde_json::to_string(&class.sessions)
            .map_err(|e| format!("Invalid class sessions: {e}"))?;
        let days_json =
            serde_json::to_string(&class.days).map_err(|e| format!("Invalid class days: {e}"))?;

        transaction
            .execute(
                "INSERT INTO classes (id, name, room, day_start, day_end, late_after, sessions, days, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    room = excluded.room,
                    day_start = excluded.day_start,
                    day_end = excluded.day_end,
                    late_after = excluded.late_after,
                    sessions = excluded.sessions,
                    days = excluded.days,
                    created_at = excluded.created_at",
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
            )
            .map_err(|e| e.to_string())?;
    }

    for student in students {
        transaction
            .execute(
                "INSERT INTO students (id, name, gender, card_serial, class_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    gender = excluded.gender,
                    card_serial = excluded.card_serial,
                    class_id = excluded.class_id,
                    created_at = excluded.created_at",
                params![
                    student.id.0.to_string(),
                    student.name,
                    student.gender.map(StudentGender::as_db_value),
                    student.card_serial,
                    student.class_id,
                    student.created_at.timestamp(),
                ],
            )
            .map_err(|e| e.to_string())?;
    }

    for event in events {
        let session_key = event.session_key.clone().unwrap_or_else(|| {
            let local_date = event
                .timestamp
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d");
            let class_key = event
                .class_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("unassigned");
            format!("{local_date}|{class_key}|day")
        });

        transaction
            .execute(
                "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    student_id = excluded.student_id,
                    class_id = excluded.class_id,
                    event_type = excluded.event_type,
                    timestamp = excluded.timestamp,
                    note = excluded.note,
                    session_key = excluded.session_key,
                    override_reason = excluded.override_reason,
                    updated_at = excluded.updated_at",
                params![
                    event.id.0.to_string(),
                    event.student_id.0.to_string(),
                    event.class_id,
                    "in",
                    event.timestamp.timestamp(),
                    event.note,
                    session_key,
                    event.override_reason,
                    event.updated_at.map(|timestamp| timestamp.timestamp()),
                ],
            )
            .map_err(|e| e.to_string())?;
    }

    if let Some(mut settings) = settings.into_iter().next() {
        if !matches!(
            settings.quarter.as_str(),
            "1st Quarter" | "2nd Quarter" | "3rd Quarter"
        ) {
            settings.quarter = "3rd Quarter".to_string();
        }
        settings.attendance_mode = settings.attendance_mode.normalize();

        transaction
            .execute(
                "INSERT INTO settings (id, day_start, day_end, late_after, quarter, q1_start, q1_end, q2_start, q2_end, q3_start, q3_end, attendance_mode, school_id, school_name, school_year, report_month, grade_level, section, adviser_name, school_head_name)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
                 ON CONFLICT(id) DO UPDATE SET
                    day_start = excluded.day_start,
                    day_end = excluded.day_end,
                    late_after = excluded.late_after,
                    quarter = excluded.quarter,
                    q1_start = excluded.q1_start,
                    q1_end = excluded.q1_end,
                    q2_start = excluded.q2_start,
                    q2_end = excluded.q2_end,
                    q3_start = excluded.q3_start,
                    q3_end = excluded.q3_end,
                    attendance_mode = excluded.attendance_mode,
                    school_id = excluded.school_id,
                    school_name = excluded.school_name,
                    school_year = excluded.school_year,
                    report_month = excluded.report_month,
                    grade_level = excluded.grade_level,
                    section = excluded.section,
                    adviser_name = excluded.adviser_name,
                    school_head_name = excluded.school_head_name",
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
            )
            .map_err(|e| e.to_string())?;
    }

    for audit_event in &audit_events {
        insert_imported_audit_event(&transaction, audit_event)?;
    }

    let metadata_json = audit_metadata_json(serde_json::json!({
        "sourceExportedAt": exported_at,
        "students": student_count,
        "classes": class_count,
        "events": event_count,
        "settings": settings_count,
        "importedAuditEvents": imported_audit_count,
    }))?;
    record_audit_event(
        &transaction,
        AuditEventInput {
            entity_type: "data_import",
            entity_id: None,
            action: "import",
            summary: "Imported JSON backup merge",
            before_json: None,
            after_json: None,
            metadata_json: Some(metadata_json),
        },
    )
    .map_err(|e| e.to_string())?;

    transaction.commit().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn wipe_all(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let transaction = conn.transaction().map_err(|e| e.to_string())?;
    let student_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM students", [], |row| row.get(0))
        .unwrap_or(0);
    let class_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM classes", [], |row| row.get(0))
        .unwrap_or(0);
    let event_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
        .unwrap_or(0);
    let settings_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
        .unwrap_or(0);
    let attendance_audit_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM attendance_event_audit", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);
    let sf2_template_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM sf2_templates", [], |row| row.get(0))
        .unwrap_or(0);

    // Clear all tables
    transaction
        .execute("DELETE FROM attendance_event_audit", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM sf2_student_mappings", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM sf2_date_mappings", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM attendance_day_status", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM sf2_templates", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM events", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM students", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM classes", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM settings", [])
        .map_err(|e| e.to_string())?;

    // Re-insert default settings
    transaction.execute(
        "INSERT OR IGNORE INTO settings (id, day_start, day_end, late_after, quarter, attendance_mode) VALUES ('app', '08:30', '15:30', '08:45', '1st Quarter', 'manual')",
        []
    ).map_err(|e| e.to_string())?;

    let metadata_json = audit_metadata_json(serde_json::json!({
        "students": student_count,
        "classes": class_count,
        "events": event_count,
        "settings": settings_count,
        "attendanceAuditEvents": attendance_audit_count,
        "sf2Templates": sf2_template_count,
    }))?;
    record_audit_event(
        &transaction,
        AuditEventInput {
            entity_type: "database",
            entity_id: None,
            action: "wipe",
            summary: "Wiped all application data",
            before_json: None,
            after_json: None,
            metadata_json: Some(metadata_json),
        },
    )
    .map_err(|e| e.to_string())?;

    transaction.commit().map_err(|e| e.to_string())
}

fn app_data_dir(app: &tauri::AppHandle) -> std::result::Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))
}

fn dialog_file_path_to_path_buf(
    file_path: tauri_plugin_dialog::FilePath,
) -> std::result::Result<PathBuf, String> {
    match file_path {
        tauri_plugin_dialog::FilePath::Path(path) => Ok(path),
        tauri_plugin_dialog::FilePath::Url(url) => {
            Err(format!("URL file paths not supported: {url}"))
        }
    }
}

fn pick_folder(app: &tauri::AppHandle) -> std::result::Result<Option<PathBuf>, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |result| {
        let _ = tx.send(result);
    });

    rx.recv()
        .map_err(|e| format!("Failed to receive folder path: {e}"))?
        .map(dialog_file_path_to_path_buf)
        .transpose()
}

fn pick_database_file(app: &tauri::AppHandle) -> std::result::Result<Option<PathBuf>, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("SQLite Database", &["db"])
        .pick_file(move |result| {
            let _ = tx.send(result);
        });

    rx.recv()
        .map_err(|e| format!("Failed to receive file path: {e}"))?
        .map(dialog_file_path_to_path_buf)
        .transpose()
}

#[tauri::command]
pub async fn export_database(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<String, String> {
    // Show save dialog to let user choose location and filename
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("SQLite Database", &["db"])
        .set_file_name(format!(
            "attendance-backup-{}.db",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ))
        .save_file(move |result| {
            let _ = tx.send(result);
        });

    let file_path = rx
        .recv()
        .map_err(|e| format!("Failed to receive file path: {}", e))?
        .ok_or_else(|| "User cancelled save dialog".to_string())?;

    // Convert FilePath to PathBuf
    let file_path_buf = match file_path {
        tauri_plugin_dialog::FilePath::Path(path) => path,
        tauri_plugin_dialog::FilePath::Url(url) => {
            return Err(format!("URL file paths not supported: {}", url));
        }
    };

    backup_service::backup_database_to_path(pool.inner(), &file_path_buf)
        .map_err(|e| e.to_string())?;

    let metadata_json = audit_metadata_json(serde_json::json!({
        "format": "sqlite",
        "path": file_path_buf.to_string_lossy(),
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        None,
        "export",
        "Exported SQLite database",
        Some(metadata_json),
    )?;

    Ok(file_path_buf.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_backup_status(app: tauri::AppHandle) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    backup_service::get_status(&app_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_backup_now(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    let status =
        backup_service::create_manual_backup(pool.inner(), &app_dir).map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "path": status.last_backup_path.as_deref(),
        "syncFolderPath": status.sync_folder_path.as_deref(),
        "googleDriveConnected": status.google_drive_connected,
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        None,
        "backup",
        "Created manual database backup",
        Some(metadata_json),
    )?;
    Ok(status)
}

#[tauri::command]
pub fn list_backups(app: tauri::AppHandle) -> std::result::Result<Vec<BackupSummary>, String> {
    let app_dir = app_data_dir(&app)?;
    backup_service::list_backups(&app_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn choose_backup_sync_folder(
    app: tauri::AppHandle,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    let Some(folder_path) = pick_folder(&app)? else {
        return backup_service::get_status(&app_dir).map_err(|e| e.to_string());
    };

    backup_service::set_sync_folder(&app_dir, Some(folder_path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_backup_sync_folder(
    app: tauri::AppHandle,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    backup_service::set_sync_folder(&app_dir, None).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn connect_google_drive_backup(
    app: tauri::AppHandle,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    backup_service::connect_google_drive(&app_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn disconnect_google_drive_backup(
    app: tauri::AppHandle,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    backup_service::disconnect_google_drive(&app_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upload_latest_backup_to_google_drive(
    app: tauri::AppHandle,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    backup_service::upload_latest_backup_to_google_drive(&app_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn choose_restore_backup(
    app: tauri::AppHandle,
) -> std::result::Result<Option<BackupPreview>, String> {
    let Some(file_path) = pick_database_file(&app)? else {
        return Ok(None);
    };

    backup_service::preview_backup(&file_path)
        .map(Some)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_backup(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    source_path: String,
) -> std::result::Result<RestoreResult, String> {
    let app_dir = app_data_dir(&app)?;
    let source_path = PathBuf::from(source_path);
    let result = backup_service::restore_backup(pool.inner(), &app_dir, &source_path)
        .map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "sourcePath": source_path.to_string_lossy(),
        "preRestoreBackupPath": result.pre_restore_backup_path.as_str(),
        "schemaVersion": result.schema_version,
        "migrated": result.migrated,
    }))?;
    record_command_audit(
        pool.inner(),
        "database",
        None,
        "restore",
        "Restored database backup",
        Some(metadata_json),
    )?;
    Ok(result)
}

#[tauri::command]
pub async fn export_json_with_folder(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<String, String> {
    // Get export data
    let export_data = collect_export_data(pool.inner())?;

    // Show save dialog to let user choose location and filename
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("JSON Files", &["json"])
        .set_file_name(format!(
            "attendance-backup-{}.json",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ))
        .save_file(move |result| {
            let _ = tx.send(result);
        });

    let file_path = rx
        .recv()
        .map_err(|e| format!("Failed to receive file path: {}", e))?
        .ok_or_else(|| "User cancelled save dialog".to_string())?;

    // Convert FilePath to PathBuf
    let file_path_buf = match file_path {
        tauri_plugin_dialog::FilePath::Path(path) => path,
        tauri_plugin_dialog::FilePath::Url(url) => {
            return Err(format!("URL file paths not supported: {}", url));
        }
    };

    // Write JSON data to file
    let json_content = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize data: {}", e))?;

    fs::write(&file_path_buf, json_content).map_err(|e| format!("Failed to write file: {}", e))?;

    let metadata_json = audit_metadata_json(serde_json::json!({
        "format": "json",
        "path": file_path_buf.to_string_lossy(),
        "students": export_data.students.len(),
        "classes": export_data.classes.len(),
        "events": export_data.events.len(),
        "settings": export_data.settings.len(),
        "auditEvents": export_data.audit_events.len(),
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        None,
        "export",
        "Exported JSON backup",
        Some(metadata_json),
    )?;

    Ok(file_path_buf.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_csv_with_folder(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    events: Vec<AttendanceEvent>,
    students: Vec<Student>,
    classes: Vec<Class>,
    _global_late_after: String,
) -> std::result::Result<String, String> {
    // Import CSV generation function from frontend logic
    // We need to recreate the CSV logic here since it's in the frontend
    let event_count = events.len();
    let student_count = students.len();
    let class_count = classes.len();
    let mut csv_content = String::new();

    // Header
    csv_content.push_str("Date,Class,Room,Name,IN,Late\n");

    // Group events by student and date
    use std::collections::HashMap;
    let mut groups: HashMap<String, (Student, Vec<AttendanceEvent>)> = HashMap::new();

    let student_map: HashMap<StudentId, Student> =
        students.into_iter().map(|s| (s.id, s)).collect();

    let class_map: HashMap<String, Class> =
        classes.into_iter().map(|c| (c.id.clone(), c)).collect();

    for event in events {
        if let Some(student) = student_map.get(&event.student_id) {
            let date = event
                .timestamp
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d")
                .to_string();
            let key = format!("{}|{}", event.student_id, date);

            let entry = groups.entry(key).or_insert((student.clone(), Vec::new()));
            entry.1.push(event);
        }
    }

    // Generate CSV rows
    for (student, events) in groups.values_mut() {
        events.sort_by_key(|a| a.timestamp);

        let mut check_in_time: Option<String> = None;
        let mut is_late = String::new();

        for event in &*events {
            let event_time = event.timestamp.with_timezone(&chrono::Local);
            let time_str = event_time.format("%H:%M").to_string();

            let is_earliest_check_in = match &check_in_time {
                Some(current_check_in) => time_str < *current_check_in,
                None => true,
            };

            if is_earliest_check_in {
                check_in_time = Some(time_str.clone());

                // Check if late
                if let Some(class) = student.class_id.as_ref().and_then(|id| class_map.get(id)) {
                    // Find matching session or use default
                    let mut late_after = &class.late_after;
                    let time_str = event_time.format("%H:%M").to_string();

                    for session in &class.sessions {
                        if time_str >= session.start_time && time_str <= session.end_time {
                            late_after = &session.late_after;
                            break;
                        }
                    }

                    let parts: Vec<&str> = late_after.split(':').collect();
                    let [h, m] = [
                        parts
                            .first()
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0),
                        parts
                            .get(1)
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0),
                    ];
                    let late_time = event_time
                        .date_naive()
                        .and_hms_opt(h, m, 0)
                        .and_then(|time| time.and_local_timezone(chrono::Local).earliest())
                        .ok_or("Invalid time")?;
                    if event_time > late_time {
                        is_late = "Yes".to_string();
                    } else {
                        is_late = "No".to_string();
                    }
                }
            }
        }

        let class_name = student
            .class_id
            .as_ref()
            .and_then(|id| class_map.get(id))
            .map(|c| c.name.as_str())
            .unwrap_or("Unknown");

        let room_name = student
            .class_id
            .as_ref()
            .and_then(|id| class_map.get(id))
            .and_then(|c| c.room.as_deref())
            .unwrap_or("N/A");

        let date = events
            .first()
            .map(|e| {
                e.timestamp
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d")
                    .to_string()
            })
            .unwrap_or_default();

        push_csv_row(
            &mut csv_content,
            &[
                date,
                class_name.to_string(),
                room_name.to_string(),
                student.name.clone(),
                check_in_time.unwrap_or_default(),
                is_late,
            ],
        );
    }

    // Show save dialog to let user choose location and filename
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("CSV Files", &["csv"])
        .set_file_name(format!(
            "attendance-records-{}.csv",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ))
        .save_file(move |result| {
            let _ = tx.send(result);
        });

    let file_path = rx
        .recv()
        .map_err(|e| format!("Failed to receive file path: {}", e))?
        .ok_or_else(|| "User cancelled save dialog".to_string())?;

    // Convert FilePath to PathBuf
    let file_path_buf = match file_path {
        tauri_plugin_dialog::FilePath::Path(path) => path,
        tauri_plugin_dialog::FilePath::Url(url) => {
            return Err(format!("URL file paths not supported: {}", url));
        }
    };

    // Write CSV data to file
    fs::write(&file_path_buf, csv_content).map_err(|e| format!("Failed to write file: {}", e))?;

    let metadata_json = audit_metadata_json(serde_json::json!({
        "format": "csv",
        "path": file_path_buf.to_string_lossy(),
        "events": event_count,
        "students": student_count,
        "classes": class_count,
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        None,
        "export",
        "Exported attendance CSV",
        Some(metadata_json),
    )?;

    Ok(file_path_buf.to_string_lossy().to_string())
}

// ── SF2 Excel Bridge Commands ───────────────────────────────────────────────

fn push_csv_row(output: &mut String, fields: &[String]) {
    let row = fields
        .iter()
        .map(|field| escape_csv_field(field))
        .collect::<Vec<_>>()
        .join(",");
    output.push_str(&row);
    output.push('\n');
}

fn escape_csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[tauri::command]
pub async fn validate_sf2_workbook_import(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<Sf2ImportValidation, String> {
    let validation =
        service::validate_workbook_import(app, pool.inner().clone()).map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "sourcePath": validation.source_path.as_str(),
        "classId": validation.class_id.as_deref(),
        "className": validation.class_name.as_str(),
        "currentStudentCount": validation.current_student_count,
        "sf2LearnerCount": validation.sf2_learner_count,
        "missingFromSf2": validation.missing_from_sf2.len(),
        "missingFromCurrent": validation.missing_from_current.len(),
        "possibleNameMismatches": validation.possible_name_mismatches.len(),
        "duplicateCurrentStudents": validation.duplicate_current_students.len(),
        "duplicateSf2Learners": validation.duplicate_sf2_learners.len(),
        "missingLearnerInfo": validation.missing_learner_info.len(),
        "hasDiscrepancies": validation.has_discrepancies,
    }))?;
    record_command_audit(
        pool.inner(),
        "sf2_workbook",
        None,
        "validate",
        "Validated SF2 workbook import",
        Some(metadata_json),
    )?;
    Ok(validation)
}

#[tauri::command]
pub async fn import_sf2_workbook(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    source_path: String,
    proceed_anyway: bool,
) -> std::result::Result<Sf2ImportSummary, String> {
    let summary = service::import_workbook(
        app,
        pool.inner().clone(),
        source_path.clone(),
        proceed_anyway,
    )
    .map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "classId": summary.class_id.as_str(),
        "className": summary.class_name.as_str(),
        "selectedSourcePath": source_path.as_str(),
        "sourcePath": summary.source_path.as_str(),
        "learnersFound": summary.learners_found,
        "studentsCreated": summary.students_created,
        "studentsReused": summary.students_reused,
        "datesMapped": summary.dates_mapped,
        "proceedAnyway": proceed_anyway,
    }))?;
    record_command_audit(
        pool.inner(),
        "sf2_workbook",
        Some(summary.template_id.as_str()),
        "import",
        "Imported SF2 workbook",
        Some(metadata_json),
    )?;
    Ok(summary)
}

#[tauri::command]
pub async fn create_sf2_workbook_from_template(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    draft: Sf2TemplateDraft,
) -> std::result::Result<Sf2ImportSummary, String> {
    let summary = service::create_workbook_from_template(app, pool.inner().clone(), draft)
        .map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "classId": summary.class_id.as_str(),
        "className": summary.class_name.as_str(),
        "sourcePath": summary.source_path.as_str(),
        "learnersFound": summary.learners_found,
        "studentsCreated": summary.students_created,
        "studentsReused": summary.students_reused,
        "datesMapped": summary.dates_mapped,
    }))?;
    record_command_audit(
        pool.inner(),
        "sf2_workbook",
        Some(summary.template_id.as_str()),
        "create",
        "Created SF2 workbook from template",
        Some(metadata_json),
    )?;
    Ok(summary)
}

#[tauri::command]
pub fn get_sf2_workbook_settings(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<Sf2WorkbookSettings, String> {
    service::workbook_settings(pool.inner().clone(), class_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_sf2_workbook_settings(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    draft: Sf2TemplateDraft,
) -> std::result::Result<Sf2ImportSummary, String> {
    let summary = service::update_workbook_settings(pool.inner().clone(), draft)
        .map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "classId": summary.class_id.as_str(),
        "className": summary.class_name.as_str(),
        "sourcePath": summary.source_path.as_str(),
        "learnersFound": summary.learners_found,
        "studentsCreated": summary.students_created,
        "studentsReused": summary.students_reused,
        "datesMapped": summary.dates_mapped,
    }))?;
    record_command_audit(
        pool.inner(),
        "sf2_workbook",
        Some(summary.template_id.as_str()),
        "update",
        "Updated SF2 workbook settings",
        Some(metadata_json),
    )?;
    Ok(summary)
}

#[tauri::command]
pub fn close_sf2_attendance_day(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: String,
    date: Option<String>,
) -> std::result::Result<Sf2CloseDaySummary, String> {
    let summary =
        service::close_day(pool.inner().clone(), class_id, date).map_err(|e| e.to_string())?;
    let entity_id = format!("{}:{}", summary.class_id, summary.date);
    let metadata_json = audit_metadata_json(serde_json::json!({
        "classId": summary.class_id.as_str(),
        "date": summary.date.as_str(),
        "presentCount": summary.present_count,
        "absentCount": summary.absent_count,
    }))?;
    record_command_audit(
        pool.inner(),
        "attendance_day_status",
        Some(entity_id.as_str()),
        "update",
        "Closed SF2 attendance day",
        Some(metadata_json),
    )?;
    Ok(summary)
}

#[tauri::command]
pub fn get_sf2_export_readiness(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<Sf2ExportReadiness, String> {
    service::export_readiness(pool.inner().clone(), class_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_sf2_export_preview(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<Sf2ExportPreview, String> {
    service::export_preview(pool.inner().clone(), class_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_sf2_preview_attendance(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: String,
    student_id: String,
    date: String,
    present: bool,
) -> std::result::Result<Sf2ExportPreview, String> {
    service::set_preview_attendance(pool.inner().clone(), class_id, student_id, date, present)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_sf2_workbook(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: String,
) -> std::result::Result<Sf2ExportResult, String> {
    let result = service::export_workbook(app, pool.inner().clone(), class_id.clone())
        .map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "classId": class_id.as_str(),
        "outputPath": result.output_path.as_str(),
        "marksWritten": result.marks_written,
        "closedDays": result.closed_days,
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        Some(class_id.as_str()),
        "export",
        "Exported SF2 workbook",
        Some(metadata_json),
    )?;
    Ok(result)
}

#[tauri::command]
pub fn open_sf2_workbook(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<String, String> {
    service::open_workbook(pool.inner().clone(), class_id).map_err(|e| e.to_string())
}

// ── Updater Commands ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub current_version: String,
}

#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current_version = app.package_info().version.to_string();

    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(error) => {
            log::debug!("updater unavailable: {error}");
            return Ok(UpdateInfo {
                available: false,
                version: None,
                notes: None,
                pub_date: None,
                current_version,
            });
        }
    };
    match updater.check().await.map_err(|e| e.to_string())? {
        Some(update) => Ok(UpdateInfo {
            available: true,
            version: Some(update.version.clone()),
            notes: update.body.clone(),
            pub_date: update.date.map(|d| d.to_string()),
            current_version,
        }),
        None => Ok(UpdateInfo {
            available: false,
            version: None,
            notes: None,
            pub_date: None,
            current_version,
        }),
    }
}

#[tauri::command]
pub async fn download_and_install(app: tauri::AppHandle) -> Result<String, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update available".to_string())?;

    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .map_err(|e| e.to_string())?;

    Ok("Update installed. The app will restart shortly.".to_string())
}
