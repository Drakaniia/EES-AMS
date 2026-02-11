// Repository Trait: AttendanceRepository
// Abstract interface for attendance data access

use async_trait::async_trait;
use crate::domain::entities::attendance::{Attendance, AttendanceStatus, AttendanceStats};
use crate::domain::errors::{DomainError, DomainResult};

#[async_trait]
pub trait AttendanceRepository: Send + Sync {
    /// Record or update attendance for a student
    async fn record(
        &self,
        student_id: i64,
        class_id: i64,
        date: String,
        status: AttendanceStatus,
        notes: Option<String>,
    ) -> DomainResult<i64>;

    /// Get attendance by ID
    async fn get_by_id(&self, id: i64) -> DomainResult<Attendance>;

    /// Get attendance for a class on a specific date
    async fn get_by_class_and_date(&self, class_id: i64, date: &str) -> DomainResult<Vec<Attendance>>;

    /// Get all unsynced attendance records
    async fn get_unsynced(&self) -> DomainResult<Vec<Attendance>>;

    /// Get unsynced records for a specific class
    async fn get_unsynced_by_class(&self, class_id: i64) -> DomainResult<Vec<Attendance>>;

    /// Mark records as synced
    async fn mark_as_synced(&self, record_ids: Vec<i64>) -> DomainResult<()>;

    /// Get attendance statistics for a class on a specific date
    async fn get_stats(&self, class_id: i64, date: &str) -> DomainResult<AttendanceStats>;

    /// Get attendance for a student within a date range
    async fn get_by_student_and_date_range(
        &self,
        student_id: i64,
        start_date: &str,
        end_date: &str,
    ) -> DomainResult<Vec<Attendance>>;

    /// Check if attendance record exists for a student on a specific date
    async fn exists(&self, student_id: i64, class_id: i64, date: &str) -> DomainResult<bool>;
}