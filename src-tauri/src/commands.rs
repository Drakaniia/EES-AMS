// Tauri Commands - Bridge between frontend and Rust backend

use crate::database::{Database, ClassRecord, StudentRecord, AttendanceRecordDB, AttendanceStats};
use crate::google_sync::{GoogleSync, GoogleCredentials, TokenData, SyncStatus};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::State;
use chrono::Utc;

pub struct AppState {
    pub db: Arc<Database>,
    pub google_sync: Arc<Mutex<GoogleSync>>,
}

#[derive(Debug, Serialize, Deserialize)]
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
    fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            id: None,
            error: None,
        }
    }

    fn success_with_id(id: i64) -> Self {
        ApiResponse {
            success: true,
            data: None,
            id: Some(id),
            error: None,
        }
    }

    fn success_empty() -> Self {
        ApiResponse {
            success: true,
            data: None,
            id: None,
            error: None,
        }
    }

    fn error(msg: String) -> Self {
        ApiResponse {
            success: false,
            data: None,
            id: None,
            error: Some(msg),
        }
    }
}

// ========================================
// Class Commands
// ========================================

#[derive(Debug, Deserialize)]
pub struct CreateClassInput {
    pub name: String,
    pub section: Option<String>,
    pub school_year: Option<String>,
}

#[tauri::command]
pub fn class_create(input: CreateClassInput, state: State<AppState>) -> ApiResponse<i64> {
    match state.db.create_class(input.name, input.section, input.school_year) {
        Ok(id) => ApiResponse::success_with_id(id),
        Err(e) => ApiResponse::error(e),
    }
}

#[tauri::command]
pub fn class_get_all(state: State<AppState>) -> ApiResponse<Vec<ClassRecord>> {
    match state.db.get_all_classes() {
        Ok(classes) => ApiResponse::success(classes),
        Err(e) => ApiResponse::error(e),
    }
}

#[tauri::command]
pub fn class_delete(id: i64, state: State<AppState>) -> ApiResponse<()> {
    match state.db.delete_class(id) {
        Ok(_) => ApiResponse::success_empty(),
        Err(e) => ApiResponse::error(e),
    }
}

// ========================================
// Student Commands
// ========================================

#[derive(Debug, Deserialize)]
pub struct CreateStudentInput {
    pub student_id: String,
    pub first_name: String,
    pub last_name: String,
    pub class_id: Option<i64>,
}

#[tauri::command]
pub fn student_create(input: CreateStudentInput, state: State<AppState>) -> ApiResponse<i64> {
    match state.db.create_student(input.student_id, input.first_name, input.last_name, input.class_id) {
        Ok(id) => ApiResponse::success_with_id(id),
        Err(e) => ApiResponse::error(e),
    }
}

#[tauri::command]
pub fn student_get_by_class(class_id: i64, state: State<AppState>) -> ApiResponse<Vec<StudentRecord>> {
    match state.db.get_students_by_class(class_id) {
        Ok(students) => ApiResponse::success(students),
        Err(e) => ApiResponse::error(e),
    }
}

#[tauri::command]
pub fn student_get_all(state: State<AppState>) -> ApiResponse<Vec<StudentRecord>> {
    match state.db.get_all_students() {
        Ok(students) => ApiResponse::success(students),
        Err(e) => ApiResponse::error(e),
    }
}

#[tauri::command]
pub fn student_delete(id: i64, state: State<AppState>) -> ApiResponse<()> {
    match state.db.delete_student(id) {
        Ok(_) => ApiResponse::success_empty(),
        Err(e) => ApiResponse::error(e),
    }
}

// ========================================
// Attendance Commands
// ========================================

#[derive(Debug, Deserialize)]
pub struct RecordAttendanceInput {
    pub student_id: i64,
    pub class_id: i64,
    pub date: String,
    pub status: String,
    pub notes: Option<String>,
}

#[tauri::command]
pub fn attendance_record(input: RecordAttendanceInput, state: State<AppState>) -> ApiResponse<i64> {
    match state.db.record_attendance(input.student_id, input.class_id, input.date, input.status, input.notes) {
        Ok(id) => ApiResponse::success_with_id(id),
        Err(e) => ApiResponse::error(e),
    }
}

#[tauri::command]
pub fn attendance_get_by_class_and_date(class_id: i64, date: String, state: State<AppState>) -> ApiResponse<Vec<AttendanceRecordDB>> {
    match state.db.get_attendance_by_class_and_date(class_id, date) {
        Ok(records) => ApiResponse::success(records),
        Err(e) => ApiResponse::error(e),
    }
}

#[tauri::command]
pub fn attendance_get_unsynced(state: State<AppState>) -> ApiResponse<Vec<AttendanceRecordDB>> {
    match state.db.get_unsynced_records() {
        Ok(records) => ApiResponse::success(records),
        Err(e) => ApiResponse::error(e),
    }
}

#[tauri::command]
pub fn attendance_get_stats(class_id: i64, state: State<AppState>) -> ApiResponse<AttendanceStats> {
    match state.db.get_today_stats(class_id) {
        Ok(stats) => ApiResponse::success(stats),
        Err(e) => ApiResponse::error(e),
    }
}

// ========================================
// Google Sync Commands
// ========================================

#[tauri::command]
pub fn google_save_credentials(credentials: GoogleCredentials, state: State<AppState>) -> ApiResponse<()> {
    let mut sync = state.google_sync.lock().unwrap();
    sync.set_credentials(credentials.clone());
    
    // Save to database settings
    if let Ok(json) = serde_json::to_string(&credentials) {
        let _ = state.db.set_setting("google_credentials".to_string(), json);
    }
    
    ApiResponse::success_empty()
}

#[tauri::command]
pub fn google_is_authenticated(state: State<AppState>) -> ApiResponse<bool> {
    let sync = state.google_sync.lock().unwrap();
    ApiResponse::success(sync.is_authenticated())
}

#[tauri::command]
pub fn google_start_auth(state: State<AppState>) -> ApiResponse<String> {
    let sync = state.google_sync.lock().unwrap();
    match sync.generate_auth_url() {
        Ok(url) => ApiResponse::success(url),
        Err(e) => ApiResponse::error(e),
    }
}

#[tauri::command]
pub async fn google_handle_callback(code: String, state: State<'_, AppState>) -> Result<ApiResponse<bool>, String> {
    let sync = state.google_sync.lock().unwrap().clone();
    drop(sync);
    
    let sync_ref = state.google_sync.lock().unwrap();
    match sync_ref.exchange_code(code).await {
        Ok(token) => {
            // Save token to database
            if let Ok(json) = serde_json::to_string(&token) {
                let _ = state.db.set_setting("google_token".to_string(), json);
            }
            Ok(ApiResponse::success(true))
        }
        Err(e) => Ok(ApiResponse::error(e)),
    }
}

#[tauri::command]
pub fn google_logout(state: State<AppState>) -> ApiResponse<()> {
    let sync = state.google_sync.lock().unwrap();
    sync.logout();
    let _ = state.db.set_setting("google_token".to_string(), "".to_string());
    ApiResponse::success_empty()
}

#[tauri::command]
pub async fn google_sync(state: State<'_, AppState>) -> Result<ApiResponse<bool>, String> {
    let sync_guard = state.google_sync.lock().unwrap();
    
    if sync_guard.get_is_syncing() {
        return Ok(ApiResponse::error("Already syncing".to_string()));
    }
    
    if !sync_guard.is_authenticated() {
        return Ok(ApiResponse::error("Not authenticated".to_string()));
    }
    
    sync_guard.set_syncing(true);
    sync_guard.set_error(None);
    drop(sync_guard);

    // Get root folder
    let sync = state.google_sync.lock().unwrap();
    let root_folder_result = sync.get_or_create_folder("Attendance Management System", None).await;
    drop(sync);

    let root_folder_id = match root_folder_result {
        Ok(id) => id,
        Err(e) => {
            let sync = state.google_sync.lock().unwrap();
            sync.set_syncing(false);
            sync.set_error(Some(e.clone()));
            return Ok(ApiResponse::error(e));
        }
    };

    // Get all classes
    let classes = match state.db.get_all_classes() {
        Ok(c) => c,
        Err(e) => {
            let sync = state.google_sync.lock().unwrap();
            sync.set_syncing(false);
            sync.set_error(Some(e.clone()));
            return Ok(ApiResponse::error(e));
        }
    };

    for class in classes {
        let class_name = format!("{}{}", 
            class.name, 
            class.section.as_ref().map(|s| format!(" - {}", s)).unwrap_or_default()
        );

        // Get or create class folder
        let sync = state.google_sync.lock().unwrap();
        let class_folder_result = sync.get_or_create_folder(&class_name, Some(&root_folder_id)).await;
        drop(sync);

        let class_folder_id = match class_folder_result {
            Ok(id) => id,
            Err(_) => continue,
        };

        // Get or create spreadsheet for current month
        let month_year = Utc::now().format("%B %Y").to_string();
        let spreadsheet_key = format!("spreadsheet_{}_{}", class.id, month_year.replace(" ", "_"));
        
        let mut spreadsheet_id = state.db.get_setting(&spreadsheet_key);

        if spreadsheet_id.is_none() {
            let sync = state.google_sync.lock().unwrap();
            let title = format!("Attendance - {}", month_year);
            let result = sync.create_spreadsheet(&title, Some(&class_folder_id)).await;
            drop(sync);

            if let Ok(id) = result {
                let _ = state.db.set_setting(spreadsheet_key.clone(), id.clone());
                spreadsheet_id = Some(id);
            }
        }

        if let Some(sheet_id) = spreadsheet_id {
            // Get unsynced records for this class
            let unsynced = match state.db.get_unsynced_records() {
                Ok(records) => records.into_iter().filter(|r| r.class_id == class.id).collect::<Vec<_>>(),
                Err(_) => continue,
            };

            if unsynced.is_empty() {
                continue;
            }

            // Get students
            let students = match state.db.get_students_by_class(class.id) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let student_map: std::collections::HashMap<i64, &StudentRecord> = 
                students.iter().map(|s| (s.id, s)).collect();

            // Format records
            let formatted_records: Vec<Vec<String>> = unsynced.iter().map(|record| {
                let student = student_map.get(&record.student_id);
                vec![
                    record.date.clone(),
                    student.map(|s| s.student_id.clone()).unwrap_or_else(|| "Unknown".to_string()),
                    student.map(|s| format!("{} {}", s.first_name, s.last_name)).unwrap_or_else(|| "Unknown".to_string()),
                    record.status.chars().next().unwrap().to_uppercase().collect::<String>() + &record.status[1..],
                    record.notes.clone().unwrap_or_default(),
                    record.created_at.clone(),
                ]
            }).collect();

            // Append to sheets
            let sync = state.google_sync.lock().unwrap();
            let append_result = sync.append_sheet_values(&sheet_id, "Attendance!A:F", formatted_records).await;
            drop(sync);

            if append_result.is_ok() {
                let record_ids: Vec<i64> = unsynced.iter().map(|r| r.id).collect();
                let _ = state.db.mark_as_synced(record_ids);
            }
        }
    }

    // Update last sync time
    let _ = state.db.set_setting("last_sync_time".to_string(), Utc::now().to_rfc3339());

    let sync = state.google_sync.lock().unwrap();
    sync.set_syncing(false);
    
    Ok(ApiResponse::success(true))
}

#[tauri::command]
pub fn google_get_sync_status(state: State<AppState>) -> ApiResponse<SyncStatus> {
    let sync = state.google_sync.lock().unwrap();
    let unsynced = state.db.get_unsynced_records().unwrap_or_default();
    let last_sync = state.db.get_setting("last_sync_time");

    let status = SyncStatus {
        is_online: true,
        last_sync_time: last_sync,
        pending_records: unsynced.len() as i32,
        is_syncing: sync.get_is_syncing(),
        error: sync.get_error(),
    };

    ApiResponse::success(status)
}
