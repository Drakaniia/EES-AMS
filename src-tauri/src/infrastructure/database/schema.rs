// Database Schema
// Internal database record structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Database counters for generating IDs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counters {
    pub classes: i64,
    pub students: i64,
    pub attendance: i64,
}

impl Default for Counters {
    fn default() -> Self {
        Counters {
            classes: 0,
            students: 0,
            attendance: 0,
        }
    }
}

/// Class record for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassRecord {
    pub id: i64,
    pub name: String,
    pub section: Option<String>,
    pub school_year: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Student record for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentRecord {
    pub id: i64,
    pub student_id: String,
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
    pub created_at: String,
    pub updated_at: String,
}

/// Attendance record for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceRecord {
    pub id: i64,
    pub student_id: i64,
    pub class_id: i64,
    pub date: String,
    pub status: String,
    pub notes: Option<String>,
    pub synced: bool,
    pub created_at: String,
}

/// Complete database schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub classes: Vec<ClassRecord>,
    pub students: Vec<StudentRecord>,
    pub attendance: Vec<AttendanceRecord>,
    pub settings: HashMap<String, String>,
    pub counters: Counters,
}

impl Default for DatabaseSchema {
    fn default() -> Self {
        DatabaseSchema {
            classes: Vec::new(),
            students: Vec::new(),
            attendance: Vec::new(),
            settings: HashMap::new(),
            counters: Counters::default(),
        }
    }
}
