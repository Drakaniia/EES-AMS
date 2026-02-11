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