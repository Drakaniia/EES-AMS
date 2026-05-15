/// Tauri commands
use crate::domain::models::*;
use crate::infrastructure::database::{
    ClassRepository, EventRepository, SettingsRepository, StudentRepository,
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Serialize;
use std::fs;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NfcReaderStatus {
    pub connected: bool,
    pub reader_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NfcCardData {
    pub serial_number: String,
    pub data: Option<String>,
}

// Global NFC reader state
static NFC_READER: Mutex<Option<Arc<Mutex<NfcReader>>>> = Mutex::new(None);

struct NfcReader {
    connected: bool,
}

impl NfcReader {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // For now, simulate NFC reader detection
        // In a real implementation, this would use PC/SC or USB APIs
        // Return error to simulate no reader connected
        Err("No NFC reader detected".into())
    }

    fn connect_to_reader(&mut self, _reader_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.connected = true;
        Ok(())
    }

    fn read_card(&mut self) -> Result<NfcCardData, Box<dyn std::error::Error>> {
        if !self.connected {
            return Err("NFC reader not connected".into());
        }

        // Simulate card reading
        // In a real implementation, this would read from actual NFC hardware
        Ok(NfcCardData {
            serial_number: format!(
                "{:02x}:{:02x}:{:02x}:{:02x}",
                rand::random::<u8>(),
                rand::random::<u8>(),
                rand::random::<u8>(),
                rand::random::<u8>()
            ),
            data: None,
        })
    }

    fn wait_for_card(&mut self) -> Result<NfcCardData, Box<dyn std::error::Error>> {
        // Simulate waiting for card
        std::thread::sleep(std::time::Duration::from_millis(500));
        self.read_card()
    }
}

#[tauri::command]
pub fn check_nfc_reader() -> Result<NfcReaderStatus, String> {
    match NfcReader::new() {
        Ok(_reader) => Ok(NfcReaderStatus {
            connected: _reader.connected,
            reader_name: Some("Simulated NFC Reader".to_string()),
            error: None,
        }),
        Err(e) => Ok(NfcReaderStatus {
            connected: false,
            reader_name: None,
            error: Some(format!("Failed to initialize NFC: {}", e)),
        }),
    }
}

#[tauri::command]
pub fn start_nfc_scanning() -> Result<String, String> {
    let mut reader = NfcReader::new().map_err(|e| e.to_string())?;

    reader
        .connect_to_reader("simulated_reader")
        .map_err(|e| e.to_string())?;

    // Store reader globally
    {
        let mut global_reader = NFC_READER.lock().unwrap();
        *global_reader = Some(Arc::new(Mutex::new(reader)));
    }

    Ok("Simulated NFC Reader".to_string())
}

#[tauri::command]
pub fn stop_nfc_scanning() -> Result<(), String> {
    let mut global_reader = NFC_READER.lock().unwrap();
    *global_reader = None;
    Ok(())
}

#[tauri::command]
pub fn read_nfc_card() -> Result<NfcCardData, String> {
    let global_reader = NFC_READER.lock().unwrap();
    let reader = global_reader
        .as_ref()
        .ok_or_else(|| "NFC scanner not started".to_string())?
        .clone();

    let mut reader = reader.lock().unwrap();
    reader.wait_for_card().map_err(|e| e.to_string())
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
) -> std::result::Result<(), String> {
    let event_id = EventId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());
    repo.delete(event_id).map_err(|e| e.to_string())
}

// ── Settings Commands ───────────────────────────────────────────────────────

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
    let student_repo = StudentRepository::new(pool.inner().clone());
    let class_repo = ClassRepository::new(pool.inner().clone());
    let event_repo = EventRepository::new(pool.inner().clone());
    let settings_repo = SettingsRepository::new(pool.inner().clone());

    let students = student_repo.list().map_err(|e| e.to_string())?;
    let classes = class_repo.list().map_err(|e| e.to_string())?;
    let events = event_repo.list().map_err(|e| e.to_string())?;
    let settings = vec![settings_repo.get().map_err(|e| e.to_string())?];

    Ok(ExportData {
        students,
        classes,
        events,
        settings,
        exported_at: chrono::Utc::now().timestamp(),
    })
}

#[tauri::command]
pub fn import_all(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    payload: ExportData,
) -> std::result::Result<(), String> {
    let student_repo = StudentRepository::new(pool.inner().clone());
    let class_repo = ClassRepository::new(pool.inner().clone());
    let event_repo = EventRepository::new(pool.inner().clone());
    let settings_repo = SettingsRepository::new(pool.inner().clone());

    // Import classes first (students may reference them)
    for class in payload.classes {
        let req = CreateClassRequest {
            name: class.name,
            room: class.room,
            day_start: class.day_start,
            day_end: class.day_end,
            late_after: class.late_after,
            sessions: class.sessions,
        };
        class_repo.create(req).map_err(|e| e.to_string())?;
    }

    // Import students
    for student in payload.students {
        let req = CreateStudentRequest {
            name: student.name,
            student_number: student.student_number,
            card_serial: student.card_serial,
            class_id: student.class_id,
        };
        student_repo.create(req).map_err(|e| e.to_string())?;
    }

    // Import events
    for event in payload.events {
        let req = CreateEventRequest {
            student_id: event.student_id,
            class_id: event.class_id,
            event_type: event.event_type,
            note: event.note,
        };
        event_repo.create(req).map_err(|e| e.to_string())?;
    }

    // Import settings (only the first one)
    if let Some(settings) = payload.settings.into_iter().next() {
        settings_repo.update(settings).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn wipe_all(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<(), String> {
    let conn = pool.get().map_err(|e| e.to_string())?;

    // Clear all tables
    conn.execute("DELETE FROM events", [])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM students", [])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM classes", [])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM settings", [])
        .map_err(|e| e.to_string())?;

    // Re-insert default settings
    conn.execute(
        "INSERT OR IGNORE INTO settings (id, day_start, day_end, late_after) VALUES ('app', '08:30', '15:30', '08:45')",
        []
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn export_database(
    app: tauri::AppHandle,
    _pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<String, String> {
    // Get database path
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let db_path = app_dir.join("attendance.db");

    if !db_path.exists() {
        return Err("Database file not found".to_string());
    }

    // Show save dialog to let user choose location and filename
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("SQLite Database", &["db"])
        .set_file_name(format!(
            "attendance-backup-{}.db",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ))
        .save_file(move |result| tx.send(result).unwrap());

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

    // Copy database file to chosen location
    fs::copy(&db_path, &file_path_buf).map_err(|e| format!("Failed to copy database: {}", e))?;

    Ok(file_path_buf.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_json_with_folder(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<String, String> {
    // Get export data
    let export_data = export_all(pool)?;

    // Show save dialog to let user choose location and filename
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("JSON Files", &["json"])
        .set_file_name(format!(
            "attendance-backup-{}.json",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ))
        .save_file(move |result| tx.send(result).unwrap());

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

    Ok(file_path_buf.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_csv_with_folder(
    app: tauri::AppHandle,
    events: Vec<AttendanceEvent>,
    students: Vec<Student>,
    classes: Vec<Class>,
    _global_late_after: String,
) -> std::result::Result<String, String> {
    // Import CSV generation function from frontend logic
    // We need to recreate the CSV logic here since it's in the frontend
    let mut csv_content = String::new();

    // Header
    csv_content.push_str("Date,Class,Room,Student #,Name,Check-in,Check-out,Hours,Late\n");

    // Group events by student and date
    use std::collections::HashMap;
    let mut groups: HashMap<String, (Student, Vec<AttendanceEvent>)> = HashMap::new();

    let student_map: HashMap<StudentId, Student> =
        students.into_iter().map(|s| (s.id, s)).collect();

    let class_map: HashMap<String, Class> =
        classes.into_iter().map(|c| (c.id.clone(), c)).collect();

    for event in events {
        if let Some(student) = student_map.get(&event.student_id) {
            let date = event.timestamp.format("%Y-%m-%d").to_string();
            let key = format!("{}|{}", event.student_id, date);

            let entry = groups.entry(key).or_insert((student.clone(), Vec::new()));
            entry.1.push(event);
        }
    }

    // Generate CSV rows
    for (student, events) in groups.values_mut() {
        events.sort_by_key(|a| a.timestamp);

        let mut check_in_time: Option<String> = None;
        let mut check_out_time: Option<String> = None;
        let mut duration_hours = String::new();
        let mut is_late = String::new();

        for event in &*events {
            let time_str = event.timestamp.format("%H:%M").to_string();

            if event.event_type == AttendanceType::In {
                if check_in_time.is_none() || time_str < *check_in_time.as_ref().unwrap() {
                    check_in_time = Some(time_str.clone());

                    // Check if late
                    if let Some(class) = student.class_id.as_ref().and_then(|id| class_map.get(id))
                    {
                        let event_time = event
                            .timestamp
                            .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());

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
                            .ok_or("Invalid time")?
                            .and_utc();
                        if event_time > late_time {
                            is_late = "Yes".to_string();
                        } else {
                            is_late = "No".to_string();
                        }
                    }
                }
            } else if event.event_type == AttendanceType::Out {
                check_out_time = Some(time_str.clone());
            }
        }

        // Calculate duration
        if let (Some(check_in), Some(check_out)) = (&check_in_time, &check_out_time) {
            if let (Ok(in_time), Ok(out_time)) = (
                chrono::NaiveTime::parse_from_str(check_in, "%H:%M"),
                chrono::NaiveTime::parse_from_str(check_out, "%H:%M"),
            ) {
                let duration = out_time.signed_duration_since(in_time);
                duration_hours = format!("{:.2}", duration.num_seconds() as f64 / 3600.0);
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
            .map(|e| e.timestamp.format("%Y-%m-%d").to_string())
            .unwrap_or_default();

        csv_content.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            date,
            class_name,
            room_name,
            student.student_number,
            student.name,
            check_in_time.unwrap_or_default(),
            check_out_time.unwrap_or_default(),
            duration_hours,
            is_late
        ));
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
        .save_file(move |result| tx.send(result).unwrap());

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

    Ok(file_path_buf.to_string_lossy().to_string())
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

    let updater = app.updater().map_err(|e| e.to_string())?;
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
