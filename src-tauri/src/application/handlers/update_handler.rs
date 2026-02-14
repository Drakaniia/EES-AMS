#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub date: String,
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub body: Option<String>,
}

#[tauri::command]
pub async fn check_for_updates(app_handle: tauri::AppHandle) -> Result<UpdateStatus, String> {
    match app_handle.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => Ok(UpdateStatus {
                    available: true,
                    current_version: update.current_version.clone(),
                    latest_version: Some(update.version.clone()),
                    body: update.body.clone(),
                }),
                Ok(None) => Ok(UpdateStatus {
                    available: false,
                    current_version: env!("CARGO_PKG_VERSION").to_string(),
                    latest_version: None,
                    body: None,
                }),
                Err(e) => Err(format!("Failed to check for updates: {}", e)),
            }
        },
        Err(e) => Err(format!("Failed to initialize updater: {}", e)),
    }
}

#[tauri::command]
pub async fn download_and_install_update(app_handle: tauri::AppHandle) -> Result<String, String> {
    match app_handle.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => {
                    let app_handle_clone = app_handle.clone();
                    // Download and install the update with progress tracking
                    match update.download_and_install(
                        move |bytes_downloaded, total_size| {
                            let progress = format!("Downloaded {} bytes of {:?}", bytes_downloaded, total_size);
                            // Emit progress event to frontend
                            let _ = app_handle_clone.emit("update-progress", progress);
                        },
                        || {
                            // on_download_finish callback
                        }
                    ).await {
                        Ok(_) => {
                            Ok("Update downloaded and installed successfully".to_string())
                        },
                        Err(e) => {
                            Err(format!("Failed to download/install update: {}", e))
                        }
                    }
                },
                Ok(None) => Err("No update available".to_string()),
                Err(e) => Err(format!("Failed to check for updates: {}", e)),
            }
        },
        Err(e) => Err(format!("Failed to initialize updater: {}", e)),
    }
}

#[tauri::command]
pub async fn restart_app(app_handle: tauri::AppHandle) -> Result<String, String> {
    app_handle.restart();
    #[allow(unreachable_code)]
    Ok("Restarting application...".to_string())
}