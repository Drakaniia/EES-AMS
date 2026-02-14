// Tauri Commands
// IPC bridge between frontend and backend

use tauri::State;
use std::sync::Arc;

// AppState for dependency injection
pub struct AppState {
    pub class_handler: Arc<crate::application::handlers::ClassHandler<
        crate::domain::services::ClassServiceImpl<
            crate::infrastructure::database::ClassRepositoryImpl
        >
    >>,
    pub student_handler: Arc<crate::application::handlers::StudentHandler<
        crate::domain::services::StudentServiceImpl<
            crate::infrastructure::database::StudentRepositoryImpl
        >
    >>,
    pub attendance_handler: Arc<crate::application::handlers::AttendanceHandler<
        crate::domain::services::AttendanceServiceImpl<
            crate::infrastructure::database::AttendanceRepositoryImpl,
            crate::infrastructure::database::StudentRepositoryImpl,
        >
    >>,
    pub google_handler: Arc<crate::application::handlers::GoogleHandler<
        crate::infrastructure::database::SettingsRepositoryImpl,
        crate::domain::services::AttendanceServiceImpl<
            crate::infrastructure::database::AttendanceRepositoryImpl,
            crate::infrastructure::database::StudentRepositoryImpl,
        >,
        crate::domain::services::ClassServiceImpl<
            crate::infrastructure::database::ClassRepositoryImpl
        >
    >>,
}

// ========================================
// Class Commands
// ========================================

#[tauri::command]
pub async fn class_create(
    input: crate::application::handlers::class_handler::CreateClassInput,
    state: State<'_, AppState>,
) -> crate::application::handlers::class_handler::ApiResponse<i64> {
    state.class_handler.create_class(input).await
}

#[tauri::command]
pub async fn class_get_all(
    state: State<'_, AppState>,
) -> crate::application::handlers::class_handler::ApiResponse<crate::domain::entities::class::Class> {
    state.class_handler.get_all_classes().await
}

#[tauri::command]
pub async fn class_delete(
    id: i64,
    state: State<'_, AppState>,
) -> crate::application::handlers::class_handler::ApiResponse<()> {
    state.class_handler.delete_class(id).await
}

// ========================================
// Student Commands
// ========================================

#[tauri::command]
pub async fn student_create(
    input: crate::application::handlers::student_handler::CreateStudentInput,
    state: State<'_, AppState>,
) -> crate::application::handlers::student_handler::ApiResponse<i64> {
    state.student_handler.create_student(input).await
}

#[tauri::command]
pub async fn student_get_all(
    state: State<'_, AppState>,
) -> crate::application::handlers::student_handler::ApiResponse<Vec<crate::domain::entities::student::Student>> {
    state.student_handler.get_all_students().await
}

#[tauri::command]
pub async fn student_get_by_class(
    class_id: i64,
    state: State<'_, AppState>,
) -> crate::application::handlers::student_handler::ApiResponse<Vec<crate::domain::entities::student::Student>> {
    state.student_handler.get_students_by_class(class_id).await
}

#[tauri::command]
pub async fn student_delete(
    id: i64,
    state: State<'_, AppState>,
) -> crate::application::handlers::student_handler::ApiResponse<()> {
    state.student_handler.delete_student(id).await
}

#[tauri::command]
pub async fn student_import_from_excel(
    file_path: String,
    class_id: Option<i64>,
    state: State<'_, AppState>,
) -> crate::application::handlers::student_handler::ImportResult {
    use crate::infrastructure::importer::StudentImporter;
    
    let importer = StudentImporter::new();
    
    match importer.import_from_excel(&file_path, class_id) {
        Ok(result) => {
            // After successful import, create students in the database
            let mut created_count = 0;
            let mut db_errors = Vec::new();
            
            for student_data in &result.imported_students {
                let input = crate::application::handlers::student_handler::CreateStudentFromSF1Input {
                    lrn: student_data.lrn.clone(),
                    last_name: student_data.last_name.clone(),
                    first_name: student_data.first_name.clone(),
                    middle_name: student_data.middle_name.clone(),
                    gender: student_data.gender.clone(),
                    birthday: student_data.birthday.clone(),
                    age: student_data.age,
                    mother_name: student_data.mother_name.clone(),
                    father_name: student_data.father_name.clone(),
                    guardian_name: student_data.guardian_name.clone(),
                    address: student_data.address.clone(),
                    class_id: student_data.class_id,
                };
                
                match state.student_handler.create_student_from_sf1(input).await {
                    Ok(_) => created_count += 1,
                    Err(e) => db_errors.push(format!("Failed to create student: {}", e.error.unwrap_or_default())),
                }
            }
            
            crate::application::handlers::student_handler::ImportResult {
                success_count: created_count,
                error_count: result.error_count + db_errors.len(),
                errors: [result.errors, db_errors].concat(),
                imported_students: Vec::new(),
            }
        }
        Err(e) => crate::application::handlers::student_handler::ImportResult {
            success_count: 0,
            error_count: 1,
            errors: vec![format!("Import failed: {}", e)],
            imported_students: Vec::new(),
        }
    }
}

// ========================================
// Attendance Commands
// ========================================

#[tauri::command]
pub async fn attendance_record(
    input: crate::application::handlers::attendance_handler::RecordAttendanceInput,
    state: State<'_, AppState>,
) -> crate::application::handlers::attendance_handler::ApiResponse<i64> {
    state.attendance_handler.record_attendance(input).await
}

#[tauri::command]
pub async fn attendance_get_by_class_and_date(
    class_id: i64,
    date: String,
    state: State<'_, AppState>,
) -> crate::application::handlers::attendance_handler::ApiResponse<crate::domain::entities::attendance::Attendance> {
    state.attendance_handler.get_by_class_and_date(class_id, date).await
}

#[tauri::command]
pub async fn attendance_get_unsynced(
    state: State<'_, AppState>,
) -> crate::application::handlers::attendance_handler::ApiResponse<crate::domain::entities::attendance::Attendance> {
    state.attendance_handler.get_unsynced().await
}

#[tauri::command]
pub async fn attendance_get_stats(
    class_id: i64,
    state: State<'_, AppState>,
) -> crate::application::handlers::attendance_handler::ApiResponse<crate::domain::entities::attendance::AttendanceStats> {
    state.attendance_handler.get_today_stats(class_id).await
}

// ========================================
// Google Sync Commands
// ========================================

#[tauri::command]
pub async fn google_save_credentials(
    credentials: crate::infrastructure::external::GoogleCredentials,
    state: State<'_, AppState>,
) -> crate::application::handlers::google_handler::ApiResponse<()> {
    state.google_handler.save_credentials(credentials).await
}

#[tauri::command]
pub async fn google_is_authenticated(
    state: State<'_, AppState>,
) -> crate::application::handlers::google_handler::ApiResponse<bool> {
    state.google_handler.is_authenticated().await
}

#[tauri::command]
pub async fn google_start_auth(
    state: State<'_, AppState>,
) -> crate::application::handlers::google_handler::ApiResponse<String> {
    state.google_handler.start_auth().await
}

#[tauri::command]
pub async fn google_handle_callback(
    code: String,
    state: State<'_, AppState>,
) -> crate::application::handlers::google_handler::ApiResponse<bool> {
    state.google_handler.handle_callback(code).await
}

#[tauri::command]
pub async fn google_logout(
    state: State<'_, AppState>,
) -> crate::application::handlers::google_handler::ApiResponse<()> {
    state.google_handler.logout().await
}

#[tauri::command]
pub async fn google_sync(
    state: State<'_, AppState>,
) -> crate::application::handlers::google_handler::ApiResponse<bool> {
    state.google_handler.sync().await
}

#[tauri::command]
pub async fn google_get_sync_status(
    state: State<'_, AppState>,
) -> crate::application::handlers::google_handler::ApiResponse<crate::application::handlers::google_handler::SyncStatus> {
    state.google_handler.get_sync_status().await
}

// ========================================
// File System Commands
// ========================================

#[tauri::command]
pub async fn fs_write_file(path: String, contents: Vec<u8>) -> Result<(), String> {
    use std::io::Write;
    use std::fs::File;
    
    match std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()) {
        Ok(_) => {},
        Err(e) => return Err(format!("Failed to create directory: {}", e)),
    }
    
    match File::create(path).and_then(|mut f| f.write_all(&contents)) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to write file: {}", e)),
    }
}

#[tauri::command]
pub async fn fs_remove_file(path: String) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to remove file: {}", e)),
    }
}

// ========================================
// Update Commands
// ========================================

pub use crate::application::handlers::update_handler::UpdateInfo;
pub use crate::application::handlers::update_handler::UpdateStatus;