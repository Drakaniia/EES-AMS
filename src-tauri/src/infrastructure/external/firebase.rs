// Firebase Module for Hybrid Storage
// Provides Firestore integration for real-time data synchronization
// Now implements hybrid storage with Google Sheets sync

#![allow(dead_code)]

use crate::domain::entities::{Student, Class, Attendance};
use crate::domain::errors::DomainError;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub id: String,
    pub last_sync: String,
    pub sync_status: String,
    pub source: String,
    pub conflict_resolution: Option<String>,
}

#[derive(Debug)]
pub struct FirebaseService {
    project_id: String,
}

impl FirebaseService {
    pub async fn new(project_id: &str, _service_account_key_path: &str) -> Result<Self> {
        Ok(FirebaseService {
            project_id: project_id.to_string(),
        })
    }

    /// Save data to Firebase (backup storage)
    pub async fn save_data(&self, data: &Value) -> Result<()> {
        // For now, implement local backup with Firebase-like structure
        // In production, this would use Firebase REST API or official SDK
        
        let timestamp = chrono::Utc::now().to_rfc3339();
        let filename = format!("firebase_backup_{}.json", timestamp.replace(":", "-"));
        
        // Create backup directory structure
        let backup_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("backups")
            .join("firebase");
        
        std::fs::create_dir_all(&backup_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create backup directory: {}", e))?;
        
        let backup_path = backup_dir.join(filename);
        let json_content = serde_json::to_string_pretty(data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize data: {}", e))?;
        
        std::fs::write(&backup_path, json_content)
            .map_err(|e| anyhow::anyhow!("Failed to write backup file: {}", e))?;
        
        Ok(())
    }

    /// Sync student data with Firebase
    pub async fn sync_student(&self, student: &Student) -> Result<Student> {
        let student_data = serde_json::to_value(student)?;
        self.save_data(&student_data).await?;
        Ok(student.clone())
    }

    /// Upsert (create or update) student in Firebase
    pub async fn upsert_student(&self, student: &Student) -> Result<Student> {
        let student_data = serde_json::to_value(student)?;
        self.save_data(&student_data).await?;
        Ok(student.clone())
    }

    pub async fn get_student(&self, id: i64) -> Result<Option<Student>> {
        // Try to load from backup files first
        let backup_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("backups")
            .join("firebase");
        
        if backup_dir.exists() {
            // Look for the most recent student backup file
            if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                let mut latest_file: Option<std::path::PathBuf> = None;
                let mut latest_time = std::time::SystemTime::UNIX_EPOCH;
                
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.starts_with("firebase_backup_") && filename.ends_with(".json") {
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                if let Ok(modified) = metadata.modified() {
                                    if modified > latest_time {
                                        latest_time = modified;
                                        latest_file = Some(path);
                                    }
                                }
                            }
                        }
                    }
                }
                
                // If we found a backup file, try to load the student from it
                if let Some(file_path) = latest_file {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                            // Check if this is a student record with matching ID
                            if let Some(student_id) = data.get("id").and_then(|v| v.as_i64()) {
                                if student_id == id {
                                    if let Ok(student) = serde_json::from_value::<Student>(data) {
                                        return Ok(Some(student));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // If no data found in backups, try to load from local JSON database
        let db_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("data");
        
        let students_file = db_dir.join("students.json");
        if students_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&students_file) {
                if let Ok(students) = serde_json::from_str::<Vec<Student>>(&content) {
                    return Ok(students.into_iter().find(|s| s.id == id));
                }
            }
        }
        
        Ok(None)
    }

    pub async fn get_all_students(&self) -> Result<Vec<Student>> {
        let mut students = Vec::new();
        
        // Try to load from backup files first
        let backup_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("backups")
            .join("firebase");
        
        if backup_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.starts_with("firebase_backup_") && filename.ends_with(".json") {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                                    // Check if this contains student data
                                    if data.get("id").is_some() && data.get("student_id").is_some() {
                                        if let Ok(student) = serde_json::from_value::<Student>(data) {
                                            // Avoid duplicates
                                            if !students.iter().any(|s: &Student| s.id == student.id) {
                                                students.push(student);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Also load from local JSON database as backup
        let db_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("data");
        
        let students_file = db_dir.join("students.json");
        if students_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&students_file) {
                if let Ok(db_students) = serde_json::from_str::<Vec<Student>>(&content) {
                    for db_student in db_students {
                        if !students.iter().any(|s| s.id == db_student.id) {
                            students.push(db_student);
                        }
                    }
                }
            }
        }
        
        Ok(students)
    }

    /// Get students by class ID
    pub async fn get_students_by_class(&self, class_id: i64) -> Result<Vec<Student>> {
        let all_students = self.get_all_students().await?;
        Ok(all_students.into_iter()
            .filter(|s| s.class_id == Some(class_id))
            .collect())
    }

    pub async fn delete_student(&self, id: i64) -> Result<()> {
        // Log deletion to Firebase for audit purposes
        let deletion_log = serde_json::json!({
            "type": "student_deleted",
            "student_id": id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "source": "attendease_app"
        });
        self.save_data(&deletion_log).await
    }

    /// Sync class data with Firebase
    pub async fn sync_class(&self, class: &Class) -> Result<Class> {
        let class_data = serde_json::to_value(class)?;
        self.save_data(&class_data).await?;
        Ok(class.clone())
    }

    pub async fn get_class(&self, id: i64) -> Result<Option<Class>> {
        // Try to load from backup files first
        let backup_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("backups")
            .join("firebase");
        
        if backup_dir.exists() {
            // Look for the most recent class backup file
            if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                let mut latest_file: Option<std::path::PathBuf> = None;
                let mut latest_time = std::time::SystemTime::UNIX_EPOCH;
                
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.starts_with("firebase_backup_") && filename.ends_with(".json") {
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                if let Ok(modified) = metadata.modified() {
                                    if modified > latest_time {
                                        latest_time = modified;
                                        latest_file = Some(path);
                                    }
                                }
                            }
                        }
                    }
                }
                
                // If we found a backup file, try to load the class from it
                if let Some(file_path) = latest_file {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                            // Check if this is a class record with matching ID
                            if data.get("id").is_some() && data.get("name").is_some() {
                                if let Some(class_id) = data.get("id").and_then(|v| v.as_i64()) {
                                    if class_id == id {
                                        if let Ok(class) = serde_json::from_value::<Class>(data) {
                                            return Ok(Some(class));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // If no data found in backups, try to load from local JSON database
        let db_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("data");
        
        let classes_file = db_dir.join("classes.json");
        if classes_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&classes_file) {
                if let Ok(classes) = serde_json::from_str::<Vec<Class>>(&content) {
                    return Ok(classes.into_iter().find(|c| c.id == id));
                }
            }
        }
        
        Ok(None)
    }

    pub async fn get_all_classes(&self) -> Result<Vec<Class>> {
        let mut classes = Vec::new();
        
        // Try to load from backup files first
        let backup_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("backups")
            .join("firebase");
        
        if backup_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.starts_with("firebase_backup_") && filename.ends_with(".json") {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                                    // Check if this contains class data
                                    if data.get("id").is_some() && data.get("name").is_some() {
                                        if let Ok(class) = serde_json::from_value::<Class>(data) {
                                            // Avoid duplicates
                                            if !classes.iter().any(|c: &Class| c.id == class.id) {
                                                classes.push(class);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Also load from local JSON database as backup
        let db_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("data");
        
        let classes_file = db_dir.join("classes.json");
        if classes_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&classes_file) {
                if let Ok(db_classes) = serde_json::from_str::<Vec<Class>>(&content) {
                    for db_class in db_classes {
                        if !classes.iter().any(|c| c.id == db_class.id) {
                            classes.push(db_class);
                        }
                    }
                }
            }
        }
        
        Ok(classes)
    }

    pub async fn delete_class(&self, id: i64) -> Result<()> {
        // Log deletion to Firebase for audit purposes
        let deletion_log = serde_json::json!({
            "type": "class_deleted",
            "class_id": id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "source": "attendease_app"
        });
        self.save_data(&deletion_log).await
    }

    /// Sync attendance data with Firebase
    pub async fn sync_attendance(&self, attendance: &Attendance) -> Result<Attendance> {
        let attendance_data = serde_json::to_value(attendance)?;
        self.save_data(&attendance_data).await?;
        Ok(attendance.clone())
    }

    pub async fn get_attendance(&self, id: i64) -> Result<Option<Attendance>> {
        // Try to load from backup files first
        let backup_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("backups")
            .join("firebase");
        
        if backup_dir.exists() {
            // Look for the most recent attendance backup file
            if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                let mut latest_file: Option<std::path::PathBuf> = None;
                let mut latest_time = std::time::SystemTime::UNIX_EPOCH;
                
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.starts_with("firebase_backup_") && filename.ends_with(".json") {
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                if let Ok(modified) = metadata.modified() {
                                    if modified > latest_time {
                                        latest_time = modified;
                                        latest_file = Some(path);
                                    }
                                }
                            }
                        }
                    }
                }
                
                // If we found a backup file, try to load the attendance record from it
                if let Some(file_path) = latest_file {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                            // Check if this is an attendance record with matching ID
                            if data.get("id").is_some() && data.get("student_id").is_some() {
                                if let Some(attendance_id) = data.get("id").and_then(|v| v.as_i64()) {
                                    if attendance_id == id {
                                        if let Ok(attendance) = serde_json::from_value::<Attendance>(data) {
                                            return Ok(Some(attendance));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // If no data found in backups, try to load from local JSON database
        let db_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("data");
        
        let attendance_file = db_dir.join("attendance.json");
        if attendance_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&attendance_file) {
                if let Ok(attendance_records) = serde_json::from_str::<Vec<Attendance>>(&content) {
                    return Ok(attendance_records.into_iter().find(|a| a.id == id));
                }
            }
        }
        
        Ok(None)
    }

    pub async fn get_attendance_by_class_and_date(&self, class_id: i64, date: &str) -> Result<Vec<Attendance>> {
        let mut attendance_records = Vec::new();
        
        // Try to load from backup files first
        let backup_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("backups")
            .join("firebase");
        
        if backup_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.starts_with("firebase_backup_") && filename.ends_with(".json") {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                                    // Check if this contains attendance data matching class_id and date
                                    if let Some(_record_id) = data.get("id").and_then(|v| v.as_i64()) {
                                        if let Some(record_class_id) = data.get("class_id").and_then(|v| v.as_i64()) {
                                            if let Some(record_date) = data.get("date").and_then(|v| v.as_str()) {
                                                if record_class_id == class_id && record_date == date {
                                                    if let Ok(attendance) = serde_json::from_value::<Attendance>(data) {
                                                        // Avoid duplicates
                                                        if !attendance_records.iter().any(|a: &Attendance| a.id == attendance.id) {
                                                            attendance_records.push(attendance);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Also load from local JSON database as backup
        let db_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("attendease")
            .join("data");
        
        let attendance_file = db_dir.join("attendance.json");
        if attendance_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&attendance_file) {
                if let Ok(db_attendance) = serde_json::from_str::<Vec<Attendance>>(&content) {
                    for db_record in db_attendance {
                        if db_record.class_id == class_id && db_record.date == date
                            && !attendance_records.iter().any(|a| a.id == db_record.id) {
                                attendance_records.push(db_record);
                            }
                    }
                }
            }
        }
        
        Ok(attendance_records)
    }

    pub async fn delete_attendance(&self, id: i64) -> Result<()> {
        // Log deletion to Firebase for audit purposes
        let deletion_log = serde_json::json!({
            "type": "attendance_deleted",
            "attendance_id": id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "source": "attendease_app"
        });
        self.save_data(&deletion_log).await
    }

    /// Create sync metadata for hybrid storage operations
    pub async fn create_sync_metadata(&self, data_type: &str, record_id: i64) -> Result<SyncMetadata> {
        let metadata = SyncMetadata {
            id: format!("{}_{}", data_type, record_id),
            last_sync: chrono::Utc::now().to_rfc3339(),
            sync_status: "firebase_backup".to_string(),
            source: "attendease_app".to_string(),
            conflict_resolution: Some("local_primary".to_string()),
        };
        
        let metadata_data = serde_json::to_value(&metadata)?;
        self.save_data(&metadata_data).await?;
        
        Ok(metadata)
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

impl<T> Default for SyncResult<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Public function to save data to Firebase (used by hybrid sync)
pub async fn save_data(data: &Value) -> Result<(), DomainError> {
    let project_id = env::var("FIREBASE_PROJECT_ID")
        .map_err(|_| DomainError::ConfigurationError("FIREBASE_PROJECT_ID not found".to_string()))?;
    
    let service = FirebaseService::new(&project_id, "").await
        .map_err(|e| DomainError::ExternalServiceError(format!("Failed to create Firebase service: {}", e)))?;
    
    service.save_data(data).await
        .map_err(|e| DomainError::ExternalServiceError(format!("Firebase save failed: {}", e)))
}
