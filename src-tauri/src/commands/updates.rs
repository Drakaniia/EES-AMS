use super::*;
use crate::infrastructure::database::DbPool;

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
    /// §9.4: set when the attendance record count fell between the previous
    /// version and this one, naming the pre-install backup to restore from.
    pub attendance_warning: Option<String>,
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
    // The warning outlives the staged marker: it is about what the *installed*
    // version did to the attendance records, not about a pending download.
    let attendance_warning = app
        .path()
        .app_data_dir()
        .ok()
        .as_deref()
        .and_then(read_attendance_warning);

    let Some(marker) = read_staged_marker(&app)? else {
        return Ok(UpdateStatus {
            current_version,
            staged_version: None,
            staged_notes: None,
            staged_pub_date: None,
            attendance_warning,
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
            attendance_warning,
        });
    }

    Ok(UpdateStatus {
        current_version,
        staged_version: Some(marker.version),
        staged_notes: marker.notes,
        staged_pub_date: marker.pub_date,
        attendance_warning,
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

    // Safeguard: snapshot the database *and* the SF2 workbooks before the
    // installer runs. The update only replaces app binaries, but a fresh backup
    // gives a rollback point if anything goes wrong; refuse to install when the
    // snapshot fails.
    //
    // `create_manual_backup` writes a full backup folder, so the workbooks are
    // inside it: the X marks are now covered by the same policy as the
    // database, and the same `?` fails the install when either cannot be
    // captured.
    let pool = app.state::<DbPool>().inner();
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;
    let pre_install = backup_service::create_backup_at(
        pool,
        &app_dir,
        crate::backup::models::BackupKind::PreInstall,
        chrono::Local::now(),
    )
    .map_err(|error| format!("Pre-install backup failed: {error}"))?;
    log::info!(
        "created pre-install backup at {} ({} workbook(s))",
        pre_install.path,
        pre_install.workbook_count
    );

    // §9.4 items 2 and 3: compare this version's attendance counts against the
    // baseline recorded when the previous version first ran, then re-baseline
    // to this version so the next update compares against the right thing.
    check_attendance_fingerprint(pool, &app_dir, &pre_install.path)?;

    let bytes = std::fs::read(&marker.file)
        .map_err(|error| format!("Failed to read staged update: {error}"))?;
    update
        .install(&bytes)
        .map_err(|error| format!("Install failed: {error}"))?;
    Ok(())
}

/// Compare the live attendance counts against the recorded baseline and, if
/// absences went backwards, record a loud warning for the Updates panel.
///
/// The warning is recorded rather than returned: the update itself must not be
/// blocked by it, because the user is the one who asked to update and may be
/// mid-way through a legitimate re-entry. It is surfaced on the Updates panel
/// until the next install, pointing at the pre-install backup that holds the
/// records.
fn check_attendance_fingerprint(
    pool: &DbPool,
    app_dir: &std::path::Path,
    pre_install_backup_path: &str,
) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|error| format!("Failed to open the database for the update check: {error}"))?;

    let current = match backup_service::read_db_fingerprint(&conn) {
        Ok(fingerprint) => fingerprint,
        Err(error) => {
            log::warn!("could not read the attendance fingerprint: {error}");
            return Ok(());
        }
    };

    let previous = match backup_service::load_db_fingerprint(app_dir) {
        Ok(previous) => previous,
        Err(error) => {
            log::warn!("could not read the stored attendance fingerprint: {error}");
            return Ok(());
        }
    };

    let notice = previous
        .as_ref()
        .and_then(|previous| {
            crate::backup::fingerprint::DbFingerprint::decrease_notice(previous, &current)
        })
        .map(|notice| format!("{notice} The pre-install backup is at {pre_install_backup_path}."));

    if let Some(ref notice) = notice {
        log::error!("{notice}");
    }

    write_attendance_warning(app_dir, notice.as_deref());

    // Re-baseline even when the comparison failed, so a transient read error
    // does not make every future update report the same drop.
    if let Err(error) = backup_service::record_db_fingerprint(app_dir, &current) {
        log::warn!("could not re-baseline the attendance fingerprint: {error}");
    }

    Ok(())
}

fn attendance_warning_path(app_dir: &std::path::Path) -> PathBuf {
    app_dir.join("attendance-warning.json")
}

fn write_attendance_warning(app_dir: &std::path::Path, notice: Option<&str>) {
    let path = attendance_warning_path(app_dir);
    let result = match notice {
        Some(notice) => std::fs::write(&path, serde_json::json!({ "message": notice }).to_string()),
        None => match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        },
    };
    if let Err(error) = result {
        log::warn!("could not update the attendance warning file: {error}");
    }
}

fn read_attendance_warning(app_dir: &std::path::Path) -> Option<String> {
    let raw = std::fs::read_to_string(attendance_warning_path(app_dir)).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    value
        .get("message")
        .and_then(|message| message.as_str())
        .map(str::to_string)
}

/// Opens a URL in the system browser (used for release notes links).
#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|error| format!("Failed to open link: {error}"))
}
