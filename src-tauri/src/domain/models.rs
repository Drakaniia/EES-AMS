/// Domain models for the attendance system
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a student
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StudentId(pub uuid::Uuid);

impl StudentId {
    #[inline]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for StudentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for an attendance event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub uuid::Uuid);

impl EventId {
    #[inline]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

/// Student record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Student {
    pub id: StudentId,
    pub name: String,
    pub student_number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_serial: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Attendance event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttendanceType {
    In,
    Out,
}

/// Attendance event record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceEvent {
    pub id: EventId,
    pub student_id: StudentId,
    #[serde(rename = "type")]
    pub event_type: AttendanceType,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub class_name: String,
    pub day_start: String,  // "08:30"
    pub day_end: String,    // "15:30"
    pub late_after: String, // "08:45"
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            class_name: "My Class".to_string(),
            day_start: "08:30".to_string(),
            day_end: "15:30".to_string(),
            late_after: "08:45".to_string(),
        }
    }
}

/// Request to create a new student
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStudentRequest {
    pub name: String,
    pub student_number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_serial: Option<String>,
}

/// Request to update a student
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStudentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub student_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_serial: Option<String>,
}

/// Request to create an attendance event
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventRequest {
    pub student_id: StudentId,
    #[serde(rename = "type")]
    pub event_type: AttendanceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Export data structure
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportData {
    pub students: Vec<Student>,
    pub events: Vec<AttendanceEvent>,
    pub settings: Settings,
    pub exported_at: DateTime<Utc>,
}
