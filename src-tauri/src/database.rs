// Database Manager for Attendance Management System
// Uses JSON file storage for cross-platform compatibility

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use chrono::{DateTime, Utc};

// Database structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub classes: Vec<ClassRecord>,
    pub students: Vec<StudentRecord>,
    pub attendance: Vec<AttendanceRecordDB>,
    pub sync_queue: Vec<SyncQueueItem>,
    pub settings: std::collections::HashMap<String, String>,
    pub counters: Counters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counters {
    pub classes: i64,
    pub students: i64,
    pub attendance: i64,
    pub sync_queue: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassRecord {
    pub id: i64,
    pub name: String,
    pub section: Option<String>,
    pub school_year: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentRecord {
    pub id: i64,
    pub student_id: String,
    pub first_name: String,
    pub last_name: String,
    pub class_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceRecordDB {
    pub id: i64,
    pub student_id: i64,
    pub class_id: i64,
    pub date: String,
    pub status: String,
    pub notes: Option<String>,
    pub synced: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncQueueItem {
    pub id: i64,
    pub table_name: String,
    pub record_id: i64,
    pub action: String,
    pub created_at: String,
    pub synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceStats {
    pub total_students: i32,
    pub present_today: i32,
    pub absent_today: i32,
    pub late_today: i32,
    pub attendance_rate: i32,
}

pub struct Database {
    data: Mutex<DatabaseSchema>,
    db_path: PathBuf,
}

impl Database {
    pub fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        let db_path = app_data_dir.join("attendance-data.json");
        
        let data = if db_path.exists() {
            let content = fs::read_to_string(&db_path)
                .map_err(|e| format!("Failed to read database: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse database: {}", e))?
        } else {
            DatabaseSchema {
                classes: Vec::new(),
                students: Vec::new(),
                attendance: Vec::new(),
                sync_queue: Vec::new(),
                settings: std::collections::HashMap::new(),
                counters: Counters {
                    classes: 0,
                    students: 0,
                    attendance: 0,
                    sync_queue: 0,
                },
            }
        };

        let db = Database {
            data: Mutex::new(data),
            db_path,
        };

        db.save()?;
        Ok(db)
    }

    fn save(&self) -> Result<(), String> {
        let data = self.data.lock().unwrap();
        let json = serde_json::to_string_pretty(&*data)
            .map_err(|e| format!("Failed to serialize database: {}", e))?;
        fs::write(&self.db_path, json)
            .map_err(|e| format!("Failed to write database: {}", e))?;
        Ok(())
    }

    // Class Operations
    pub fn create_class(&self, name: String, section: Option<String>, school_year: Option<String>) -> Result<i64, String> {
        let mut data = self.data.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        data.counters.classes += 1;
        let id = data.counters.classes;

        let class = ClassRecord {
            id,
            name,
            section,
            school_year,
            created_at: now.clone(),
            updated_at: now,
        };

        data.classes.push(class);
        drop(data);
        
        self.add_to_sync_queue("classes", id, "insert")?;
        self.save()?;
        Ok(id)
    }

    pub fn get_all_classes(&self) -> Result<Vec<ClassRecord>, String> {
        let data = self.data.lock().unwrap();
        let mut classes = data.classes.clone();
        classes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(classes)
    }

    pub fn delete_class(&self, id: i64) -> Result<(), String> {
        let mut data = self.data.lock().unwrap();
        data.classes.retain(|c| c.id != id);
        drop(data);
        
        self.add_to_sync_queue("classes", id, "delete")?;
        self.save()?;
        Ok(())
    }

    // Student Operations
    pub fn create_student(&self, student_id: String, first_name: String, last_name: String, class_id: Option<i64>) -> Result<i64, String> {
        let mut data = self.data.lock().unwrap();
        
        // Check for duplicate student_id
        if data.students.iter().any(|s| s.student_id == student_id) {
            return Err("Student ID already exists".to_string());
        }

        let now = Utc::now().to_rfc3339();
        data.counters.students += 1;
        let id = data.counters.students;

        let student = StudentRecord {
            id,
            student_id,
            first_name,
            last_name,
            class_id,
            created_at: now.clone(),
            updated_at: now,
        };

        data.students.push(student);
        drop(data);
        
        self.add_to_sync_queue("students", id, "insert")?;
        self.save()?;
        Ok(id)
    }

    pub fn get_students_by_class(&self, class_id: i64) -> Result<Vec<StudentRecord>, String> {
        let data = self.data.lock().unwrap();
        let mut students: Vec<StudentRecord> = data.students.iter()
            .filter(|s| s.class_id == Some(class_id))
            .cloned()
            .collect();
        students.sort_by(|a, b| {
            a.last_name.cmp(&b.last_name)
                .then_with(|| a.first_name.cmp(&b.first_name))
        });
        Ok(students)
    }

    pub fn get_all_students(&self) -> Result<Vec<StudentRecord>, String> {
        let data = self.data.lock().unwrap();
        let mut students = data.students.clone();
        students.sort_by(|a, b| {
            a.last_name.cmp(&b.last_name)
                .then_with(|| a.first_name.cmp(&b.first_name))
        });
        Ok(students)
    }

    pub fn delete_student(&self, id: i64) -> Result<(), String> {
        let mut data = self.data.lock().unwrap();
        data.students.retain(|s| s.id != id);
        drop(data);
        
        self.add_to_sync_queue("students", id, "delete")?;
        self.save()?;
        Ok(())
    }

    // Attendance Operations
    pub fn record_attendance(&self, student_id: i64, class_id: i64, date: String, status: String, notes: Option<String>) -> Result<i64, String> {
        let mut data = self.data.lock().unwrap();
        
        // Check for existing record
        if let Some(existing) = data.attendance.iter_mut().find(|a| {
            a.student_id == student_id && a.class_id == class_id && a.date == date
        }) {
            existing.status = status;
            existing.notes = notes;
            existing.synced = 0;
            let id = existing.id;
            drop(data);
            
            self.add_to_sync_queue("attendance", id, "update")?;
            self.save()?;
            return Ok(id);
        }

        let now = Utc::now().to_rfc3339();
        data.counters.attendance += 1;
        let id = data.counters.attendance;

        let record = AttendanceRecordDB {
            id,
            student_id,
            class_id,
            date,
            status,
            notes,
            synced: 0,
            created_at: now,
        };

        data.attendance.push(record);
        drop(data);
        
        self.add_to_sync_queue("attendance", id, "insert")?;
        self.save()?;
        Ok(id)
    }

    pub fn get_attendance_by_class_and_date(&self, class_id: i64, date: String) -> Result<Vec<AttendanceRecordDB>, String> {
        let data = self.data.lock().unwrap();
        let records: Vec<AttendanceRecordDB> = data.attendance.iter()
            .filter(|a| a.class_id == class_id && a.date == date)
            .cloned()
            .collect();
        Ok(records)
    }

    pub fn get_unsynced_records(&self) -> Result<Vec<AttendanceRecordDB>, String> {
        let data = self.data.lock().unwrap();
        let records: Vec<AttendanceRecordDB> = data.attendance.iter()
            .filter(|a| a.synced == 0)
            .cloned()
            .collect();
        Ok(records)
    }

    pub fn mark_as_synced(&self, record_ids: Vec<i64>) -> Result<(), String> {
        if record_ids.is_empty() {
            return Ok(());
        }

        let mut data = self.data.lock().unwrap();
        for record in data.attendance.iter_mut() {
            if record_ids.contains(&record.id) {
                record.synced = 1;
            }
        }
        drop(data);
        
        self.save()?;
        Ok(())
    }

    pub fn get_today_stats(&self, class_id: i64) -> Result<AttendanceStats, String> {
        let data = self.data.lock().unwrap();
        let today = Utc::now().format("%Y-%m-%d").to_string();

        let total_students = data.students.iter()
            .filter(|s| s.class_id == Some(class_id))
            .count() as i32;

        let today_attendance: Vec<&AttendanceRecordDB> = data.attendance.iter()
            .filter(|a| a.class_id == class_id && a.date == today)
            .collect();

        let present_today = today_attendance.iter()
            .filter(|a| a.status == "present")
            .count() as i32;

        let absent_today = today_attendance.iter()
            .filter(|a| a.status == "absent")
            .count() as i32;

        let late_today = today_attendance.iter()
            .filter(|a| a.status == "late")
            .count() as i32;

        let attendance_rate = if total_students > 0 {
            ((present_today + late_today) as f32 / total_students as f32 * 100.0).round() as i32
        } else {
            0
        };

        Ok(AttendanceStats {
            total_students,
            present_today,
            absent_today,
            late_today,
            attendance_rate,
        })
    }

    // Sync Queue Operations
    fn add_to_sync_queue(&self, table_name: &str, record_id: i64, action: &str) -> Result<(), String> {
        let mut data = self.data.lock().unwrap();
        data.counters.sync_queue += 1;
        let id = data.counters.sync_queue;

        let item = SyncQueueItem {
            id,
            table_name: table_name.to_string(),
            record_id,
            action: action.to_string(),
            created_at: Utc::now().to_rfc3339(),
            synced_at: None,
        };

        data.sync_queue.push(item);
        Ok(())
    }

    pub fn get_pending_sync_items(&self) -> Result<Vec<SyncQueueItem>, String> {
        let data = self.data.lock().unwrap();
        let items: Vec<SyncQueueItem> = data.sync_queue.iter()
            .filter(|item| item.synced_at.is_none())
            .cloned()
            .collect();
        Ok(items)
    }

    pub fn mark_sync_items_complete(&self, item_ids: Vec<i64>) -> Result<(), String> {
        if item_ids.is_empty() {
            return Ok(());
        }

        let mut data = self.data.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        for item in data.sync_queue.iter_mut() {
            if item_ids.contains(&item.id) {
                item.synced_at = Some(now.clone());
            }
        }
        drop(data);
        
        self.save()?;
        Ok(())
    }

    // Settings Operations
    pub fn get_setting(&self, key: &str) -> Option<String> {
        let data = self.data.lock().unwrap();
        data.settings.get(key).cloned()
    }

    pub fn set_setting(&self, key: String, value: String) -> Result<(), String> {
        let mut data = self.data.lock().unwrap();
        data.settings.insert(key, value);
        drop(data);
        
        self.save()?;
        Ok(())
    }
}
