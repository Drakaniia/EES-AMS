// Firebase Module for Hybrid Storage
// Provides Firestore integration for real-time data synchronization

use crate::domain::entities::{Student, Class, Attendance};
use anyhow::Result;
use firestore_db_and_auth::{_credentials::Credentials, sessions::ServiceSession};
use firestore::{FirestoreDb, FirestoreError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub id: String,
    pub last_sync: String,
    pub sync_status: String,
    pub source: String, // "local" | "firebase" | "google_drive"
    pub conflict_resolution: Option<String>,
}

#[derive(Debug)]
pub struct FirebaseService {
    db: FirestoreDb,
    project_id: String,
}

impl FirebaseService {
    pub async fn new(project_id: &str, service_account_key_path: &str) -> Result<Self> {
        // Create credentials from service account file
        let credentials = Credentials::from_file(service_account_key_path).await?;
        
        // Initialize Firestore database
        let db = FirestoreDb::new(project_id).await?;
        
        Ok(FirebaseService {
            db,
            project_id: project_id.to_string(),
        })
    }

    // Student operations
    pub async fn sync_student(&self, student: &Student) -> Result<Student> {
        let student_data = serde_json::to_value(student)?;
        
        let result: Student = self.db.fluent()
            .insert()
            .into("students")
            .document_id(&student.id.to_string())
            .object(&student)
            .execute()
            .await?;
            
        Ok(result)
    }

    pub async fn get_student(&self, student_id: i64) -> Result<Option<Student>> {
        let result: Option<Student> = self.db.fluent()
            .select()
            .by_id("students")
            .obj()
            .one(&student_id.to_string())
            .await?;
            
        Ok(result)
    }

    pub async fn get_students_by_class(&self, class_id: i64) -> Result<Vec<Student>> {
        let students: Vec<Student> = self.db.fluent()
            .select()
            .from("students")
            .where_eq("class_id", class_id)
            .obj()
            .query()
            .await?;
            
        Ok(students)
    }

    pub async fn upsert_student(&self, student: &Student) -> Result<Student> {
        let result: Student = self.db.fluent()
            .update()
            .in_col("students")
            .document_id(&student.id.to_string())
            .object(student)
            .execute()
            .await?;
            
        Ok(result)
    }

    pub async fn delete_student(&self, student_id: i64) -> Result<()> {
        self.db.fluent()
            .delete()
            .from("students")
            .document_id(&student_id.to_string())
            .execute()
            .await?;
            
        Ok(())
    }

    // Batch operations for efficient syncing
    pub async fn batch_sync_students(&self, students: &[Student]) -> Result<Vec<Student>> {
        let mut results = Vec::new();
        
        for student in students {
            match self.upsert_student(student).await {
                Ok(synced_student) => results.push(synced_student),
                Err(e) => eprintln!("Failed to sync student {}: {}", student.id, e),
            }
        }
        
        Ok(results)
    }

    // Class operations
    pub async fn sync_class(&self, class: &Class) -> Result<Class> {
        let result: Class = self.db.fluent()
            .insert()
            .into("classes")
            .document_id(&class.id.to_string())
            .object(class)
            .execute()
            .await?;
            
        Ok(result)
    }

    pub async fn get_classes(&self) -> Result<Vec<Class>> {
        let classes: Vec<Class> = self.db.fluent()
            .select()
            .from("classes")
            .obj()
            .query()
            .await?;
            
        Ok(classes)
    }

    // Attendance operations
    pub async fn sync_attendance(&self, attendance: &Attendance) -> Result<Attendance> {
        let result: Attendance = self.db.fluent()
            .insert()
            .into("attendance")
            .document_id(&attendance.id.to_string())
            .object(attendance)
            .execute()
            .await?;
            
        Ok(result)
    }

    pub async fn get_attendance_by_date(&self, date: &str) -> Result<Vec<Attendance>> {
        let attendance: Vec<Attendance> = self.db.fluent()
            .select()
            .from("attendance")
            .where_eq("date", date)
            .obj()
            .query()
            .await?;
            
        Ok(attendance)
    }

    // Sync metadata operations
    pub async fn get_sync_metadata(&self, id: &str) -> Result<Option<SyncMetadata>> {
        let result: Option<SyncMetadata> = self.db.fluent()
            .select()
            .by_id("sync_metadata")
            .obj()
            .one(id)
            .await?;
            
        Ok(result)
    }

    pub async fn update_sync_metadata(&self, metadata: &SyncMetadata) -> Result<SyncMetadata> {
        let result: SyncMetadata = self.db.fluent()
            .update()
            .in_col("sync_metadata")
            .document_id(&metadata.id)
            .object(metadata)
            .execute()
            .await?;
            
        Ok(result)
    }

    // Conflict resolution
    pub async fn resolve_student_conflict(&self, local: &Student, remote: &Student) -> Result<Student> {
        // Simple strategy: use the most recently updated version
        let local_updated = chrono::DateTime::parse_from_rfc3339(&local.updated_at)?;
        let remote_updated = chrono::DateTime::parse_from_rfc3339(&remote.updated_at)?;
        
        let winner = if local_updated > remote_updated {
            local
        } else {
            remote
        };
        
        self.upsert_student(winner).await
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResult<T> {
    pub synced_items: Vec<T>,
    pub failed_items: Vec<String>,
    pub conflicts: Vec<ConflictInfo<T>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConflictInfo<T> {
    pub local_version: T,
    pub remote_version: T,
    pub conflict_type: String,
    pub resolution: Option<String>,
}

impl<T> SyncResult<T> {
    pub fn new() -> Self {
        SyncResult {
            synced_items: Vec::new(),
            failed_items: Vec::new(),
            conflicts: Vec::new(),
        }
    }
}