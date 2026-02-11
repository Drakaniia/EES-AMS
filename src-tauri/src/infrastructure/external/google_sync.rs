// Google Sheets Sync Service
// Infrastructure layer - External service integration

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenUrl,
    AuthorizationCode, TokenResponse, basic::BasicClient, reqwest::async_http_client,
};
use reqwest;
use std::collections::HashMap;

const SHEETS_API: &str = "https://sheets.googleapis.com/v4/spreadsheets";
const DRIVE_API: &str = "https://www.googleapis.com/drive/v3/files";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

pub struct GoogleSync {
    credentials: Option<GoogleCredentials>,
    token: Arc<Mutex<Option<TokenData>>>,
    is_syncing: Arc<Mutex<bool>>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl GoogleSync {
    pub fn new() -> Self {
        GoogleSync {
            credentials: None,
            token: Arc::new(Mutex::new(None)),
            is_syncing: Arc::new(Mutex::new(false)),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_credentials(&mut self, credentials: GoogleCredentials) {
        self.credentials = Some(credentials);
    }

    pub fn set_token(&self, token: TokenData) {
        let mut t = self.token.lock().unwrap();
        *t = Some(token);
    }

    pub fn get_token(&self) -> Option<TokenData> {
        let t = self.token.lock().unwrap();
        t.clone()
    }

    pub fn is_authenticated(&self) -> bool {
        let t = self.token.lock().unwrap();
        t.is_some()
    }

    pub fn generate_auth_url(&self) -> Result<String, String> {
        let creds = self.credentials.as_ref()
            .ok_or("Credentials not set")?;

        let client = BasicClient::new(
            ClientId::new(creds.client_id.clone()),
            Some(ClientSecret::new(creds.client_secret.clone())),
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
                .map_err(|e| format!("Invalid auth URL: {}", e))?,
            Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                .map_err(|e| format!("Invalid token URL: {}", e))?)
        )
        .set_redirect_uri(
            RedirectUrl::new(creds.redirect_uri.clone())
                .map_err(|e| format!("Invalid redirect URI: {}", e))?
        );

        let (auth_url, _csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(oauth2::Scope::new("https://www.googleapis.com/auth/spreadsheets".to_string()))
            .add_scope(oauth2::Scope::new("https://www.googleapis.com/auth/drive.file".to_string()))
            .url();

        Ok(auth_url.to_string())
    }

    pub async fn exchange_code(&self, code: String) -> Result<TokenData, String> {
        let creds = self.credentials.as_ref()
            .ok_or("Credentials not set")?;

        let client = BasicClient::new(
            ClientId::new(creds.client_id.clone()),
            Some(ClientSecret::new(creds.client_secret.clone())),
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
                .map_err(|e| format!("Invalid auth URL: {}", e))?,
            Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                .map_err(|e| format!("Invalid token URL: {}", e))?)
        )
        .set_redirect_uri(
            RedirectUrl::new(creds.redirect_uri.clone())
                .map_err(|e| format!("Invalid redirect URI: {}", e))?
        );

        let token_result = client
            .exchange_code(AuthorizationCode::new(code))
            .request_async(async_http_client)
            .await
            .map_err(|e| format!("Token exchange failed: {}", e))?;

        let token_data = TokenData {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
            expires_in: token_result.expires_in().map(|d| d.as_secs()),
        };

        self.set_token(token_data.clone());
        Ok(token_data)
    }

    pub fn logout(&self) {
        let mut t = self.token.lock().unwrap();
        *t = None;
    }

    async fn fetch_with_auth(&self, url: &str, method: &str, body: Option<serde_json::Value>) -> Result<reqwest::Response, String> {
        let token = self.get_token()
            .ok_or("Not authenticated")?;

        let client = reqwest::Client::new();
        let mut request = match method {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "PATCH" => client.patch(url),
            _ => return Err("Unsupported HTTP method".to_string()),
        };

        request = request
            .header("Authorization", format!("Bearer {}", token.access_token))
            .header("Content-Type", "application/json");

        if let Some(body_data) = body {
            request = request.json(&body_data);
        }

        request.send().await
            .map_err(|e| format!("Request failed: {}", e))
    }

    pub async fn create_folder(&self, name: &str, parent_id: Option<&str>) -> Result<String, String> {
        let mut metadata = serde_json::json!({
            "name": name,
            "mimeType": "application/vnd.google-apps.folder"
        });

        if let Some(parent) = parent_id {
            metadata["parents"] = serde_json::json!([parent]);
        }

        let response = self.fetch_with_auth(DRIVE_API, "POST", Some(metadata)).await?;
        
        if !response.status().is_success() {
            return Err(format!("Drive API error: {}", response.status()));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        data["id"].as_str()
            .map(|s| s.to_string())
            .ok_or("No folder ID in response".to_string())
    }

    pub async fn get_or_create_folder(&self, name: &str, parent_id: Option<&str>) -> Result<String, String> {
        let mut query = format!("name='{}' and mimeType='application/vnd.google-apps.folder' and trashed=false", name);
        if let Some(parent) = parent_id {
            query.push_str(&format!(" and '{}' in parents", parent));
        }

        let search_url = format!("{}?q={}&fields=files(id,name)", DRIVE_API, urlencoding::encode(&query));
        let response = self.fetch_with_auth(&search_url, "GET", None).await?;

        if !response.status().is_success() {
            return Err(format!("Drive API error: {}", response.status()));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(files) = data["files"].as_array() {
            if !files.is_empty() {
                if let Some(id) = files[0]["id"].as_str() {
                    return Ok(id.to_string());
                }
            }
        }

        self.create_folder(name, parent_id).await
    }

    pub async fn create_spreadsheet(&self, title: &str, folder_id: Option<&str>) -> Result<String, String> {
        let spreadsheet_body = serde_json::json!({
            "properties": { "title": title },
            "sheets": [
                { "properties": { "title": "Attendance" } },
                { "properties": { "title": "Students" } },
                { "properties": { "title": "Summary" } }
            ]
        });

        let response = self.fetch_with_auth(SHEETS_API, "POST", Some(spreadsheet_body)).await?;

        if !response.status().is_success() {
            return Err(format!("Sheets API error: {}", response.status()));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let spreadsheet_id = data["spreadsheetId"].as_str()
            .ok_or("No spreadsheet ID in response")?
            .to_string();

        if let Some(folder) = folder_id {
            let move_url = format!("{}?addParents={}", 
                format!("{}/{}", DRIVE_API, spreadsheet_id), folder);
            let _ = self.fetch_with_auth(&move_url, "PATCH", None).await;
        }

        self.update_sheet_values(&spreadsheet_id, "Attendance!A1:F1", 
            vec![vec!["Date", "Student ID", "Student Name", "Status", "Notes", "Recorded At"]]).await?;

        self.update_sheet_values(&spreadsheet_id, "Students!A1:E1",
            vec![vec!["Student ID", "First Name", "Last Name", "Class", "Added At"]]).await?;

        Ok(spreadsheet_id)
    }

    async fn update_sheet_values(&self, spreadsheet_id: &str, range: &str, values: Vec<Vec<&str>>) -> Result<(), String> {
        let url = format!("{}/{}/values/{}?valueInputOption=RAW", 
            SHEETS_API, spreadsheet_id, urlencoding::encode(range));

        let body = serde_json::json!({ "values": values });
        let response = self.fetch_with_auth(&url, "PUT", Some(body)).await?;

        if !response.status().is_success() {
            return Err(format!("Failed to update sheet values: {}", response.status()));
        }

        Ok(())
    }

    pub async fn append_sheet_values(&self, spreadsheet_id: &str, range: &str, values: Vec<Vec<String>>) -> Result<(), String> {
        let url = format!("{}/{}/values/{}:append?valueInputOption=RAW&insertDataOption=INSERT_ROWS",
            SHEETS_API, spreadsheet_id, urlencoding::encode(range));

        let body = serde_json::json!({ "values": values });
        let response = self.fetch_with_auth(&url, "POST", Some(body)).await?;

        if !response.status().is_success() {
            return Err(format!("Failed to append sheet values: {}", response.status()));
        }

        Ok(())
    }

    pub fn set_syncing(&self, syncing: bool) {
        let mut s = self.is_syncing.lock().unwrap();
        *s = syncing;
    }

    pub fn get_is_syncing(&self) -> bool {
        let s = self.is_syncing.lock().unwrap();
        *s
    }

    pub fn set_error(&self, error: Option<String>) {
        let mut e = self.last_error.lock().unwrap();
        *e = error;
    }

    pub fn get_error(&self) -> Option<String> {
        let e = self.last_error.lock().unwrap();
        e.clone()
    }
}