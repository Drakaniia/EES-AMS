// Google Handler
// Application-level handler for Google sync operations

use crate::infrastructure::external::{GoogleSync, GoogleCredentials, TokenData};
use crate::domain::repositories::SettingsRepository;
use crate::domain::entities::attendance::Attendance;
use crate::domain::entities::class::Class;
use crate::domain::entities::student::Student;
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
        let mut sync = self.google_sync.lock().unwrap();
        sync.set_credentials(credentials.clone());
        
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
        let sync = self.google_sync.lock().unwrap().clone();
        drop(sync);
        
        let sync_ref = self.google_sync.lock().unwrap();
        match sync_ref.exchange_code(code).await {
            Ok(token) => {
                if let Ok(json) = serde_json::to_string(&token) {
                    let _ = self.settings_repo.set("google_token".to_string(), json).await;
                }
                ApiResponse::success(true)
            }
            Err(e) => ApiResponse::error(e),
        }
    }

    pub async fn logout(&self) -> ApiResponse<()> {
        let sync = self.google_sync.lock().unwrap();
        sync.logout();
        let _ = self.settings_repo.set("google_token".to_string(), "".to_string()).await;
        ApiResponse::success_empty()
    }

    pub async fn sync(&self) -> ApiResponse<bool> {
        let sync_guard = self.google_sync.lock().unwrap();
        
        if sync_guard.get_is_syncing() {
            return ApiResponse::error("Already syncing".to_string());
        }
        
        if !sync_guard.is_authenticated() {
            return ApiResponse::error("Not authenticated".to_string());
        }
        
        sync_guard.set_syncing(true);
        sync_guard.set_error(None);
        drop(sync_guard);

        let sync_result = self.perform_sync().await;

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

    async fn perform_sync(&self) -> Result<(), String> {
        // Get root folder
        let sync = self.google_sync.lock().unwrap();
        let root_folder_result = sync.get_or_create_folder("Attendance Management System", None).await;
        drop(sync);

        let root_folder_id = match root_folder_result {
            Ok(id) => id,
            Err(e) => return Err(e),
        };

        let classes = match self.class_service.get_all_classes().await {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };

        for class in classes {
            let class_name = format!("{}{}", 
                class.name, 
                class.section.as_ref().map(|s| format!(" - {}", s)).unwrap_or_default()
            );

            let sync = self.google_sync.lock().unwrap();
            let class_folder_result = sync.get_or_create_folder(&class_name, Some(&root_folder_id)).await;
            drop(sync);

            let class_folder_id = match class_folder_result {
                Ok(id) => id,
                Err(_) => continue,
            };

            let month_year = Utc::now().format("%B %Y").to_string();
            let spreadsheet_key = format!("spreadsheet_{}_{}", class.id, month_year.replace(" ", "_"));
            
            let spreadsheet_id = self.settings_repo.get(&spreadsheet_key).await?;

            if spreadsheet_id.is_none() {
                let sync = self.google_sync.lock().unwrap();
                let title = format!("Attendance - {}", month_year);
                let result = sync.create_spreadsheet(&title, Some(&class_folder_id)).await;
                drop(sync);

                if let Ok(id) = result {
                    let _ = self.settings_repo.set(spreadsheet_key.clone(), id).await;
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

                let sync = self.google_sync.lock().unwrap();
                let append_result = sync.append_sheet_values(&sheet_id, "Attendance!A:E", formatted_records).await;
                drop(sync);

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
        let sync = self.google_sync.lock().unwrap();
        let unsynced = match self.attendance_service.get_unsynced_records().await {
            Ok(r) => r,
            Err(_) => vec![],
        };
        let last_sync = self.settings_repo.get("last_sync_time").await.ok().flatten();

        let status = SyncStatus {
            is_online: true,
            last_sync_time: last_sync,
            pending_records: unsynced.len() as i32,
            is_syncing: sync.get_is_syncing(),
            error: sync.get_error(),
        };

        ApiResponse::success(status)
    }
}