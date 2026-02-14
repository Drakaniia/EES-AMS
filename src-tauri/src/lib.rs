// Layered Architecture for EES-AMS
// Clean Architecture with Domain, Infrastructure, and Application layers

mod domain;
mod infrastructure;
mod application;

use tauri::Manager;
use domain::{
    services::{ClassService, StudentService, AttendanceService, ClassServiceImpl, StudentServiceImpl, AttendanceServiceImpl},
    repositories::{ClassRepository, StudentRepository, AttendanceRepository, SettingsRepository, UserRepository},
};
use infrastructure::{
    JsonDatabase,
    ClassRepositoryImpl,
    StudentRepositoryImpl,
    AttendanceRepositoryImpl,
    SettingsRepositoryImpl,
    UserRepositoryImpl,
    GoogleSync,
};
use application::{
    handlers::{ClassHandler, StudentHandler, AttendanceHandler, GoogleHandler, AuthHandler},
    commands::{AppState, 
        class_create, class_get_all, class_delete,
        student_create, student_get_all, student_get_by_class, student_delete, student_import_from_excel,
        attendance_record, attendance_get_by_class_and_date, attendance_get_unsynced, attendance_get_stats,
        google_save_credentials, google_is_authenticated, google_start_auth, google_handle_callback, google_logout, google_sync, google_get_sync_status,
        auth_register, auth_login, auth_validate_token, auth_get_current_user, auth_update_profile, auth_logout,
        fs_write_file, fs_remove_file,
        check_for_updates, download_and_install_update, restart_app
    },
};
use std::sync::{Arc, Mutex};
use std::env;

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
            let db = Arc::new(db);

            // Initialize repositories
            let class_repo = ClassRepositoryImpl::new(db.as_ref().clone());
            let student_repo = StudentRepositoryImpl::new(db.as_ref().clone());
            let student_repo2 = StudentRepositoryImpl::new(db.as_ref().clone());
            let attendance_repo = AttendanceRepositoryImpl::new(db.as_ref().clone());
            let settings_repo = SettingsRepositoryImpl::new(db.as_ref().clone());
            let user_repo = Arc::new(UserRepositoryImpl::new(app_data_dir.clone()));

            // Initialize domain services
            let class_service = ClassServiceImpl::new(class_repo);
            let student_service = StudentServiceImpl::new(student_repo);
            let attendance_service = AttendanceServiceImpl::new(attendance_repo, student_repo2);

            // Initialize Google Sync
            let google_sync = Arc::new(Mutex::new(GoogleSync::new()));
            
            // Load saved credentials and token
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let settings_repo_clone = settings_repo.clone();
            runtime.block_on(async {
                if let Some(Ok(creds_json)) = settings_repo_clone.get("google_credentials").await {
                    if let Ok(credentials) = serde_json::from_str(&creds_json) {
                        google_sync.lock().unwrap().set_credentials(credentials);
                    }
                }
                
                if let Some(Ok(token_json)) = settings_repo_clone.get("google_token").await {
                    if let Ok(token) = serde_json::from_str(&token_json) {
                        google_sync.lock().unwrap().set_token(token);
                    }
                }
            });

            // Initialize auth service
            let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "default_jwt_secret_change_in_production".to_string());
            let auth_service = domain::services::AuthService::new(
                Box::new((*user_repo).clone()),
                jwt_secret
            );
            let auth_handler = Arc::new(
                Mutex::new(AuthHandler::new(auth_service))
            );

            // Initialize application layer handlers
            let class_handler = Arc::new(
                ClassHandler::new(class_service)
            );
            let student_handler = Arc::new(
                StudentHandler::new(student_service)
            );
            let attendance_handler = Arc::new(
                AttendanceHandler::new(attendance_service)
            );
            let google_handler = Arc::new(
                GoogleHandler::new(google_sync.clone(), settings_repo, attendance_service, class_service)
            );

            // Create app state
            let state = AppState {
                class_handler,
                student_handler,
                attendance_handler,
                google_handler,
                auth_handler,
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
            student_import_from_excel,
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
            // auth_register,
            // auth_login,
            // auth_validate_token,
            // auth_get_current_user,
            // auth_update_profile,
            // auth_logout,
            fs_write_file,
            fs_remove_file,
            // check_for_updates,
            // download_and_install_update,
            // restart_app,
        ])
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
