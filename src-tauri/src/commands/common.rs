use super::*;

pub(super) fn audit_metadata_json(value: serde_json::Value) -> std::result::Result<String, String> {
    serde_json::to_string(&value)
        .map_err(|error| format!("Failed to serialize audit metadata: {error}"))
}

pub(super) fn record_command_audit(
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

pub(super) fn collect_export_data(
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

pub(super) fn insert_imported_audit_event(
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

pub(super) fn app_data_dir(app: &tauri::AppHandle) -> std::result::Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))
}

pub(super) fn dialog_file_path_to_path_buf(
    file_path: tauri_plugin_dialog::FilePath,
) -> std::result::Result<PathBuf, String> {
    match file_path {
        tauri_plugin_dialog::FilePath::Path(path) => Ok(path),
        tauri_plugin_dialog::FilePath::Url(url) => {
            Err(format!("URL file paths not supported: {url}"))
        }
    }
}

pub(super) fn pick_folder(app: &tauri::AppHandle) -> std::result::Result<Option<PathBuf>, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |result| {
        let _ = tx.send(result);
    });

    rx.recv()
        .map_err(|e| format!("Failed to receive folder path: {e}"))?
        .map(dialog_file_path_to_path_buf)
        .transpose()
}

pub(super) fn save_file_dialog(
    app: &tauri::AppHandle,
    filter_name: &str,
    extensions: &[&str],
    default_name: String,
) -> std::result::Result<PathBuf, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter(filter_name, extensions)
        .set_file_name(default_name)
        .save_file(move |result| {
            let _ = tx.send(result);
        });

    let file_path = rx
        .recv()
        .map_err(|e| format!("Failed to receive file path: {}", e))?
        .ok_or_else(|| "User cancelled save dialog".to_string())?;

    dialog_file_path_to_path_buf(file_path)
}

pub(super) fn pick_database_file(
    app: &tauri::AppHandle,
) -> std::result::Result<Option<PathBuf>, String> {
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
