// Domain Entity: Student
// Represents a student in the attendance management system

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Represents a student
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct Student {
    pub id: i64,
    pub student_id: String,
    pub first_name: String,
    pub last_name: String,
    pub class_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

impl Student {
    pub fn new(id: i64, student_id: String, first_name: String, last_name: String, class_id: Option<i64>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Student {
            id,
            student_id,
            first_name,
            last_name,
            class_id,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn update(&mut self, first_name: Option<String>, last_name: Option<String>, class_id: Option<i64>) {
        if let Some(fn_name) = first_name {
            self.first_name = fn_name;
        }
        if let Some(ln_name) = last_name {
            self.last_name = ln_name;
        }
        if let Some(cid) = class_id {
            self.class_id = Some(cid);
        }
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn is_assigned_to_class(&self) -> bool {
        self.class_id.is_some()
    }
}