// Student Repository Implementation
// JSON file-based implementation of StudentRepository trait

use async_trait::async_trait;
use crate::domain::entities::student::Student;
use crate::domain::repositories::StudentRepository;
use crate::domain::errors::{DomainError, DomainResult};
use crate::infrastructure::database::schema::StudentRecord;
use crate::infrastructure::database::JsonDatabase;

pub struct StudentRepositoryImpl {
    db: JsonDatabase,
}

impl StudentRepositoryImpl {
    pub fn new(db: JsonDatabase) -> Self {
        StudentRepositoryImpl { db }
    }

    fn record_to_entity(record: &StudentRecord) -> Student {
        Student {
            id: record.id,
            student_id: record.student_id.clone(),
            first_name: record.first_name.clone(),
            last_name: record.last_name.clone(),
            class_id: record.class_id,
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        }
    }
}

#[async_trait]
impl StudentRepository for StudentRepositoryImpl {
    async fn create(
        &self,
        student_id: String,
        first_name: String,
        last_name: String,
        class_id: Option<i64>,
    ) -> DomainResult<i64> {
        let mut data = self.db.get_data().lock().unwrap();
        
        if data.students.iter().any(|s| s.student_id == student_id) {
            return Err(DomainError::AlreadyExists(format!("Student ID {} already exists", student_id)));
        }

        let now = chrono::Utc::now().to_rfc3339();
        data.counters.students += 1;
        let id = data.counters.students;

        let student = StudentRecord {
            id,
            student_id: student_id.clone(),
            first_name,
            last_name,
            class_id,
            created_at: now.clone(),
            updated_at: now,
        };

        data.students.push(student);
        drop(data);
        
        self.db.save()?;
        Ok(id)
    }

    async fn get_by_id(&self, id: i64) -> DomainResult<Student> {
        let data = self.db.get_data().lock().unwrap();
        data.students.iter()
            .find(|s| s.id == id)
            .map(Self::record_to_entity)
            .ok_or_else(|| DomainError::NotFound(format!("Student with id {} not found", id)))
    }

    async fn get_all(&self) -> DomainResult<Vec<Student>> {
        let data = self.db.get_data().lock().unwrap();
        let mut students: Vec<Student> = data.students.iter()
            .map(Self::record_to_entity)
            .collect();
        students.sort_by(|a, b| {
            a.last_name.cmp(&b.last_name)
                .then_with(|| a.first_name.cmp(&b.first_name))
        });
        Ok(students)
    }

    async fn get_by_class(&self, class_id: i64) -> DomainResult<Vec<Student>> {
        let data = self.db.get_data().lock().unwrap();
        let mut students: Vec<Student> = data.students.iter()
            .filter(|s| s.class_id == Some(class_id))
            .map(Self::record_to_entity)
            .collect();
        students.sort_by(|a, b| {
            a.last_name.cmp(&b.last_name)
                .then_with(|| a.first_name.cmp(&b.first_name))
        });
        Ok(students)
    }

    async fn get_by_student_id(&self, student_id: &str) -> DomainResult<Option<Student>> {
        let data = self.db.get_data().lock().unwrap();
        Ok(data.students.iter()
            .find(|s| s.student_id == student_id)
            .map(Self::record_to_entity))
    }

    async fn delete(&self, id: i64) -> DomainResult<()> {
        let mut data = self.db.get_data().lock().unwrap();
        let original_len = data.students.len();
        data.students.retain(|s| s.id != id);
        
        if data.students.len() == original_len {
            return Err(DomainError::NotFound(format!("Student with id {} not found", id)));
        }
        
        drop(data);
        self.db.save()?;
        Ok(())
    }

    async fn student_id_exists(&self, student_id: &str) -> DomainResult<bool> {
        let data = self.db.get_data().lock().unwrap();
        Ok(data.students.iter().any(|s| s.student_id == student_id))
    }

    async fn count_by_class(&self, class_id: i64) -> DomainResult<i32> {
        let data = self.db.get_data().lock().unwrap();
        Ok(data.students.iter()
            .filter(|s| s.class_id == Some(class_id))
            .count() as i32)
    }
}