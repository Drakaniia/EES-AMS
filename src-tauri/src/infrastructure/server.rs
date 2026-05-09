/// HTTP API server
use crate::domain::{error::Result, models::*};
use crate::infrastructure::database::{
    DbPool, EventRepository, SettingsRepository, StudentRepository,
};
use axum::{
    extract::{Path, State},
    http::{header, Method, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
}

impl AppState {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    #[inline]
    fn student_repo(&self) -> StudentRepository {
        StudentRepository::new(self.pool.clone())
    }

    #[inline]
    fn event_repo(&self) -> EventRepository {
        EventRepository::new(self.pool.clone())
    }

    #[inline]
    fn settings_repo(&self) -> SettingsRepository {
        SettingsRepository::new(self.pool.clone())
    }
}

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        // Students
        .route("/api/students", get(list_students).post(create_student))
        .route(
            "/api/students/:id",
            get(get_student).put(update_student).delete(delete_student),
        )
        .route("/api/students/card/:serial", get(find_student_by_card))
        // Events
        .route("/api/events", get(list_events).post(create_event))
        .route("/api/events/:id", delete(delete_event))
        .route("/api/events/student/:student_id", get(list_student_events))
        .route(
            "/api/events/student/:student_id/last",
            get(get_last_student_event),
        )
        // Settings
        .route("/api/settings", get(get_settings).put(update_settings))
        // Export/Import
        .route("/api/export", get(export_data))
        .route("/api/import", post(import_data))
        .route("/api/wipe", post(wipe_data))
        // Health check
        .route("/api/health", get(health_check))
        .layer(cors)
        .with_state(Arc::new(state))
}

/// Start the HTTP server
pub async fn start_server(state: AppState, port: u16) -> anyhow::Result<()> {
    let app = create_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    log::info!("starting HTTP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// ============================================================================
// Student Handlers
// ============================================================================

async fn list_students(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Student>>> {
    let students = state.student_repo().list()?;
    Ok(Json(students))
}

async fn get_student(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Student>> {
    let id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| {
        crate::domain::error::AppError::InvalidInput(format!("invalid student ID: {}", e))
    })?);
    let student = state.student_repo().get(id)?;
    Ok(Json(student))
}

async fn find_student_by_card(
    State(state): State<Arc<AppState>>,
    Path(serial): Path<String>,
) -> Result<impl IntoResponse> {
    let student = state.student_repo().find_by_card(&serial)?;
    match student {
        Some(s) => Ok((StatusCode::OK, Json(Some(s)))),
        None => Ok((StatusCode::NOT_FOUND, Json(None::<Student>))),
    }
}

async fn create_student(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateStudentRequest>,
) -> Result<Json<Student>> {
    let student = state.student_repo().create(req)?;
    Ok(Json(student))
}

async fn update_student(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateStudentRequest>,
) -> Result<Json<Student>> {
    let id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| {
        crate::domain::error::AppError::InvalidInput(format!("invalid student ID: {}", e))
    })?);
    let student = state.student_repo().update(id, req)?;
    Ok(Json(student))
}

async fn delete_student(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    let id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| {
        crate::domain::error::AppError::InvalidInput(format!("invalid student ID: {}", e))
    })?);
    state.student_repo().delete(id)?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Event Handlers
// ============================================================================

async fn list_events(State(state): State<Arc<AppState>>) -> Result<Json<Vec<AttendanceEvent>>> {
    let events = state.event_repo().list()?;
    Ok(Json(events))
}

async fn list_student_events(
    State(state): State<Arc<AppState>>,
    Path(student_id): Path<String>,
) -> Result<Json<Vec<AttendanceEvent>>> {
    let student_id = StudentId(uuid::Uuid::parse_str(&student_id).map_err(|e| {
        crate::domain::error::AppError::InvalidInput(format!("invalid student ID: {}", e))
    })?);
    let events = state.event_repo().list_for_student(student_id)?;
    Ok(Json(events))
}

async fn get_last_student_event(
    State(state): State<Arc<AppState>>,
    Path(student_id): Path<String>,
) -> Result<impl IntoResponse> {
    let student_id = StudentId(uuid::Uuid::parse_str(&student_id).map_err(|e| {
        crate::domain::error::AppError::InvalidInput(format!("invalid student ID: {}", e))
    })?);
    let event = state.event_repo().last_for_student(student_id)?;
    match event {
        Some(e) => Ok((StatusCode::OK, Json(Some(e)))),
        None => Ok((StatusCode::NOT_FOUND, Json(None::<AttendanceEvent>))),
    }
}

async fn create_event(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateEventRequest>,
) -> Result<Json<AttendanceEvent>> {
    let event = state.event_repo().create(req)?;
    Ok(Json(event))
}

async fn delete_event(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    let id = EventId(uuid::Uuid::parse_str(&id).map_err(|e| {
        crate::domain::error::AppError::InvalidInput(format!("invalid event ID: {}", e))
    })?);
    state.event_repo().delete(id)?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Settings Handlers
// ============================================================================

async fn get_settings(State(state): State<Arc<AppState>>) -> Result<Json<Settings>> {
    let settings = state.settings_repo().get()?;
    Ok(Json(settings))
}

async fn update_settings(
    State(state): State<Arc<AppState>>,
    Json(settings): Json<Settings>,
) -> Result<Json<Settings>> {
    let settings = state.settings_repo().update(settings)?;
    Ok(Json(settings))
}

// ============================================================================
// Data Management Handlers
// ============================================================================

async fn export_data(State(state): State<Arc<AppState>>) -> Result<Json<ExportData>> {
    let students = state.student_repo().list()?;
    let events = state.event_repo().list()?;
    let settings = state.settings_repo().get()?;

    let export = ExportData {
        students,
        events,
        settings,
        exported_at: chrono::Utc::now(),
    };

    Ok(Json(export))
}

async fn import_data(
    State(state): State<Arc<AppState>>,
    Json(data): Json<ExportData>,
) -> Result<StatusCode> {
    // Import students
    for student in data.students {
        let req = CreateStudentRequest {
            name: student.name,
            student_number: student.student_number,
            card_serial: student.card_serial,
        };
        let _ = state.student_repo().create(req);
    }

    // Import events
    for event in data.events {
        let req = CreateEventRequest {
            student_id: event.student_id,
            event_type: event.event_type,
            note: event.note,
        };
        let _ = state.event_repo().create(req);
    }

    // Import settings
    let _ = state.settings_repo().update(data.settings);

    Ok(StatusCode::OK)
}

async fn wipe_data(State(state): State<Arc<AppState>>) -> Result<StatusCode> {
    let conn = state.pool.get()?;
    conn.execute("DELETE FROM events", [])?;
    conn.execute("DELETE FROM students", [])?;
    conn.execute(
        "UPDATE settings SET class_name = 'Horizon Class', day_start = '08:30', day_end = '15:30', late_after = '08:45' WHERE id = 1",
        [],
    )?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Health Check
// ============================================================================

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
