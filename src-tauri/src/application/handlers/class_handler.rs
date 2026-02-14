// Class Handler
// Application-level handler for class operations

use crate::domain::services::ClassService;
use crate::domain::errors::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateClassInput {
    pub name: String,
    pub section: Option<String>,
    pub school_year: Option<String>,
}

#[derive(Debug, Serialize)]
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
    #[allow(dead_code)]
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            id: None,
            error: None,
        }
    }

    pub fn success_with_id(id: i64) -> Self {
        ApiResponse {
            success: true,
            data: None,
            id: Some(id),
            error: None,
        }
    }

    #[allow(dead_code)]
    pub fn success_empty() -> Self {
        ApiResponse {
            success: true,
            data: None,
            id: None,
            error: None,
        }
    }

    pub fn error(msg: String) -> Self {
        ApiResponse {
            success: false,
            data: None,
            id: None,
            error: Some(msg),
        }
    }

    pub fn from_domain_result(result: Result<T, DomainError>) -> ApiResponse<T> {
        match result {
            Ok(data) => ApiResponse::success(data),
            Err(e) => ApiResponse::error(e.to_string()),
        }
    }
}

pub struct ClassHandler<S: ClassService> {
    service: S,
}

impl<S: ClassService> ClassHandler<S> {
    pub fn new(service: S) -> Self {
        ClassHandler { service }
    }

    pub async fn create_class(&self, input: CreateClassInput) -> ApiResponse<i64> {
        match self.service.create_class(input.name, input.section, input.school_year).await {
            Ok(class) => ApiResponse::success_with_id(class.id),
            Err(e) => ApiResponse::error(e.to_string()),
        }
    }

    pub async fn get_all_classes(&self) -> ApiResponse<Vec<crate::domain::entities::class::Class>> {
        ApiResponse::from_domain_result(self.service.get_all_classes().await)
    }

    pub async fn delete_class(&self, id: i64) -> ApiResponse<()> {
        ApiResponse::from_domain_result(self.service.delete_class(id).await)
    }
}