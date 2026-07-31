use super::*;

// ── Updater Commands ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub current_version: String,
}

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
        }),
        Ok(None) => Ok(UpdateInfo {
            available: false,
            version: None,
            notes: None,
            pub_date: None,
            current_version,
        }),
        Err(error) => {
            // No published release, unreachable endpoint, or network failure:
            // report "no update available" instead of surfacing an error.
            log::debug!("update check failed: {error}");
            Ok(UpdateInfo {
                available: false,
                version: None,
                notes: None,
                pub_date: None,
                current_version,
            })
        }
    }
}

#[tauri::command]
pub async fn download_and_install(app: tauri::AppHandle) -> Result<String, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update available".to_string())?;

    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .map_err(|e| e.to_string())?;

    Ok("Update installed. The app will restart shortly.".to_string())
}
