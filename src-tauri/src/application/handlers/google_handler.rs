// Google Handler
// Application-level handler for Google sync operations

#![allow(dead_code)]

use crate::infrastructure::external::{GoogleSync, GoogleCredentials};
use crate::domain::repositories::SettingsRepository;
use crate::domain::services::{AttendanceService, ClassService};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use chrono::Utc;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            id: None,
            error: None,
        }
    }

    #[allow(dead_code)]
    pub fn success_with_id(id: i64) -> Self {
        ApiResponse {
            success: true,
            data: None,
            id: Some(id),
            error: None,
        }
    }

    pub fn success_empty() -> Self {
        ApiResponse {
            success: true,
            data: None,
            id: None,
            error: None,
        }
    }

    pub fn error(msg: String) -> Self {
        ApiResponse {
            success: false,
            data: None,
            id: None,
            error: Some(msg),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub is_online: bool,
    pub last_sync_time: Option<String>,
    pub pending_records: i32,
    pub is_syncing: bool,
    pub error: Option<String>,
}

pub struct GoogleHandler<
    SR: SettingsRepository,
    AS: AttendanceService,
    CS: ClassService,
> {
    google_sync: Arc<Mutex<GoogleSync>>,
    settings_repo: SR,
    attendance_service: AS,
    class_service: CS,
}

impl<SR: SettingsRepository, AS: AttendanceService, CS: ClassService>
    GoogleHandler<SR, AS, CS>
{
    pub fn new(
        google_sync: Arc<Mutex<GoogleSync>>,
        settings_repo: SR,
        attendance_service: AS,
        class_service: CS,
    ) -> Self {
        GoogleHandler {
            google_sync,
            settings_repo,
            attendance_service,
            class_service,
        }
    }

    pub async fn save_credentials(&self, credentials: GoogleCredentials) -> ApiResponse<()> {
        {
            let mut sync = self.google_sync.lock().unwrap();
            sync.set_credentials(credentials.clone());
        }
        
        if let Ok(json) = serde_json::to_string(&credentials) {
            let _ = self.settings_repo.set("google_credentials".to_string(), json).await;
        }
        
        ApiResponse::success_empty()
    }

    pub async fn is_authenticated(&self) -> ApiResponse<bool> {
        let sync = self.google_sync.lock().unwrap();
        ApiResponse::success(sync.is_authenticated())
    }

    pub async fn start_auth(&self) -> ApiResponse<String> {
        let sync = self.google_sync.lock().unwrap();
        match sync.generate_auth_url() {
            Ok(url) => ApiResponse::success(url),
            Err(e) => ApiResponse::error(e),
        }
    }

    pub async fn handle_callback(&self, code: String) -> ApiResponse<bool> {
        // Clone the sync to avoid holding MutexGuard across .await
        let sync = self.google_sync.lock().unwrap().clone();
        let token = match sync.exchange_code(code).await {
            Ok(token) => token,
            Err(e) => return ApiResponse::error(e),
        };

        // Save token back into the shared state
        self.google_sync.lock().unwrap().set_token(token.clone());

        if let Ok(json) = serde_json::to_string(&token) {
            let _ = self.settings_repo.set("google_token".to_string(), json).await;
        }
        ApiResponse::success(true)
    }

    pub async fn logout(&self) -> ApiResponse<()> {
        {
            let sync = self.google_sync.lock().unwrap();
            sync.logout();
        }
        let _ = self.settings_repo.set("google_token".to_string(), "".to_string()).await;
        ApiResponse::success_empty()
    }

    pub async fn sync(&self) -> ApiResponse<bool> {
        {
            let sync_guard = self.google_sync.lock().unwrap();
            
            if sync_guard.get_is_syncing() {
                return ApiResponse::error("Already syncing".to_string());
            }
            
            if !sync_guard.is_authenticated() {
                return ApiResponse::error("Not authenticated".to_string());
            }
            
            sync_guard.set_syncing(true);
            sync_guard.set_error(None);
        }

        let sync_result = self.perform_sync().await;

        {
            let sync = self.google_sync.lock().unwrap();
            sync.set_syncing(false);
            
            match sync_result {
                Ok(_) => ApiResponse::success(true),
                Err(e) => {
                    sync.set_error(Some(e.clone()));
                    ApiResponse::error(e)
                }
            }
        }
    }

    async fn perform_sync(&self) -> Result<(), String> {
        // Clone sync to avoid holding MutexGuard across .await
        let sync = self.google_sync.lock().unwrap().clone();
        let root_folder_id = sync.get_or_create_folder("Attendance Management System", None).await?;

        let classes = match self.class_service.get_all_classes().await {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };

        for class in classes {
            let class_name = format!("{}{}", 
                class.name, 
                class.section.as_ref().map(|s| format!(" - {}", s)).unwrap_or_default()
            );

            let sync = self.google_sync.lock().unwrap().clone();
            let class_folder_id = match sync.get_or_create_folder(&class_name, Some(&root_folder_id)).await {
                Ok(id) => id,
                Err(_) => continue,
            };

            let month_year = Utc::now().format("%B %Y").to_string();
            let spreadsheet_key = format!("spreadsheet_{}_{}", class.id, month_year.replace(" ", "_"));
            
            let mut spreadsheet_id = self.settings_repo.get(&spreadsheet_key).await.map_err(|e| e.to_string())?;

            if spreadsheet_id.is_none() {
                let sync = self.google_sync.lock().unwrap().clone();
                let title = format!("Attendance - {}", month_year);
                let result = sync.create_spreadsheet(&title, Some(&class_folder_id)).await;

                if let Ok(id) = result {
                    let _ = self.settings_repo.set(spreadsheet_key.clone(), id.clone()).await;
                    spreadsheet_id = Some(id);
                }
            }

            if let Some(sheet_id) = spreadsheet_id {
                let unsynced = match self.attendance_service.get_unsynced_records().await {
                    Ok(records) => records.into_iter().filter(|r| r.class_id == class.id).collect::<Vec<_>>(),
                    Err(_) => continue,
                };

                if unsynced.is_empty() {
                    continue;
                }

                // Format and append records (simplified for brevity)
                let formatted_records: Vec<Vec<String>> = unsynced.iter().map(|record| {
                    vec![
                        record.date.clone(),
                        format!("{}", record.student_id),
                        record.status.as_str().to_string(),
                        record.notes.clone().unwrap_or_default(),
                        record.created_at.clone(),
                    ]
                }).collect();

                let sync = self.google_sync.lock().unwrap().clone();
                let append_result = sync.append_sheet_values(&sheet_id, "Attendance!A:E", formatted_records).await;

                if append_result.is_ok() {
                    let record_ids: Vec<i64> = unsynced.iter().map(|r| r.id).collect();
                    let _ = self.attendance_service.mark_as_synced(record_ids).await;
                }
            }
        }

        let _ = self.settings_repo.set("last_sync_time".to_string(), Utc::now().to_rfc3339()).await;
        Ok(())
    }

    pub async fn get_sync_status(&self) -> ApiResponse<SyncStatus> {
        let unsynced = self.attendance_service.get_unsynced_records().await.unwrap_or_default();
        let last_sync = self.settings_repo.get("last_sync_time").await.ok().flatten();

        let (is_syncing, error) = {
            let sync = self.google_sync.lock().unwrap();
            (sync.get_is_syncing(), sync.get_error())
        };

        let status = SyncStatus {
            is_online: true,
            last_sync_time: last_sync,
            pending_records: unsynced.len() as i32,
            is_syncing,
            error,
        };

        ApiResponse::success(status)
    }

    /// Hybrid sync with Firebase backup and Google Sheets
    #[allow(dead_code)]
    pub async fn sync_with_data(&self, data: serde_json::Value) -> ApiResponse<bool> {
        // First save to Firebase as backup
        if let Err(e) = crate::infrastructure::external::firebase::save_data(&data).await {
            eprintln!("Warning: Failed to save to Firebase backup: {}", e);
        }

        // Then try to sync to Google Sheets if authenticated
        let is_authenticated = {
            let sync_guard = self.google_sync.lock().unwrap();
            sync_guard.is_authenticated()
        };
        
        if is_authenticated {
            let response = self.sync().await;
            if response.success {
                response
            } else {
                // If Google sync fails, we still have the Firebase backup
                ApiResponse::error(format!("Google sync failed, data saved to Firebase backup: {}", response.error.unwrap_or_default()))
            }
        } else {
            // Not authenticated to Google, but we have Firebase backup
            ApiResponse::success(false)
        }
    }
}