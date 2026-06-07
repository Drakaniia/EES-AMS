use super::*;

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
