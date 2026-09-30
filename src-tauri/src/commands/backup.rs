use super::*;

#[tauri::command]
pub fn get_backup_status(app: tauri::AppHandle) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    backup_service::get_status(&app_dir).map_err(chain)
}

#[tauri::command]
pub fn create_backup_now(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    let status = backup_service::create_manual_backup(pool.inner(), &app_dir).map_err(chain)?;
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
    backup_service::list_backups(&app_dir).map_err(chain)
}

/// D13 "Back up workbooks now": copies the SF2 workbooks into their own backup
/// folder without duplicating the database.
#[tauri::command]
pub fn create_workbooks_backup_now(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    let status = backup_service::create_workbooks_backup(&app_dir).map_err(chain)?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "path": status.last_workbooks_backup_path.as_deref(),
        "syncFolderPath": status.sync_folder_path.as_deref(),
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        None,
        "workbooks_backup",
        "Created manual SF2 workbooks backup",
        Some(metadata_json),
    )?;
    Ok(status)
}

#[tauri::command]
pub fn open_backup_folder(app: tauri::AppHandle) -> std::result::Result<String, String> {
    let app_dir = app_data_dir(&app)?;
    let status = backup_service::get_status(&app_dir).map_err(chain)?;
    let backup_dir = PathBuf::from(status.local_backup_dir);

    fs::create_dir_all(&backup_dir)
        .map_err(|error| format!("Failed to create backup folder: {error}"))?;
    open::that(&backup_dir).map_err(|error| format!("Failed to open backup folder: {error}"))?;

    Ok(backup_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn choose_backup_sync_folder(
    app: tauri::AppHandle,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    let Some(folder_path) = pick_folder(&app)? else {
        return backup_service::get_status(&app_dir).map_err(chain);
    };

    backup_service::set_sync_folder(&app_dir, Some(folder_path)).map_err(chain)
}

#[tauri::command]
pub fn clear_backup_sync_folder(
    app: tauri::AppHandle,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    backup_service::set_sync_folder(&app_dir, None).map_err(chain)
}

#[tauri::command]
pub fn connect_google_drive_backup(
    app: tauri::AppHandle,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    backup_service::connect_google_drive(&app_dir).map_err(chain)
}

#[tauri::command]
pub fn disconnect_google_drive_backup(
    app: tauri::AppHandle,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    backup_service::disconnect_google_drive(&app_dir).map_err(chain)
}

#[tauri::command]
pub fn upload_latest_backup_to_google_drive(
    app: tauri::AppHandle,
) -> std::result::Result<BackupStatus, String> {
    let app_dir = app_data_dir(&app)?;
    backup_service::upload_latest_backup_to_google_drive(&app_dir).map_err(chain)
}

/// Let the user pick a backup to restore.
///
/// Picks a backup *folder* — the shape every backup written today has, and the
/// only one that carries the SF2 workbooks. `preview_backup` also accepts the
/// legacy flat `*.db` shape; [`choose_restore_database_file`] is the picker
/// for those.
#[tauri::command]
pub async fn choose_restore_backup(
    app: tauri::AppHandle,
) -> std::result::Result<Option<BackupPreview>, String> {
    let Some(folder_path) = pick_folder(&app)? else {
        return Ok(None);
    };

    backup_service::preview_backup(&folder_path)
        .map(Some)
        .map_err(chain)
}

/// Let the user pick a legacy flat `*.db` backup written by the previous build.
#[tauri::command]
pub async fn choose_restore_database_file(
    app: tauri::AppHandle,
) -> std::result::Result<Option<BackupPreview>, String> {
    let Some(file_path) = pick_database_file(&app)? else {
        return Ok(None);
    };

    backup_service::preview_backup(&file_path)
        .map(Some)
        .map_err(chain)
}

#[tauri::command]
pub fn restore_backup(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    source_path: String,
) -> std::result::Result<RestoreResult, String> {
    let app_dir = app_data_dir(&app)?;
    let source_path = PathBuf::from(source_path);
    let result =
        backup_service::restore_backup(pool.inner(), &app_dir, &source_path).map_err(chain)?;
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
