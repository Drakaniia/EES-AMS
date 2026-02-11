// Domain Entity: Class
// Represents a class in the attendance management system

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Represents a class or section in the school
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct Class {
    pub id: i64,
    pub name: String,
    pub section: Option<String>,
    pub school_year: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Class {
    pub fn new(id: i64, name: String, section: Option<String>, school_year: Option<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Class {
            id,
            name,
            section,
            school_year,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn update(&mut self, name: Option<String>, section: Option<String>, school_year: Option<String>) {
        if let Some(n) = name {
            self.name = n;
        }
        if let Some(s) = section {
            self.section = Some(s);
        }
        if let Some(sy) = school_year {
            self.school_year = Some(sy);
        }
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn display_name(&self) -> String {
        match &self.section {
            Some(section) => format!("{} - {}", self.name, section),
            None => self.name.clone(),
        }
    }
}