use super::*;

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
pub fn clear_audit_events(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<usize, String> {
    let repo = AuditRepository::new(pool.inner().clone());
    repo.clear().map_err(|e| e.to_string())
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
