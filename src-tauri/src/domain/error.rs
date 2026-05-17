/// Error types for the attendance system
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

    #[error("duplicate check-in: {0}")]
    DuplicateCheckIn(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("internal server error: {0}")]
    #[allow(dead_code)]
    Internal(String),
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
