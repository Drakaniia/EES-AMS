// Attendance Handler
// Application-level handler for attendance operations

#![allow(dead_code)]

use crate::domain::services::AttendanceService;
use crate::domain::entities::attendance::{Attendance, AttendanceStatus, AttendanceStats};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RecordAttendanceInput {
    pub student_id: i64,
    pub class_id: i64,
    pub date: String,
    pub status: String,
    pub notes: Option<String>,
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

    pub fn from_domain_result(result: Result<T, crate::domain::errors::DomainError>) -> ApiResponse<T> {
        match result {
            Ok(data) => ApiResponse::success(data),
            Err(e) => ApiResponse::error(e.to_string()),
        }
    }
}

pub struct AttendanceHandler<S: AttendanceService> {
    service: S,
}

impl<S: AttendanceService> AttendanceHandler<S> {
    pub fn new(service: S) -> Self {
        AttendanceHandler { service }
    }

    pub async fn record_attendance(&self, input: RecordAttendanceInput) -> ApiResponse<i64> {
        let status = AttendanceStatus::from_str(&input.status);
        match self
            .service
            .record_attendance(input.student_id, input.class_id, input.date, status, input.notes)
            .await
        {
            Ok(attendance) => ApiResponse::success_with_id(attendance.id),
            Err(e) => ApiResponse::error(e.to_string()),
        }
    }

    pub async fn get_by_class_and_date(&self, class_id: i64, date: String) -> ApiResponse<Vec<Attendance>> {
        ApiResponse::from_domain_result(self.service.get_attendance_by_class_and_date(class_id, &date).await)
    }

    pub async fn get_today_stats(&self, class_id: i64) -> ApiResponse<AttendanceStats> {
        ApiResponse::from_domain_result(self.service.get_today_stats(class_id).await)
    }

    pub async fn get_unsynced(&self) -> ApiResponse<Vec<Attendance>> {
        ApiResponse::from_domain_result(self.service.get_unsynced_records().await)
    }
}