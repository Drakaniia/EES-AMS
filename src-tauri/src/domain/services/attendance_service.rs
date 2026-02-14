// Attendance Service
// Business logic for attendance operations

use crate::domain::entities::attendance::{Attendance, AttendanceStatus, AttendanceStats};
use crate::domain::repositories::{AttendanceRepository, StudentRepository};
use crate::domain::errors::{DomainError, DomainResult};
use async_trait::async_trait;

#[async_trait]
pub trait AttendanceService: Send + Sync {
    async fn record_attendance(
        &self,
        student_id: i64,
        class_id: i64,
        date: String,
        status: AttendanceStatus,
        notes: Option<String>,
    ) -> DomainResult<Attendance>;
    async fn get_attendance_by_class_and_date(&self, class_id: i64, date: &str) -> DomainResult<Vec<Attendance>>;
    async fn get_today_stats(&self, class_id: i64) -> DomainResult<AttendanceStats>;
    async fn get_unsynced_records(&self) -> DomainResult<Vec<Attendance>>;
    async fn mark_as_synced(&self, record_ids: Vec<i64>) -> DomainResult<()>;
}

pub struct AttendanceServiceImpl<AR: AttendanceRepository, SR: StudentRepository> {
    attendance_repo: AR,
    student_repo: SR,
}

impl<AR: AttendanceRepository, SR: StudentRepository> AttendanceServiceImpl<AR, SR> {
    pub fn new(attendance_repo: AR, student_repo: SR) -> Self {
        AttendanceServiceImpl {
            attendance_repo,
            student_repo,
        }
    }
}

#[async_trait]
impl<AR: AttendanceRepository + Send + Sync, SR: StudentRepository + Send + Sync> AttendanceService
    for AttendanceServiceImpl<AR, SR>
{
    async fn record_attendance(
        &self,
        student_id: i64,
        class_id: i64,
        date: String,
        status: AttendanceStatus,
        notes: Option<String>,
    ) -> DomainResult<Attendance> {
        // Validate student exists
        let student = self.student_repo.get_by_id(student_id).await?;
        
        // Validate student is assigned to the class
        if student.class_id != Some(class_id) {
            return Err(DomainError::BusinessRuleViolation(
                "Student is not assigned to this class".to_string(),
            ));
        }

        // Validate date format (basic check)
        if date.trim().is_empty() {
            return Err(DomainError::ValidationError("Date cannot be empty".to_string()));
        }

        let id = self.attendance_repo
            .record(student_id, class_id, date.clone(), status.clone(), notes.clone())
            .await?;

        Ok(Attendance::new(id, student_id, class_id, date, status, notes))
    }

    async fn get_attendance_by_class_and_date(&self, class_id: i64, date: &str) -> DomainResult<Vec<Attendance>> {
        self.attendance_repo.get_by_class_and_date(class_id, date).await
    }

    async fn get_today_stats(&self, class_id: i64) -> DomainResult<AttendanceStats> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        self.attendance_repo.get_stats(class_id, &today).await
    }

    async fn get_unsynced_records(&self) -> DomainResult<Vec<Attendance>> {
        self.attendance_repo.get_unsynced().await
    }

    async fn mark_as_synced(&self, record_ids: Vec<i64>) -> DomainResult<()> {
        self.attendance_repo.mark_as_synced(record_ids).await
    }
}