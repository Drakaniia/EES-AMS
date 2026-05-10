/// Tauri commands
use crate::domain::models::*;
use crate::infrastructure::database::{
    ClassRepository, EventRepository, SettingsRepository, StudentRepository,
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub local_ip: String,
    pub port: u16,
    pub url: String,
}

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
pub fn get_server_info() -> ServerInfo {
    let local_ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let port = crate::DEFAULT_PORT;
    let url = format!("http://{}:{}", local_ip, port);

    ServerInfo {
        local_ip,
        port,
        url,
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
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<Vec<Student>, String> {
    let repo = StudentRepository::new(pool.inner().clone());
    repo.list_by_class(class_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_student(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<Student, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    repo.get(student_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn find_student_by_card(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    serial: String,
) -> std::result::Result<Option<Student>, String> {
    let repo = StudentRepository::new(pool.inner().clone());
    repo.find_by_card(&serial).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_student(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    req: CreateStudentRequest,
) -> std::result::Result<Student, String> {
    let repo = StudentRepository::new(pool.inner().clone());
    repo.create(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_student(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    id: String,
    req: UpdateStudentRequest,
) -> std::result::Result<Student, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    repo.update(student_id, req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_student(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<(), String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    repo.delete(student_id).map_err(|e| e.to_string())
}

// ── Class Commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_classes(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
) -> std::result::Result<Vec<Class>, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_class(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<Option<Class>, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.get(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_class(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    req: CreateClassRequest,
) -> std::result::Result<Class, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.create(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_class(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    id: String,
    req: UpdateClassRequest,
) -> std::result::Result<Class, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.update(&id, req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_class(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<(), String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.delete(&id).map_err(|e| e.to_string())
}

// ── Event Commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_events(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
) -> std::result::Result<Vec<AttendanceEvent>, String> {
    let repo = EventRepository::new(pool.inner().clone());
    repo.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_events_for_student(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    student_id: String,
) -> std::result::Result<Vec<AttendanceEvent>, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&student_id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());
    repo.list_for_student(student_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn last_event_for_student(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    student_id: String,
) -> std::result::Result<Option<AttendanceEvent>, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&student_id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());
    repo.last_for_student(student_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_event(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    req: CreateEventRequest,
) -> std::result::Result<AttendanceEvent, String> {
    let repo = EventRepository::new(pool.inner().clone());
    repo.create(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_event(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<(), String> {
    let event_id = EventId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());
    repo.delete(event_id).map_err(|e| e.to_string())
}

// ── Settings Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn get_settings(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
) -> std::result::Result<Settings, String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.get().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
    settings: Settings,
) -> std::result::Result<Settings, String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.update(settings).map_err(|e| e.to_string())
}

// ── Export/Import Commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn export_all(
    pool: tauri::State<Pool<SqliteConnectionManager>>,
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
    pool: tauri::State<Pool<SqliteConnectionManager>>,
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
            day_start: class.day_start,
            day_end: class.day_end,
            late_after: class.late_after,
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
    pool: tauri::State<Pool<SqliteConnectionManager>>,
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
