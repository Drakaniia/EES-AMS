/// Error types for the attendance system
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tauri::ipc::InvokeError;
use thiserror::Error;

/// Application error type
#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("connection pool error: {0}")]
    Pool(#[from] r2d2::Error),

    #[error("student not found: {0}")]
    StudentNotFound(String),

    #[error("event not found: {0}")]
    EventNotFound(String),

    #[error("class not found: {0}")]
    ClassNotFound(String),

    #[error("card already registered: {0}")]
    CardAlreadyRegistered(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("internal server error: {0}")]
    #[allow(dead_code)]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::StudentNotFound(msg) | Self::EventNotFound(msg) | Self::ClassNotFound(msg) => {
                (StatusCode::NOT_FOUND, msg)
            }
            Self::CardAlreadyRegistered(msg) | Self::InvalidInput(msg) => {
                (StatusCode::BAD_REQUEST, msg)
            }
            Self::Database(ref e) => {
                log::error!("database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database error occurred".to_string(),
                )
            }
            Self::Pool(ref e) => {
                log::error!("connection pool error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "connection pool error".to_string(),
                )
            }
            Self::Internal(msg) => {
                log::error!("internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, AppError>;

impl From<AppError> for InvokeError {
    fn from(val: AppError) -> Self {
        InvokeError::from(val.to_string())
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Internal(s.to_string())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}
