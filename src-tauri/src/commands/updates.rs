use super::*;

use parking_lot::Mutex;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;
use tauri_plugin_updater::Update;

// ── Updater State ───────────────────────────────────────────────────────────

/// Managed state for the update lifecycle.
pub struct UpdateState {
    /// In-session update handle used by a staged install.
    pub update: Mutex<Option<Update>>,
    /// Abort handle for the active download; used by `cancel_update_download`.
    pub download: Mutex<Option<tokio::task::AbortHandle>>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            update: Mutex::new(None),
            download: Mutex::new(None),
        }
    }
}

/// Marker persisted next to the downloaded installer so a staged update
/// survives app restarts.
#[derive(Serialize, Deserialize)]
struct StagedMarker {
    version: String,
    notes: Option<String>,
    pub_date: Option<String>,
    file: String,
}

fn staged_dir(app: &tauri::AppHandle) -> std::result::Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("Failed to resolve cache directory: {error}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Failed to create cache directory: {error}"))?;
    Ok(dir)
}

fn staged_marker_path(app: &tauri::AppHandle) -> std::result::Result<PathBuf, String> {
    Ok(staged_dir(app)?.join("staged-update.json"))
}

/// Reads the staged-update marker, cleaning up silently when the marker or the
/// installer file is missing/corrupt (a staged download is best-effort).
fn read_staged_marker(app: &tauri::AppHandle) -> std::result::Result<Option<StagedMarker>, String> {
    let path = staged_marker_path(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => {
            let _ = std::fs::remove_file(&path);
            return Ok(None);
        }
    };
    let marker: StagedMarker = match serde_json::from_str(&raw) {
        Ok(marker) => marker,
        Err(_) => {
            let _ = std::fs::remove_file(&path);
            return Ok(None);
        }
    };
    if !std::path::Path::new(&marker.file).exists() {
        let _ = std::fs::remove_file(&path);
        return Ok(None);
    }
    Ok(Some(marker))
}

fn write_staged_marker(
    app: &tauri::AppHandle,
    marker: &StagedMarker,
) -> std::result::Result<(), String> {
    let path = staged_marker_path(app)?;
    let raw = serde_json::to_string(marker)
        .map_err(|error| format!("Failed to serialize staged marker: {error}"))?;
    std::fs::write(&path, raw).map_err(|error| format!("Failed to write staged marker: {error}"))
}

fn cleanup_staged(app: &tauri::AppHandle) {
    if let Ok(Some(marker)) = read_staged_marker(app) {
        let _ = std::fs::remove_file(marker.file);
    }
    if let Ok(path) = staged_marker_path(app) {
        let _ = std::fs::remove_file(path);
    }
}

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub current_version: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub current_version: String,
    pub staged_version: Option<String>,
    pub staged_notes: Option<String>,
    pub staged_pub_date: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

// ── Commands ────────────────────────────────────────────────────────────────

/// Checks for an update. Unlike a plain "no update" result, a failure to reach
/// the update server is surfaced via the `error` field so the UI can distinguish
/// "up to date" from "check failed".
#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current_version = app.package_info().version.to_string();

    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(error) => {
            log::debug!("updater unavailable: {error}");
            return Ok(UpdateInfo {
                available: false,
                version: None,
                notes: None,
                pub_date: None,
                current_version,
                error: Some(format!("Update service unavailable: {error}")),
            });
        }
    };

    match updater.check().await {
        Ok(Some(update)) => Ok(UpdateInfo {
            available: true,
            version: Some(update.version.clone()),
            notes: update.body.clone(),
            pub_date: update.date.map(|d| d.to_string()),
            current_version,
            error: None,
        }),
        Ok(None) => Ok(UpdateInfo {
            available: false,
            version: None,
            notes: None,
            pub_date: None,
            current_version,
            error: None,
        }),
        Err(error) => {
            log::debug!("update check failed: {error}");
            Ok(UpdateInfo {
                available: false,
                version: None,
                notes: None,
                pub_date: None,
                current_version,
                error: Some(format!("Could not reach the update server: {error}")),
            })
        }
    }
}

/// Reports the installed version plus any staged download that survived an app
/// restart. Never hits the network.
#[tauri::command]
pub fn get_update_status(app: tauri::AppHandle) -> Result<UpdateStatus, String> {
    let current_version = app.package_info().version.to_string();

    let Some(marker) = read_staged_marker(&app)? else {
        return Ok(UpdateStatus {
            current_version,
            staged_version: None,
            staged_notes: None,
            staged_pub_date: None,
        });
    };

    if marker.version == current_version {
        // The update was already applied; clear the stale marker.
        cleanup_staged(&app);
        return Ok(UpdateStatus {
            current_version,
            staged_version: None,
            staged_notes: None,
            staged_pub_date: None,
        });
    }

    Ok(UpdateStatus {
        current_version,
        staged_version: Some(marker.version),
        staged_notes: marker.notes,
        staged_pub_date: marker.pub_date,
    })
}

/// Downloads the pending update, emitting `update://progress` events and
/// persisting the verified installer so a staged install survives restarts.
#[tauri::command]
pub async fn download_update(app: tauri::AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|error| error.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|error| format!("Update check failed: {error}"))?
        .ok_or_else(|| "No update available".to_string())?;

    let progress_app = app.clone();
    let download = update.clone();
    let task = tokio::spawn(async move {
        let mut downloaded: u64 = 0;
        let bytes = download
            .download(
                move |chunk_len, total| {
                    downloaded += chunk_len as u64;
                    let _ = progress_app
                        .emit("update://progress", UpdateProgress { downloaded, total });
                },
                || {},
            )
            .await
            .map_err(|error| format!("Download failed: {error}"))?;
        Ok::<Vec<u8>, String>(bytes)
    });

    *app.state::<UpdateState>().download.lock() = Some(task.abort_handle());

    let bytes = match task.await {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(error)) => return Err(error),
        Err(_) => return Err("Download cancelled".to_string()),
    };

    let version = update.version.clone();
    let file_path = staged_dir(&app)?.join(format!("update-{version}.exe"));
    std::fs::write(&file_path, &bytes)
        .map_err(|error| format!("Failed to save update file: {error}"))?;

    let marker = StagedMarker {
        version: version.clone(),
        notes: update.body.clone(),
        pub_date: update.date.map(|d| d.to_string()),
        file: file_path.to_string_lossy().to_string(),
    };
    write_staged_marker(&app, &marker)?;

    *app.state::<UpdateState>().update.lock() = Some(update);
    Ok(())
}

/// Aborts an in-flight `download_update`. The download command then resolves
/// with a "Download cancelled" error, which the frontend maps back to the
/// available state.
#[tauri::command]
pub fn cancel_update_download(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(abort) = app.state::<UpdateState>().download.lock().take() {
        abort.abort();
    }
    Ok(())
}

/// Installs the staged update. On Windows the installer is launched and the app
/// process exits; the NSIS installer relaunches the app after installing.
#[tauri::command]
pub async fn install_staged_update(app: tauri::AppHandle) -> Result<(), String> {
    static INSTALLING: AtomicBool = AtomicBool::new(false);
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err("An update install is already in progress".to_string());
    }

    let result = install_staged_inner(&app).await;
    if result.is_err() {
        INSTALLING.store(false, Ordering::SeqCst);
    }
    result
}

async fn install_staged_inner(app: &tauri::AppHandle) -> Result<(), String> {
    let marker = read_staged_marker(app)?.ok_or_else(|| "No staged update found".to_string())?;
    let current_version = app.package_info().version.to_string();
    if marker.version == current_version {
        cleanup_staged(app);
        return Err("Already running the staged version".to_string());
    }

    let update = {
        // Take the session handle out of state so the mutex guard drops before
        // any await below (keeps the command future Send).
        let in_session = {
            let state = app.state::<UpdateState>();
            let mut guard = state.update.lock();
            guard.take()
        };
        match in_session {
            Some(update) => update,
            None => {
                // Fresh launch after a restart: rebuild the handle by re-checking
                // (requires network). The verified bytes are already on disk.
                let updater = app.updater().map_err(|error| error.to_string())?;
                let update = updater
                    .check()
                    .await
                    .map_err(|error| format!("Update check failed (internet required): {error}"))?
                    .ok_or_else(|| "Update no longer available".to_string())?;
                if update.version != marker.version {
                    return Err(format!(
                        "A different update (v{}) is now available; download it again",
                        update.version
                    ));
                }
                update
            }
        }
    };

    let bytes = std::fs::read(&marker.file)
        .map_err(|error| format!("Failed to read staged update: {error}"))?;
    update
        .install(&bytes)
        .map_err(|error| format!("Install failed: {error}"))?;
    Ok(())
}

/// Opens a URL in the system browser (used for release notes links).
#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|error| format!("Failed to open link: {error}"))
}
