// Repository Trait: StudentRepository
// Abstract interface for student data access

use async_trait::async_trait;
use crate::domain::entities::student::Student;
use crate::domain::errors::{DomainError, DomainResult};

#[async_trait]
pub trait StudentRepository: Send + Sync {
    /// Create a new student
    async fn create(
        &self,
        student_id: String,
        first_name: String,
        last_name: String,
        class_id: Option<i64>,
    ) -> DomainResult<i64>;

    /// Create a new student from SF1 data
    async fn create_from_sf1(
        &self,
        student_id: String,
        lrn: Option<String>,
        last_name: String,
        first_name: String,
        middle_name: Option<String>,
        gender: Option<String>,
        birthday: Option<String>,
        age: Option<i32>,
        mother_name: Option<String>,
        father_name: Option<String>,
        guardian_name: Option<String>,
        address: Option<String>,
        class_id: Option<i64>,
    ) -> DomainResult<i64>;

    /// Get a student by ID
    async fn get_by_id(&self, id: i64) -> DomainResult<Student>;

    /// Get all students, sorted by last name then first name
    async fn get_all(&self) -> DomainResult<Vec<Student>>;

    /// Get students by class ID
    async fn get_by_class(&self, class_id: i64) -> DomainResult<Vec<Student>>;

    /// Get a student by their student_id field
    async fn get_by_student_id(&self, student_id: &str) -> DomainResult<Option<Student>>;

    /// Delete a student by ID
    async fn delete(&self, id: i64) -> DomainResult<()>;

    /// Check if a student_id already exists
    async fn student_id_exists(&self, student_id: &str) -> DomainResult<bool>;

    /// Check if an LRN already exists
    async fn lrn_exists(&self, lrn: &str) -> DomainResult<bool>;

    /// Get count of students in a class
    async fn count_by_class(&self, class_id: i64) -> DomainResult<i32>;
}