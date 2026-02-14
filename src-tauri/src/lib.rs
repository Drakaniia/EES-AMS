// Layered Architecture for EES-AMS
// Clean Architecture with Domain, Infrastructure, and Application layers

mod domain;
mod infrastructure;
mod application;

use tauri::Manager;
use domain::{
    services::{ClassServiceImpl, StudentServiceImpl, AttendanceServiceImpl},
    repositories::SettingsRepository,
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
    handlers::{ClassHandler, StudentHandler, AttendanceHandler, GoogleHandler},
    commands::{AppState, 
        class_create, class_get_all, class_delete,
        student_create, student_get_all, student_get_by_class, student_delete, student_import_from_excel,
        attendance_record, attendance_get_by_class_and_date, attendance_get_unsynced, attendance_get_stats,
        google_save_credentials, google_is_authenticated, google_start_auth, google_handle_callback, google_logout, google_sync, google_get_sync_status,
        fs_write_file, fs_remove_file
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

            // Clone for UserRepository before moving into JsonDatabase
            let app_data_dir_for_user = app_data_dir.clone();

            // Initialize infrastructure layer
            let db = JsonDatabase::new(app_data_dir)
                .expect("Failed to initialize database");
            let db = Arc::new(db);

            // Initialize repositories
            let class_repo = ClassRepositoryImpl::new((*db).clone());
            let student_repo = StudentRepositoryImpl::new((*db).clone());
            let student_repo2 = StudentRepositoryImpl::new((*db).clone());
            let attendance_repo = AttendanceRepositoryImpl::new((*db).clone());
            let settings_repo = SettingsRepositoryImpl::new((*db).clone());
            let _user_repo = Arc::new(UserRepositoryImpl::new(app_data_dir_for_user));

            // Create additional repos for GoogleHandler's separate service instances
            let class_repo_for_google = ClassRepositoryImpl::new((*db).clone());
            let attendance_repo_for_google = AttendanceRepositoryImpl::new((*db).clone());
            let student_repo_for_google = StudentRepositoryImpl::new((*db).clone());

            // Initialize domain services
            let class_service = ClassServiceImpl::new(class_repo);
            let student_service = StudentServiceImpl::new(student_repo);
            let attendance_service = AttendanceServiceImpl::new(attendance_repo, student_repo2);

            // Separate service instances for GoogleHandler
            let class_service_for_google = ClassServiceImpl::new(class_repo_for_google);
            let attendance_service_for_google = AttendanceServiceImpl::new(attendance_repo_for_google, student_repo_for_google);

            // Initialize Google Sync
            let google_sync = Arc::new(Mutex::new(GoogleSync::new()));
            
            // Load saved credentials and token
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let settings_repo_clone = settings_repo.clone();
            runtime.block_on(async {
                if let Ok(Some(creds_json)) = settings_repo_clone.get("google_credentials").await {
                    if let Ok(credentials) = serde_json::from_str::<crate::infrastructure::external::GoogleCredentials>(&creds_json) {
                        google_sync.lock().unwrap().set_credentials(credentials);
                    }
                }
                
                if let Ok(Some(token_json)) = settings_repo_clone.get("google_token").await {
                    if let Ok(token) = serde_json::from_str::<crate::infrastructure::external::TokenData>(&token_json) {
                        google_sync.lock().unwrap().set_token(token);
                    }
                }
            });

            /*
            // Initialize auth service
            let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "default_jwt_secret_change_in_production".to_string());
            let auth_service = domain::services::AuthService::new(
                Box::new((*user_repo).clone()),
                jwt_secret
            );
            let auth_handler = Arc::new(
                Mutex::new(AuthHandler::new(auth_service))
            );
            */

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
                GoogleHandler::new(google_sync.clone(), settings_repo, attendance_service_for_google, class_service_for_google)
            );

            // Create app state
            let state = AppState {
                class_handler,
                student_handler,
                attendance_handler,
                google_handler,
                // auth_handler,
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
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
