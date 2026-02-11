// Student Handler
// Application-level handler for student operations

use crate::domain::services::StudentService;
use crate::domain::entities::student::Student;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateStudentInput {
    pub student_id: String,
    pub first_name: String,
    pub last_name: String,
    pub class_id: Option<i64>,
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

    pub fn from_domain_result(result: Result<T, crate::domain::errors::DomainError>) -> ApiResponse<T> {
        match result {
            Ok(data) => ApiResponse::success(data),
            Err(e) => ApiResponse::error(e.to_string()),
        }
    }
}

pub struct StudentHandler<S: StudentService> {
    service: S,
}

impl<S: StudentService> StudentHandler<S> {
    pub fn new(service: S) -> Self {
        StudentHandler { service }
    }

    pub async fn create_student(&self, input: CreateStudentInput) -> ApiResponse<i64> {
        match self
            .service
            .create_student(input.student_id, input.first_name, input.last_name, input.class_id)
            .await
        {
            Ok(student) => ApiResponse::success_with_id(student.id),
            Err(e) => ApiResponse::error(e.to_string()),
        }
    }

    pub async fn get_all_students(&self) -> ApiResponse<Vec<Student>> {
        ApiResponse::from_domain_result(self.service.get_all_students().await)
    }

    pub async fn get_students_by_class(&self, class_id: i64) -> ApiResponse<Vec<Student>> {
        ApiResponse::from_domain_result(self.service.get_students_by_class(class_id).await)
    }

    pub async fn delete_student(&self, id: i64) -> ApiResponse<()> {
        ApiResponse::from_domain_result(self.service.delete_student(id).await)
    }
}