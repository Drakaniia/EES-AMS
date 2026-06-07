use super::*;

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
