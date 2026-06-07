use super::*;

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
