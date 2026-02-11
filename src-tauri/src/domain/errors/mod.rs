// Domain Errors Module
// Centralized error types for the domain layer

use std::fmt;
use serde::{Serialize, Deserialize};
use ts_rs::TS;

/// Domain error types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum DomainError {
    NotFound(String),
    AlreadyExists(String),
    ValidationError(String),
    BusinessRuleViolation(String),
    InfrastructureError(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::NotFound(msg) => write!(f, "Not found: {}", msg),
            DomainError::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            DomainError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            DomainError::BusinessRuleViolation(msg) => write!(f, "Business rule violation: {}", msg),
            DomainError::InfrastructureError(msg) => write!(f, "Infrastructure error: {}", msg),
        }
    }
}

impl std::error::Error for DomainError {}

/// Result type for domain operations
pub type DomainResult<T> = Result<T, DomainError>;