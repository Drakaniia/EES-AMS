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

#[derive(Debug, Deserialize)]
pub struct CreateStudentFromSF1Input {
    pub lrn: Option<String>,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: Option<String>,
    pub gender: Option<String>,
    pub birthday: Option<String>,
    pub age: Option<i32>,
    pub mother_name: Option<String>,
    pub father_name: Option<String>,
    pub guardian_name: Option<String>,
    pub address: Option<String>,
    pub class_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub success_count: usize,
    pub error_count: usize,
    pub errors: Vec<String>,
    pub imported_students: Vec<crate::domain::entities::student::Student>,
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

    pub async fn create_student_from_sf1(&self, input: CreateStudentFromSF1Input) -> ApiResponse<i64> {
        // Generate student_id from LRN or create a default one
        let student_id = input.lrn.clone().unwrap_or_else(|| {
            // Generate a temporary ID that will be replaced
            format!("TEMP_{}", chrono::Utc::now().timestamp_nanos())
        });
        
        match self
            .service
            .create_student_from_sf1(
                input.lrn,
                student_id,
                input.last_name,
                input.first_name,
                input.middle_name,
                input.gender,
                input.birthday,
                input.age,
                input.mother_name,
                input.father_name,
                input.guardian_name,
                input.address,
                input.class_id,
            )
            .await
        {
            Ok(student) => ApiResponse::success_with_id(student.id),
            Err(e) => ApiResponse::error(e.to_string()),
        }
    }
}