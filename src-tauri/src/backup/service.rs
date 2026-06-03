use super::models::{BackupKind, BackupPreview, BackupStatus, BackupSummary, RestoreResult};
use crate::infrastructure::database::{migrate_db, DbPool, CURRENT_SCHEMA_VERSION};
use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use rand::{distributions::Alphanumeric, Rng};
use rusqlite::{Connection, DatabaseName, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use url::Url;

const BACKUP_DIR_NAME: &str = "backups";
const STATE_FILE_NAME: &str = "backup-state.json";
const RETENTION_LIMIT: usize = 30;
const BACKUP_PREFIX: &str = "attendance-";
const SYNC_BACKUP_DIR_NAME: &str = "EES-AMS Backups";
const GOOGLE_DRIVE_FOLDER_NAME: &str = "EES-AMS Backups";
const GOOGLE_DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive";
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const GOOGLE_DRIVE_UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";
const KEYRING_SERVICE: &str = "ees-ams";
const KEYRING_REFRESH_TOKEN_USER: &str = "google-drive-refresh-token";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupState {
    sync_folder_path: Option<String>,
    last_backup_at: Option<i64>,
    last_backup_path: Option<String>,
    last_error: Option<String>,
    last_sync_error: Option<String>,
    google_drive: Option<GoogleDriveState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleDriveState {
    folder_id: String,
    folder_name: String,
    connected_at: i64,
    last_backup_at: Option<i64>,
    last_file_id: Option<String>,
    last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DriveFileResponse {
    id: String,
    name: Option<String>,
}

pub fn spawn_backup_scheduler(pool: DbPool, app_dir: PathBuf) {
    thread::spawn(move || {
        if let Err(error) = ensure_daily_backup(&pool, &app_dir) {
            record_backup_error(&app_dir, error);
        }

        loop {
            thread::sleep(Duration::from_secs(60 * 60));
            if let Err(error) = ensure_daily_backup(&pool, &app_dir) {
                record_backup_error(&app_dir, error);
            }
        }
    });
}

pub fn get_status(app_dir: &Path) -> Result<BackupStatus> {
    let backups = list_backups(app_dir)?;
    let backup_dir = backup_dir(app_dir);
    let state = load_state(app_dir).unwrap_or_else(|error| BackupState {
        last_error: Some(format!("Failed to read backup settings: {error}")),
        ..BackupState::default()
    });
    let latest = backups.first();
    let google_drive = state.google_drive.clone();

    Ok(BackupStatus {
        local_backup_dir: backup_dir.to_string_lossy().to_string(),
        backup_count: backups.len(),
        retention_limit: RETENTION_LIMIT,
        last_backup_at: state
            .last_backup_at
            .or_else(|| latest.map(|backup| backup.created_at)),
        last_backup_path: state
            .last_backup_path
            .or_else(|| latest.map(|backup| backup.path.clone())),
        sync_folder_path: state.sync_folder_path,
        last_error: state.last_error,
        last_sync_error: state.last_sync_error,
        google_drive_configured: google_client_id().is_ok(),
        google_drive_connected: google_drive.is_some(),
        google_drive_folder_id: google_drive.as_ref().map(|drive| drive.folder_id.clone()),
        google_drive_folder_name: google_drive.as_ref().map(|drive| drive.folder_name.clone()),
        last_google_drive_backup_at: google_drive.as_ref().and_then(|drive| drive.last_backup_at),
        last_google_drive_file_id: google_drive
            .as_ref()
            .and_then(|drive| drive.last_file_id.clone()),
        last_google_drive_error: google_drive
            .as_ref()
            .and_then(|drive| drive.last_error.clone()),
    })
}

pub fn list_backups(app_dir: &Path) -> Result<Vec<BackupSummary>> {
    let backup_dir = backup_dir(app_dir);
    fs::create_dir_all(&backup_dir)
        .with_context(|| format!("failed to create backup directory {}", backup_dir.display()))?;

    let mut backups = Vec::new();
    for entry in fs::read_dir(&backup_dir)
        .with_context(|| format!("failed to read backup directory {}", backup_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("db") {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !is_app_backup_file(file_name) {
            continue;
        }

        backups.push(summary_from_path(&path)?);
    }

    backups.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.file_name.cmp(&left.file_name))
    });

    Ok(backups)
}

pub fn create_manual_backup(pool: &DbPool, app_dir: &Path) -> Result<BackupStatus> {
    create_backup_at(pool, app_dir, BackupKind::Manual, Local::now())?;
    get_status(app_dir)
}

pub fn backup_database_to_path(pool: &DbPool, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("backup destination has no parent directory"))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;

    let temp_path = destination.with_extension("db.tmp");
    if temp_path.exists() {
        fs::remove_file(&temp_path).with_context(|| {
            format!("failed to remove stale temp backup {}", temp_path.display())
        })?;
    }

    let source = pool.get().context("failed to get database connection")?;
    source
        .backup(
            DatabaseName::Main,
            &temp_path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .with_context(|| format!("failed to export database {}", temp_path.display()))?;
    preview_backup(&temp_path).context("exported database failed validation")?;

    if destination.exists() {
        fs::remove_file(destination)
            .with_context(|| format!("failed to replace {}", destination.display()))?;
    }
    fs::rename(&temp_path, destination).with_context(|| {
        format!(
            "failed to finalize database export {} -> {}",
            temp_path.display(),
            destination.display()
        )
    })?;

    Ok(())
}

pub fn ensure_daily_backup(pool: &DbPool, app_dir: &Path) -> Result<Option<BackupSummary>> {
    ensure_daily_backup_at(pool, app_dir, Local::now())
}

pub fn ensure_daily_backup_at(
    pool: &DbPool,
    app_dir: &Path,
    now: DateTime<Local>,
) -> Result<Option<BackupSummary>> {
    let today = now.date_naive();
    let has_backup_today = list_backups(app_dir)?.iter().any(|backup| {
        DateTime::from_timestamp(backup.created_at, 0)
            .map(|timestamp| timestamp.with_timezone(&Local).date_naive() == today)
            .unwrap_or(false)
    });

    if has_backup_today {
        return Ok(None);
    }

    create_backup_at(pool, app_dir, BackupKind::Auto, now).map(Some)
}

pub fn set_sync_folder(app_dir: &Path, folder_path: Option<PathBuf>) -> Result<BackupStatus> {
    let mut state = load_state(app_dir).unwrap_or_default();
    state.sync_folder_path = folder_path
        .map(|path| prepare_sync_folder(&path))
        .transpose()?
        .map(|path| path.to_string_lossy().to_string());
    state.last_sync_error = None;
    save_state(app_dir, &state)?;
    get_status(app_dir)
}

pub fn connect_google_drive(app_dir: &Path) -> Result<BackupStatus> {
    let client_id = google_client_id()?;
    let oauth = authorize_google_drive(&client_id)?;
    if !oauth
        .token_type
        .as_deref()
        .unwrap_or("Bearer")
        .eq_ignore_ascii_case("Bearer")
    {
        bail!("Google returned an unsupported OAuth token type");
    }
    log::debug!(
        "received Google Drive access token that expires in {:?} seconds",
        oauth.expires_in
    );

    if let Some(refresh_token) = oauth.refresh_token.as_deref() {
        save_refresh_token(refresh_token)?;
    } else if load_refresh_token().is_err() {
        bail!("Google did not return a refresh token. Try disconnecting and connecting again.");
    }

    let folder = create_google_drive_folder(&oauth.access_token)?;
    let mut state = load_state(app_dir).unwrap_or_default();
    state.google_drive = Some(GoogleDriveState {
        folder_id: folder.id,
        folder_name: folder
            .name
            .unwrap_or_else(|| GOOGLE_DRIVE_FOLDER_NAME.to_string()),
        connected_at: Utc::now().timestamp(),
        last_backup_at: None,
        last_file_id: None,
        last_error: None,
    });
    save_state(app_dir, &state)?;

    get_status(app_dir)
}

pub fn disconnect_google_drive(app_dir: &Path) -> Result<BackupStatus> {
    let mut state = load_state(app_dir).unwrap_or_default();
    state.google_drive = None;
    let _ = delete_refresh_token();
    save_state(app_dir, &state)?;
    get_status(app_dir)
}

pub fn upload_latest_backup_to_google_drive(app_dir: &Path) -> Result<BackupStatus> {
    let latest = list_backups(app_dir)?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No local backup is available to upload"))?;
    let mut state = load_state(app_dir).unwrap_or_default();
    upload_backup_to_google_drive(&mut state, Path::new(&latest.path))?;
    save_state(app_dir, &state)?;
    get_status(app_dir)
}

pub fn preview_backup(source_path: &Path) -> Result<BackupPreview> {
    if !source_path.exists() {
        bail!("Backup file does not exist: {}", source_path.display());
    }

    let metadata = fs::metadata(source_path)
        .with_context(|| format!("failed to inspect backup {}", source_path.display()))?;
    let modified_at = metadata_timestamp(&metadata)?;
    let file_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("backup.db")
        .to_string();

    let conn = Connection::open_with_flags(source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open backup {}", source_path.display()))?;
    run_integrity_check(&conn)?;

    let schema_version = read_schema_version(&conn)?;
    if schema_version > CURRENT_SCHEMA_VERSION {
        bail!(
            "Backup schema version {schema_version} is newer than this app supports ({CURRENT_SCHEMA_VERSION})"
        );
    }

    require_core_tables(&conn)?;

    let mut warnings = Vec::new();
    if schema_version < CURRENT_SCHEMA_VERSION {
        warnings.push(format!(
            "Backup will be migrated from schema version {schema_version} to {CURRENT_SCHEMA_VERSION} during restore."
        ));
    }

    Ok(BackupPreview {
        source_path: source_path.to_string_lossy().to_string(),
        file_name,
        modified_at,
        size_bytes: metadata.len(),
        schema_version,
        student_count: count_table_rows(&conn, "students")?,
        class_count: count_table_rows(&conn, "classes")?,
        event_count: count_table_rows(&conn, "events")?,
        settings_count: count_table_rows(&conn, "settings")?,
        sf2_template_count: count_table_rows(&conn, "sf2_templates")?,
        warnings,
    })
}

pub fn restore_backup(pool: &DbPool, app_dir: &Path, source_path: &Path) -> Result<RestoreResult> {
    let preview = preview_backup(source_path)?;
    let pre_restore_backup = create_backup_at(pool, app_dir, BackupKind::PreRestore, Local::now())
        .context("failed to create pre-restore safety backup")?;

    let mut pooled = pool.get().context("failed to get database connection")?;
    let conn: &mut Connection = &mut pooled;
    conn.restore(
        DatabaseName::Main,
        source_path,
        None::<fn(rusqlite::backup::Progress)>,
    )
    .with_context(|| format!("failed to restore backup {}", source_path.display()))?;
    migrate_db(conn).context("failed to migrate restored database")?;
    run_integrity_check(conn).context("restored database failed integrity check")?;

    Ok(RestoreResult {
        restored_path: source_path.to_string_lossy().to_string(),
        pre_restore_backup_path: pre_restore_backup.path,
        restored_at: Utc::now().timestamp(),
        schema_version: read_schema_version(conn)?,
        migrated: preview.schema_version < CURRENT_SCHEMA_VERSION,
        warnings: preview.warnings,
    })
}

pub fn enforce_retention(app_dir: &Path) -> Result<()> {
    let backups = list_backups(app_dir)?;
    for backup in backups.into_iter().skip(RETENTION_LIMIT) {
        fs::remove_file(&backup.path)
            .with_context(|| format!("failed to remove old backup {}", backup.path))?;
    }
    Ok(())
}

fn create_backup_at(
    pool: &DbPool,
    app_dir: &Path,
    kind: BackupKind,
    now: DateTime<Local>,
) -> Result<BackupSummary> {
    let backup_dir = backup_dir(app_dir);
    fs::create_dir_all(&backup_dir)
        .with_context(|| format!("failed to create backup directory {}", backup_dir.display()))?;

    let final_path = unique_backup_path(&backup_dir, kind, now);
    let temp_path = final_path.with_file_name(format!(
        "{}.tmp",
        final_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("attendance-backup.db")
    ));
    if temp_path.exists() {
        fs::remove_file(&temp_path).with_context(|| {
            format!("failed to remove stale temp backup {}", temp_path.display())
        })?;
    }

    let source = pool.get().context("failed to get database connection")?;
    source
        .backup(
            DatabaseName::Main,
            &temp_path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .with_context(|| format!("failed to create backup {}", temp_path.display()))?;

    preview_backup(&temp_path).context("created backup failed validation")?;
    fs::rename(&temp_path, &final_path).with_context(|| {
        format!(
            "failed to finalize backup {} -> {}",
            temp_path.display(),
            final_path.display()
        )
    })?;

    enforce_retention(app_dir)?;

    let summary = summary_from_path(&final_path)?;
    let mut state = load_state(app_dir).unwrap_or_default();
    state.last_backup_at = Some(summary.created_at);
    state.last_backup_path = Some(summary.path.clone());
    state.last_error = None;
    state.last_sync_error = copy_to_sync_folder(&state, &final_path).err().map(|error| {
        log::warn!("backup sync failed: {error}");
        error.to_string()
    });
    if let Err(error) = upload_backup_to_google_drive(&mut state, &final_path) {
        if let Some(google_drive) = state.google_drive.as_mut() {
            google_drive.last_error = Some(error.to_string());
        }
        log::warn!("Google Drive backup upload failed: {error}");
    }
    save_state(app_dir, &state)?;

    Ok(summary)
}

fn copy_to_sync_folder(state: &BackupState, source_path: &Path) -> Result<()> {
    let Some(sync_folder_path) = &state.sync_folder_path else {
        return Ok(());
    };

    let sync_folder = PathBuf::from(sync_folder_path);
    if !sync_folder.is_dir() {
        bail!("sync folder is unavailable: {}", sync_folder.display());
    }

    let file_name = source_path
        .file_name()
        .ok_or_else(|| anyhow!("backup file name is missing"))?;
    let destination = sync_folder.join(file_name);
    fs::copy(source_path, &destination).with_context(|| {
        format!(
            "failed to copy backup to sync folder {}",
            destination.display()
        )
    })?;

    Ok(())
}

fn prepare_sync_folder(selected_folder: &Path) -> Result<PathBuf> {
    let sync_folder = if selected_folder
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(SYNC_BACKUP_DIR_NAME))
    {
        selected_folder.to_path_buf()
    } else {
        selected_folder.join(SYNC_BACKUP_DIR_NAME)
    };

    fs::create_dir_all(&sync_folder)
        .with_context(|| format!("failed to create sync folder {}", sync_folder.display()))?;

    Ok(sync_folder)
}

fn google_client_id() -> Result<String> {
    option_env!("EES_AMS_GOOGLE_CLIENT_ID")
        .map(str::to_string)
        .or_else(|| std::env::var("EES_AMS_GOOGLE_CLIENT_ID").ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow!(
                "Google Drive is not configured. Set EES_AMS_GOOGLE_CLIENT_ID before building the app."
            )
        })
}

fn authorize_google_drive(client_id: &str) -> Result<OAuthTokenResponse> {
    let listener = TcpListener::bind("127.0.0.1:0").context("failed to start OAuth callback")?;
    listener
        .set_nonblocking(true)
        .context("failed to configure OAuth callback")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/oauth2/callback");
    let state = random_oauth_value(32);
    let code_verifier = random_oauth_value(64);
    let code_challenge = pkce_challenge(&code_verifier);

    let auth_url = Url::parse_with_params(
        GOOGLE_AUTH_URL,
        &[
            ("client_id", client_id),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", GOOGLE_DRIVE_SCOPE),
            ("access_type", "offline"),
            ("prompt", "consent"),
            ("state", state.as_str()),
            ("code_challenge", code_challenge.as_str()),
            ("code_challenge_method", "S256"),
        ],
    )?;

    open::that(auth_url.as_str()).context("failed to open Google sign-in in browser")?;
    let code = wait_for_oauth_code(listener, &state)?;

    let client = reqwest::blocking::Client::new();
    client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("code", code.as_str()),
            ("code_verifier", code_verifier.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .context("failed to exchange Google authorization code")?
        .error_for_status()
        .context("Google rejected the authorization code")?
        .json::<OAuthTokenResponse>()
        .context("failed to read Google OAuth response")
}

fn wait_for_oauth_code(listener: TcpListener, expected_state: &str) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(180);

    loop {
        if Instant::now() > deadline {
            bail!("Timed out waiting for Google sign-in");
        }

        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buffer = [0_u8; 8192];
                let read = stream
                    .read(&mut buffer)
                    .context("failed to read OAuth callback")?;
                let request = String::from_utf8_lossy(&buffer[..read]);
                let result = parse_oauth_callback(&request, expected_state);
                let html = if result.is_ok() {
                    "<h1>EES-AMS Google Drive connected</h1><p>You can close this tab.</p>"
                } else {
                    "<h1>EES-AMS Google Drive failed</h1><p>Return to the app and try again.</p>"
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    html.len(),
                    html
                );
                let _ = stream.write_all(response.as_bytes());
                return result;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(error).context("failed to receive OAuth callback"),
        }
    }
}

fn parse_oauth_callback(request: &str, expected_state: &str) -> Result<String> {
    let first_line = request
        .lines()
        .next()
        .ok_or_else(|| anyhow!("OAuth callback was empty"))?;
    let path = first_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow!("OAuth callback path was missing"))?;
    let callback_url = Url::parse(&format!("http://127.0.0.1{path}"))?;
    let mut code = None;
    let mut state = None;
    let mut oauth_error = None;

    for (key, value) in callback_url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            "error" => oauth_error = Some(value.to_string()),
            _ => {}
        }
    }

    if let Some(error) = oauth_error {
        bail!("Google sign-in failed: {error}");
    }
    if state.as_deref() != Some(expected_state) {
        bail!("Google sign-in state did not match");
    }

    code.ok_or_else(|| anyhow!("Google sign-in did not return an authorization code"))
}

fn random_oauth_value(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn pkce_challenge(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn create_google_drive_folder(access_token: &str) -> Result<DriveFileResponse> {
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "name": GOOGLE_DRIVE_FOLDER_NAME,
        "mimeType": "application/vnd.google-apps.folder"
    });

    client
        .post(format!("{GOOGLE_DRIVE_FILES_URL}?fields=id,name"))
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .context("failed to create Google Drive backup folder")?
        .error_for_status()
        .context("Google Drive rejected backup folder creation")?
        .json::<DriveFileResponse>()
        .context("failed to read Google Drive folder response")
}

fn refresh_google_access_token() -> Result<String> {
    let client_id = google_client_id()?;
    let refresh_token = load_refresh_token()?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("client_id", client_id.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .context("failed to refresh Google Drive access token")?
        .error_for_status()
        .context("Google rejected the saved Drive connection")?
        .json::<OAuthTokenResponse>()
        .context("failed to read Google token refresh response")?;

    Ok(response.access_token)
}

fn upload_backup_to_google_drive(state: &mut BackupState, source_path: &Path) -> Result<()> {
    let Some(google_drive) = state.google_drive.as_mut() else {
        return Ok(());
    };

    let access_token = refresh_google_access_token()?;
    let file_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("backup file name is invalid"))?;
    let metadata = serde_json::json!({
        "name": file_name,
        "parents": [google_drive.folder_id]
    });
    let bytes = fs::read(source_path)
        .with_context(|| format!("failed to read backup {}", source_path.display()))?;
    let metadata_part = reqwest::blocking::multipart::Part::text(metadata.to_string())
        .mime_str("application/json")
        .context("failed to build Google Drive metadata upload")?;
    let file_part = reqwest::blocking::multipart::Part::bytes(bytes)
        .file_name(file_name.to_string())
        .mime_str("application/x-sqlite3")
        .context("failed to build Google Drive backup upload")?;
    let form = reqwest::blocking::multipart::Form::new()
        .part("metadata", metadata_part)
        .part("file", file_part);

    let response = reqwest::blocking::Client::new()
        .post(format!(
            "{GOOGLE_DRIVE_UPLOAD_URL}?uploadType=multipart&fields=id,name"
        ))
        .bearer_auth(access_token)
        .multipart(form)
        .send()
        .context("failed to upload backup to Google Drive")?
        .error_for_status()
        .context("Google Drive rejected backup upload")?
        .json::<DriveFileResponse>()
        .context("failed to read Google Drive upload response")?;

    google_drive.last_backup_at = Some(Utc::now().timestamp());
    google_drive.last_file_id = Some(response.id);
    google_drive.last_error = None;

    Ok(())
}

fn save_refresh_token(refresh_token: &str) -> Result<()> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_REFRESH_TOKEN_USER)?
        .set_password(refresh_token)
        .context("failed to store Google Drive refresh token")
}

fn load_refresh_token() -> Result<String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_REFRESH_TOKEN_USER)?
        .get_password()
        .context("failed to read Google Drive refresh token")
}

fn delete_refresh_token() -> Result<()> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_REFRESH_TOKEN_USER)?
        .delete_credential()
        .context("failed to delete Google Drive refresh token")
}

fn record_backup_error(app_dir: &Path, error: anyhow::Error) {
    let mut state = load_state(app_dir).unwrap_or_default();
    state.last_error = Some(error.to_string());
    if let Err(write_error) = save_state(app_dir, &state) {
        log::warn!("failed to record backup error: {write_error}");
    }
    log::warn!("automatic backup failed: {error}");
}

fn backup_dir(app_dir: &Path) -> PathBuf {
    app_dir.join(BACKUP_DIR_NAME)
}

fn state_path(app_dir: &Path) -> PathBuf {
    app_dir.join(STATE_FILE_NAME)
}

fn load_state(app_dir: &Path) -> Result<BackupState> {
    let path = state_path(app_dir);
    if !path.exists() {
        return Ok(BackupState::default());
    }

    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

fn save_state(app_dir: &Path, state: &BackupState) -> Result<()> {
    fs::create_dir_all(app_dir)
        .with_context(|| format!("failed to create app data directory {}", app_dir.display()))?;
    let path = state_path(app_dir);
    let temp_path = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(state)?;
    fs::write(&temp_path, content)
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("failed to replace {}", path.display()))?;
    }
    fs::rename(&temp_path, &path)
        .with_context(|| format!("failed to finalize {}", path.display()))?;
    Ok(())
}

fn summary_from_path(path: &Path) -> Result<BackupSummary> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to inspect {}", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("backup file name is invalid"))?
        .to_string();

    Ok(BackupSummary {
        path: path.to_string_lossy().to_string(),
        file_name: file_name.clone(),
        created_at: backup_timestamp_from_file_name(&file_name)
            .unwrap_or(metadata_timestamp(&metadata)?),
        size_bytes: metadata.len(),
        kind: backup_kind_from_file_name(&file_name),
    })
}

fn unique_backup_path(backup_dir: &Path, kind: BackupKind, now: DateTime<Local>) -> PathBuf {
    let timestamp = now.format("%Y%m%d_%H%M%S");
    let base_name = format!(
        "{BACKUP_PREFIX}{}-{timestamp}.db",
        backup_kind_file_part(kind)
    );
    let mut path = backup_dir.join(&base_name);
    let mut suffix = 2;

    while path.exists() {
        path = backup_dir.join(format!(
            "{BACKUP_PREFIX}{}-{timestamp}-{suffix}.db",
            backup_kind_file_part(kind)
        ));
        suffix += 1;
    }

    path
}

fn backup_kind_file_part(kind: BackupKind) -> &'static str {
    match kind {
        BackupKind::Auto => "auto",
        BackupKind::Manual => "manual",
        BackupKind::PreRestore => "pre-restore",
        BackupKind::Unknown => "unknown",
    }
}

fn backup_kind_from_file_name(file_name: &str) -> BackupKind {
    if file_name.starts_with("attendance-auto-") {
        BackupKind::Auto
    } else if file_name.starts_with("attendance-manual-") {
        BackupKind::Manual
    } else if file_name.starts_with("attendance-pre-restore-") {
        BackupKind::PreRestore
    } else {
        BackupKind::Unknown
    }
}

fn is_app_backup_file(file_name: &str) -> bool {
    file_name.starts_with(BACKUP_PREFIX) && file_name.ends_with(".db")
}

fn backup_timestamp_from_file_name(file_name: &str) -> Option<i64> {
    let timestamp = file_name
        .strip_prefix("attendance-auto-")
        .or_else(|| file_name.strip_prefix("attendance-manual-"))
        .or_else(|| file_name.strip_prefix("attendance-pre-restore-"))?
        .trim_end_matches(".db");
    let timestamp = timestamp.split('-').next().unwrap_or(timestamp);
    let naive = NaiveDateTime::parse_from_str(timestamp, "%Y%m%d_%H%M%S").ok()?;
    Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
        .map(|value| value.timestamp())
}

fn metadata_timestamp(metadata: &fs::Metadata) -> Result<i64> {
    let modified: DateTime<Utc> = metadata
        .modified()
        .context("failed to read file modified time")?
        .into();
    Ok(modified.timestamp())
}

fn open_table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table_name],
        |row| row.get(0),
    )?;
    Ok(exists > 0)
}

fn require_core_tables(conn: &Connection) -> Result<()> {
    for table in ["classes", "students", "events", "settings"] {
        if !open_table_exists(conn, table)? {
            bail!("Backup is not an EES-AMS database: missing {table} table");
        }
    }
    Ok(())
}

fn count_table_rows(conn: &Connection, table_name: &str) -> Result<i64> {
    if !open_table_exists(conn, table_name)? {
        return Ok(0);
    }

    conn.query_row(&format!("SELECT COUNT(*) FROM {table_name}"), [], |row| {
        row.get(0)
    })
    .map_err(Into::into)
}

fn read_schema_version(conn: &Connection) -> Result<i32> {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(Into::into)
}

fn run_integrity_check(conn: &Connection) -> Result<()> {
    let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if result == "ok" {
        Ok(())
    } else {
        bail!("Backup failed SQLite integrity check: {result}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::init_db;
    use rusqlite::params;

    fn seed_student(pool: &DbPool, name: &str) {
        let conn = pool.get().expect("database connection should be available");
        conn.execute(
            "INSERT INTO students (id, name, gender, card_serial, class_id, created_at)
             VALUES (?1, ?2, NULL, NULL, NULL, 0)",
            params![uuid::Uuid::new_v4().to_string(), name],
        )
        .expect("student should be inserted");
    }

    #[test]
    fn online_backup_creates_readable_sqlite_backup() {
        let app_dir = tempfile::tempdir().expect("app data dir should be created");
        let db_file = tempfile::NamedTempFile::new().expect("database file should be created");
        let pool = init_db(db_file.path()).expect("database should initialize");
        seed_student(&pool, "Ada Lovelace");

        let summary = create_backup_at(&pool, app_dir.path(), BackupKind::Manual, Local::now())
            .expect("backup should be created");
        let preview = preview_backup(Path::new(&summary.path)).expect("backup should be readable");

        assert_eq!(preview.student_count, 1);
        assert_eq!(preview.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn daily_policy_skips_duplicate_backups_for_same_local_day() {
        let app_dir = tempfile::tempdir().expect("app data dir should be created");
        let db_file = tempfile::NamedTempFile::new().expect("database file should be created");
        let pool = init_db(db_file.path()).expect("database should initialize");
        let now = Local
            .with_ymd_and_hms(2026, 6, 3, 8, 0, 0)
            .single()
            .expect("test time should be valid");

        let first = ensure_daily_backup_at(&pool, app_dir.path(), now)
            .expect("first backup check should succeed");
        let second = ensure_daily_backup_at(&pool, app_dir.path(), now)
            .expect("second backup check should succeed");

        assert!(first.is_some());
        assert!(second.is_none());
        assert_eq!(list_backups(app_dir.path()).unwrap().len(), 1);
    }

    #[test]
    fn retention_keeps_newest_thirty_app_backups() {
        let app_dir = tempfile::tempdir().expect("app data dir should be created");
        let backup_dir = backup_dir(app_dir.path());
        fs::create_dir_all(&backup_dir).expect("backup dir should be created");
        for second in 1..=35 {
            let file = backup_dir.join(format!("attendance-auto-20260101_0800{second:02}.db"));
            fs::write(file, b"not validated by retention").expect("fixture should be written");
        }

        enforce_retention(app_dir.path()).expect("retention should succeed");
        let backups = list_backups(app_dir.path()).expect("backups should be listed");

        assert_eq!(backups.len(), RETENTION_LIMIT);
        assert!(backups
            .iter()
            .all(|backup| !backup.file_name.contains("20260101_080001")));
    }

    #[test]
    fn restore_preview_rejects_corrupt_and_newer_schema_files() {
        let app_dir = tempfile::tempdir().expect("app data dir should be created");
        let corrupt = app_dir.path().join("corrupt.db");
        fs::write(&corrupt, b"not sqlite").expect("corrupt fixture should be written");
        assert!(preview_backup(&corrupt).is_err());

        let newer = app_dir.path().join("newer.db");
        let conn = Connection::open(&newer).expect("newer fixture should open");
        conn.execute_batch("PRAGMA user_version = 999;")
            .expect("newer schema version should be written");

        let error = preview_backup(&newer).expect_err("newer schema should be rejected");
        assert!(error.to_string().contains("newer than this app supports"));
    }

    #[test]
    fn restore_creates_safety_backup_and_replaces_database_contents() {
        let app_dir = tempfile::tempdir().expect("app data dir should be created");
        let db_file = tempfile::NamedTempFile::new().expect("database file should be created");
        let pool = init_db(db_file.path()).expect("database should initialize");
        seed_student(&pool, "Original Student");
        let source = create_backup_at(&pool, app_dir.path(), BackupKind::Manual, Local::now())
            .expect("source backup should be created");
        seed_student(&pool, "Later Student");

        let result = restore_backup(&pool, app_dir.path(), Path::new(&source.path))
            .expect("restore should succeed");
        let conn = pool.get().expect("database connection should be available");
        let student_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM students", [], |row| row.get(0))
            .expect("student count should be readable");

        assert_eq!(student_count, 1);
        assert!(Path::new(&result.pre_restore_backup_path).exists());
    }

    #[test]
    fn sync_folder_copy_is_non_destructive_and_reports_success() {
        let app_dir = tempfile::tempdir().expect("app data dir should be created");
        let sync_dir = tempfile::tempdir().expect("sync dir should be created");
        let db_file = tempfile::NamedTempFile::new().expect("database file should be created");
        let pool = init_db(db_file.path()).expect("database should initialize");
        set_sync_folder(app_dir.path(), Some(sync_dir.path().to_path_buf()))
            .expect("sync folder should be configured");

        let status = create_manual_backup(&pool, app_dir.path()).expect("backup should succeed");
        let backup_name = Path::new(status.last_backup_path.as_ref().unwrap())
            .file_name()
            .unwrap()
            .to_owned();

        assert!(sync_dir
            .path()
            .join(SYNC_BACKUP_DIR_NAME)
            .join(backup_name)
            .exists());
        assert_eq!(status.last_sync_error, None);
    }

    #[test]
    fn set_sync_folder_creates_app_backup_subfolder() {
        let app_dir = tempfile::tempdir().expect("app data dir should be created");
        let drive_dir = tempfile::tempdir().expect("drive dir should be created");

        let status = set_sync_folder(app_dir.path(), Some(drive_dir.path().to_path_buf()))
            .expect("sync folder should be configured");

        let sync_path = drive_dir.path().join(SYNC_BACKUP_DIR_NAME);
        let expected = sync_path.to_string_lossy().to_string();
        assert!(sync_path.is_dir());
        assert_eq!(status.sync_folder_path.as_deref(), Some(expected.as_str()));
    }
}
