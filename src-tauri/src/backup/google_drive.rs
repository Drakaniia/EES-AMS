use super::file_ops::{
    load_state, save_state, BackupState, GoogleDriveState, KEYRING_REFRESH_TOKEN_USER,
    KEYRING_SERVICE,
};
use super::models::BackupStatus;
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use rand::{distributions::Alphanumeric, Rng};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use url::Url;

// ── Constants ─────────────────────────────────────────────────────────

const GOOGLE_DRIVE_FOLDER_NAME: &str = "EES-AMS Backups";
const GOOGLE_DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive";
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const GOOGLE_DRIVE_UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";

// ── Private Types ─────────────────────────────────────────────────────

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

// ── Public API ────────────────────────────────────────────────────────

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

    super::backup_ops::get_status(app_dir)
}

pub fn disconnect_google_drive(app_dir: &Path) -> Result<BackupStatus> {
    let mut state = load_state(app_dir).unwrap_or_default();
    state.google_drive = None;
    let _ = delete_refresh_token();
    save_state(app_dir, &state)?;
    super::backup_ops::get_status(app_dir)
}

pub fn upload_latest_backup_to_google_drive(app_dir: &Path) -> Result<BackupStatus> {
    let latest = super::backup_ops::list_backups(app_dir)?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No local backup is available to upload"))?;
    let mut state = load_state(app_dir).unwrap_or_default();
    upload_backup_to_google_drive(&mut state, Path::new(&latest.path))?;
    save_state(app_dir, &state)?;
    super::backup_ops::get_status(app_dir)
}

pub(crate) fn upload_backup_to_google_drive(
    state: &mut BackupState,
    source_path: &Path,
) -> Result<()> {
    let Some(google_drive) = state.google_drive.as_mut() else {
        return Ok(());
    };

    let access_token = refresh_google_access_token()?;
    let file_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow::anyhow!("backup file name is invalid"))?;
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

// ── Client ID ─────────────────────────────────────────────────────────

fn google_client_id() -> Result<String> {
    option_env!("EES_AMS_GOOGLE_CLIENT_ID")
        .map(str::to_string)
        .or_else(|| std::env::var("EES_AMS_GOOGLE_CLIENT_ID").ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Google Drive is not configured. Set EES_AMS_GOOGLE_CLIENT_ID before building the app."
            )
        })
}

// ── OAuth Flow ────────────────────────────────────────────────────────

fn authorize_google_drive(client_id: &str) -> Result<OAuthTokenResponse> {
    let listener =
        TcpListener::bind("127.0.0.1:0").context("failed to start OAuth callback")?;
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
        .ok_or_else(|| anyhow::anyhow!("OAuth callback was empty"))?;
    let path = first_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("OAuth callback path was missing"))?;
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

    code.ok_or_else(|| anyhow::anyhow!("Google sign-in did not return an authorization code"))
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

// ── Token Storage ─────────────────────────────────────────────────────

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
