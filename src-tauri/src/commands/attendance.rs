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
pub fn list_events_for_date(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    date: String,
) -> std::result::Result<Vec<AttendanceEvent>, String> {
    let repo = EventRepository::new(pool.inner().clone());
    repo.list_for_local_date(&date).map_err(|e| e.to_string())
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
    let event = repo.create(req).map_err(|e| e.to_string())?;

    // Run SF2 Excel sync in background thread so the UI doesn't block on file I/O
    if let Some(ref class_id) = event.class_id {
        let pool = pool.inner().clone();
        let class_id = class_id.clone();
        std::thread::spawn(move || {
            if let Err(e) = crate::sf2::service::sync_attendance_to_sf2_workbook(
                pool,
                &class_id,
            ) {
                log::warn!("SF2 workbook sync failed after adding attendance event: {e}");
            }
        });
    }

    Ok(event)
}

#[tauri::command]
pub fn add_events(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    reqs: Vec<CreateEventRequest>,
) -> std::result::Result<Vec<AttendanceEvent>, String> {
    let repo = EventRepository::new(pool.inner().clone());
    let events = repo.create_many(reqs).map_err(|e| e.to_string())?;

    let class_ids: std::collections::HashSet<&str> = events
        .iter()
        .filter_map(|event| event.class_id.as_deref())
        .collect();
    for class_id in class_ids {
        let pool = pool.inner().clone();
        let class_id = class_id.to_string();
        std::thread::spawn(move || {
            if let Err(e) = crate::sf2::service::sync_attendance_to_sf2_workbook(
                pool,
                &class_id,
            ) {
                log::warn!("SF2 workbook sync failed after adding batch attendance events: {e}");
            }
        });
    }

    Ok(events)
}

#[tauri::command]
pub fn delete_events(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    ids: Vec<String>,
    reason: Option<String>,
) -> std::result::Result<(), String> {
    let repo = EventRepository::new(pool.inner().clone());
    let event_ids: Vec<EventId> = ids
        .into_iter()
        .filter_map(|id| uuid::Uuid::parse_str(&id).ok().map(EventId))
        .collect();

    // Collect class IDs before deleting so we can sync once per class
    let class_ids: std::collections::HashSet<String> = event_ids
        .iter()
        .filter_map(|event_id| repo.get(*event_id).ok())
        .filter_map(|event| event.class_id)
        .collect();

    repo.delete_many(&event_ids, reason)
        .map_err(|e| e.to_string())?;

    // Run SF2 Excel sync once per affected class on a background thread
    for class_id in class_ids {
        let pool = pool.inner().clone();
        std::thread::spawn(move || {
            if let Err(e) = crate::sf2::service::sync_attendance_to_sf2_workbook(
                pool,
                &class_id,
            ) {
                log::warn!("SF2 workbook sync failed after deleting attendance events: {e}");
            }
        });
    }

    Ok(())
}

#[tauri::command]
pub fn delete_event(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
    reason: Option<String>,
) -> std::result::Result<(), String> {
    let event_id = EventId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());

    // Look up event to get the class_id before deleting it
    let class_id = repo.get(event_id).ok().and_then(|event| event.class_id);

    repo.delete(event_id, reason).map_err(|e| e.to_string())?;

    // Run SF2 Excel sync in background thread so the UI doesn't block on file I/O
    if let Some(ref class_id) = class_id {
        let pool = pool.inner().clone();
        let class_id = class_id.clone();
        std::thread::spawn(move || {
            if let Err(e) = crate::sf2::service::sync_attendance_to_sf2_workbook(
                pool,
                &class_id,
            ) {
                log::warn!("SF2 workbook sync failed after deleting attendance event: {e}");
            }
        });
    }

    Ok(())
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
