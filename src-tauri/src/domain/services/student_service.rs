// Student Service
// Business logic for student operations

use crate::domain::entities::student::Student;
use crate::domain::repositories::StudentRepository;
use crate::domain::errors::{DomainError, DomainResult};
use async_trait::async_trait;

#[async_trait]
pub trait StudentService: Send + Sync {
    async fn create_student(&self, student_id: String, first_name: String, last_name: String, class_id: Option<i64>) -> DomainResult<Student>;
    async fn create_student_from_sf1(
        &self,
        lrn: Option<String>,
        student_id: String,
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
    ) -> DomainResult<Student>;
    async fn get_all_students(&self) -> DomainResult<Vec<Student>>;
    async fn get_student_by_id(&self, id: i64) -> DomainResult<Student>;
    async fn get_students_by_class(&self, class_id: i64) -> DomainResult<Vec<Student>>;
    async fn delete_student(&self, id: i64) -> DomainResult<()>;
    async fn unassign_from_class(&self, id: i64) -> DomainResult<()>;
}

pub struct StudentServiceImpl<R: StudentRepository> {
    student_repo: R,
}

impl<R: StudentRepository> StudentServiceImpl<R> {
    pub fn new(student_repo: R) -> Self {
        StudentServiceImpl { student_repo }
    }
}

#[async_trait]
impl<R: StudentRepository + Send + Sync> StudentService for StudentServiceImpl<R> {
    async fn create_student(&self, student_id: String, first_name: String, last_name: String, class_id: Option<i64>) -> DomainResult<Student> {
        // Validate inputs
        if student_id.trim().is_empty() {
            return Err(DomainError::ValidationError("Student ID cannot be empty".to_string()));
        }
        if first_name.trim().is_empty() {
            return Err(DomainError::ValidationError("First name cannot be empty".to_string()));
        }
        if last_name.trim().is_empty() {
            return Err(DomainError::ValidationError("Last name cannot be empty".to_string()));
        }

        // Check for duplicate student_id
        if self.student_repo.student_id_exists(&student_id).await? {
            return Err(DomainError::AlreadyExists(format!("Student ID '{}' already exists", student_id)));
        }

        // Validate class_id if provided
        if let Some(cid) = class_id {
            if self.student_repo.count_by_class(cid).await? == 0 {
                // Note: In a real implementation, we'd verify the class exists via ClassRepository
                // For now, we'll allow it and handle it at the UI level
            }
        }

        let id = self.student_repo.create(student_id.clone(), first_name.clone(), last_name.clone(), class_id).await?;
        Ok(Student::new(id, student_id, first_name, last_name, class_id))
    }

    async fn get_all_students(&self) -> DomainResult<Vec<Student>> {
        self.student_repo.get_all().await
    }

    async fn get_student_by_id(&self, id: i64) -> DomainResult<Student> {
        self.student_repo.get_by_id(id).await
    }

    async fn get_students_by_class(&self, class_id: i64) -> DomainResult<Vec<Student>> {
        self.student_repo.get_by_class(class_id).await
    }

    async fn delete_student(&self, id: i64) -> DomainResult<()> {
        // Verify student exists
        self.student_repo.get_by_id(id).await?;
        self.student_repo.delete(id).await
    }

    async fn create_student_from_sf1(
        &self,
        lrn: Option<String>,
        student_id: String,
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
    ) -> DomainResult<Student> {
        // Validate required fields
        if last_name.trim().is_empty() {
            return Err(DomainError::ValidationError("Last name cannot be empty".to_string()));
        }
        if first_name.trim().is_empty() {
            return Err(DomainError::ValidationError("First name cannot be empty".to_string()));
        }

        // Check for duplicate student_id if provided
        if !student_id.trim().is_empty() && self.student_repo.student_id_exists(&student_id).await? {
            return Err(DomainError::AlreadyExists(format!("Student ID '{}' already exists", student_id)));
        }

        // Check for duplicate LRN if provided
        if let Some(ref lrn_val) = lrn {
            if self.student_repo.lrn_exists(lrn_val).await? {
                return Err(DomainError::AlreadyExists(format!("LRN '{}' already exists", lrn_val)));
            }
        }

        let id = self.student_repo.create_from_sf1(
            student_id.clone(),
            lrn.clone(),
            last_name.clone(),
            first_name.clone(),
            middle_name.clone(),
            gender.clone(),
            birthday.clone(),
            age,
            mother_name.clone(),
            father_name.clone(),
            guardian_name.clone(),
            address.clone(),
            class_id,
        ).await?;
        
        Ok(Student::new_from_sf1(
            id,
            lrn,
            last_name,
            first_name,
            middle_name,
            gender,
            birthday,
            age,
            mother_name,
            father_name,
            guardian_name,
            address,
            class_id,
        ))
    }

    async fn unassign_from_class(&self, id: i64) -> DomainResult<()> {
        let mut student = self.student_repo.get_by_id(id).await?;
        student.class_id = None;
        // In a real implementation, we'd have an update method
        // For now, this is a placeholder
        Ok(())
    }
}