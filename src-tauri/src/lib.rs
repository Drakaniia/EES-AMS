// Layered Architecture for EES-AMS
// Clean Architecture with Domain, Infrastructure, and Application layers

mod domain;
mod infrastructure;
mod application;

use domain::{
    services::{ClassService, StudentService, AttendanceService, ClassServiceImpl, StudentServiceImpl, AttendanceServiceImpl},
    repositories::{ClassRepository, StudentRepository, AttendanceRepository, SettingsRepository},
};
use infrastructure::{
    JsonDatabase,
    ClassRepositoryImpl,
    StudentRepositoryImpl,
    AttendanceRepositoryImpl,
    SettingsRepositoryImpl,
    GoogleSync,
};
use application::{
    handlers::{ClassHandler, StudentHandler, AttendanceHandler, GoogleHandler},
    commands::{AppState, 
        class_create, class_get_all, class_delete,
        student_create, student_get_all, student_get_by_class, student_delete,
        attendance_record, attendance_get_by_class_and_date, attendance_get_unsynced, attendance_get_stats,
        google_save_credentials, google_is_authenticated, google_start_auth, google_handle_callback, google_logout, google_sync, google_get_sync_status
    },
};
use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Get app data directory
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data directory");
            
            // Create directory if it doesn't exist
            std::fs::create_dir_all(&app_data_dir)
                .expect("Failed to create app data directory");

            // Initialize infrastructure layer
            let db = JsonDatabase::new(app_data_dir)
                .expect("Failed to initialize database");
            let db_arc = Arc::new(db);

            // Initialize repositories
            let class_repo = Arc::new(ClassRepositoryImpl::new(db_arc.clone()));
            let student_repo = Arc::new(StudentRepositoryImpl::new(db_arc.clone()));
            let attendance_repo = Arc::new(AttendanceRepositoryImpl::new(db_arc.clone()));
            let settings_repo = Arc::new(SettingsRepositoryImpl::new(db_arc.clone()));

            // Initialize domain services
            let class_service = Arc::new(
                ClassServiceImpl::new(class_repo.clone())
            );
            let student_service = Arc::new(
                StudentServiceImpl::new(student_repo.clone())
            );
            let attendance_service = Arc::new(
                AttendanceServiceImpl::new(attendance_repo.clone(), student_repo.clone())
            );

            // Initialize Google Sync
            let google_sync = Arc::new(Mutex::new(GoogleSync::new()));
            
            // Load saved credentials and token
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                if let Some(Ok(creds_json)) = settings_repo.get("google_credentials").await {
                    if let Ok(credentials) = serde_json::from_str(&creds_json) {
                        google_sync.lock().unwrap().set_credentials(credentials);
                    }
                }
                
                if let Some(Ok(token_json)) = settings_repo.get("google_token").await {
                    if let Ok(token) = serde_json::from_str(&token_json) {
                        google_sync.lock().unwrap().set_token(token);
                    }
                }
            });

            // Initialize application layer handlers
            let class_handler = Arc::new(
                ClassHandler::new(class_service.clone())
            );
            let student_handler = Arc::new(
                StudentHandler::new(student_service.clone())
            );
            let attendance_handler = Arc::new(
                AttendanceHandler::new(attendance_service.clone())
            );
            let google_handler = Arc::new(
                GoogleHandler::new(google_sync.clone(), settings_repo.clone(), attendance_service.clone(), class_service.clone())
            );

            // Create app state
            let state = AppState {
                class_handler,
                student_handler,
                attendance_handler,
                google_handler,
            };

            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            class_create,
            class_get_all,
            class_delete,
            student_create,
            student_get_all,
            student_get_by_class,
            student_delete,
            attendance_record,
            attendance_get_by_class_and_date,
            attendance_get_unsynced,
            attendance_get_stats,
            google_save_credentials,
            google_is_authenticated,
            google_start_auth,
            google_handle_callback,
            google_logout,
            google_sync,
            google_get_sync_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
