use super::*;
use std::fs;
use std::io::Read;

/// Save a custom branding logo to the app data directory.
/// Returns the stored file path relative to the app data dir.
/// Save a custom branding logo from raw bytes.
#[tauri::command]
pub fn save_branding_logo(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    file_data: Vec<u8>,
    filename: String,
) -> std::result::Result<String, String> {
    save_branding_logo_inner(&app, pool.inner().clone(), file_data, filename)
}

/// Get the current branding logo path from settings.
#[tauri::command]
pub fn get_branding_logo_path(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<Option<String>, String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    let settings = repo.get().map_err(|e| e.to_string())?;
    Ok(settings.branding_logo_path)
}

/// Get the path to the bundled default logo (school seal).
#[tauri::command]
pub fn get_default_logo_path() -> std::result::Result<String, String> {
    // The default logo is bundled as a static asset in the frontend.
    // This returns a marker so the frontend knows to use the built-in import.
    Ok("__default__".to_string())
}

/// Open a file picker dialog and save the selected image as branding logo.
/// Returns the stored file path on success.
#[tauri::command]
pub fn pick_branding_logo(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Images", &["png", "jpg", "jpeg", "svg"])
        .pick_file(move |result| {
            let _ = tx.send(result);
        });

    let file_path = rx
        .recv()
        .map_err(|e| format!("Failed to receive file path: {e}"))?
        .ok_or_else(|| "User cancelled file picker".to_string())?;

    let path = match file_path {
        tauri_plugin_dialog::FilePath::Path(p) => p,
        tauri_plugin_dialog::FilePath::Url(url) => {
            return Err(format!("URL file paths not supported: {url}"));
        }
    };

    let mut file = fs::File::open(&path).map_err(|e| format!("Failed to open file: {e}"))?;
    let mut file_data = Vec::new();
    file.read_to_end(&mut file_data)
        .map_err(|e| format!("Failed to read file: {e}"))?;

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("logo.png")
        .to_string();

    save_branding_logo_inner(&app, pool.inner().clone(), file_data, filename)
}

/// Save branding logo from raw file data (used by pick_branding_logo and direct upload).
fn save_branding_logo_inner(
    app: &tauri::AppHandle,
    pool: Pool<SqliteConnectionManager>,
    file_data: Vec<u8>,
    filename: String,
) -> std::result::Result<String, String> {
    let branding_dir = app_data_dir(app)?.join("assets").join("branding");
    fs::create_dir_all(&branding_dir)
        .map_err(|e| format!("Failed to create branding directory: {e}"))?;

    let ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");

    let valid_exts = ["png", "jpg", "jpeg", "svg"];
    if !valid_exts.contains(&ext.to_lowercase().as_str()) {
        return Err(format!(
            "Unsupported file type: .{ext}. Please use PNG, JPG, or SVG."
        ));
    }

    if file_data.len() < 8 {
        return Err("File too small to be a valid image".to_string());
    }

    let uuid = uuid::Uuid::new_v4();
    let dest = branding_dir.join(format!("user-{uuid}.{ext}"));

    fs::write(&dest, &file_data)
        .map_err(|e| format!("Failed to write logo file: {e}"))?;

    let settings_repo = SettingsRepository::new(pool);
    let mut settings = settings_repo.get().map_err(|e| e.to_string())?;
    settings.branding_logo_path = Some(dest.to_string_lossy().to_string());
    settings_repo.update(settings).map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().to_string())
}

/// Delete a previously uploaded branding logo file.
#[tauri::command]
pub fn delete_branding_logo(path: String) -> std::result::Result<(), String> {
    let p = std::path::Path::new(&path);
    if p.exists() {
        fs::remove_file(p).map_err(|e| format!("Failed to delete logo: {e}"))?;
    }
    Ok(())
}

/// Reset branding to defaults: clears logo_path and sets title to "EES AMS".
#[tauri::command]
pub fn reset_branding(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<Settings, String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    let mut settings = repo.get().map_err(|e| e.to_string())?;
    settings.branding_logo_path = None;
    settings.branding_title = "EES AMS".to_string();
    repo.update(settings).map_err(|e| e.to_string())
}
