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

impl std::fmt::Display for StudentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Student gender used by DepEd SF2 male/female roster sections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StudentGender {
    Male,
    Female,
}

impl StudentGender {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::Male => "male",
            Self::Female => "female",
        }
    }

    pub fn sf2_block(self) -> &'static str {
        match self {
            Self::Male => "MALE",
            Self::Female => "FEMALE",
        }
    }

    pub fn from_db_value(value: Option<&str>) -> Option<Self> {
        match value.map(str::trim) {
            Some(value) if value.eq_ignore_ascii_case("male") => Some(Self::Male),
            Some(value) if value.eq_ignore_ascii_case("female") => Some(Self::Female),
            _ => None,
        }
    }

    pub fn from_sf2_block(value: Option<&str>) -> Option<Self> {
        match value.map(str::trim) {
            Some(value) if value.eq_ignore_ascii_case("MALE") => Some(Self::Male),
            Some(value) if value.eq_ignore_ascii_case("FEMALE") => Some(Self::Female),
            _ => None,
        }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<StudentGender>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_serial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Attendance event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttendanceType {
    In,
}

/// Attendance interface mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AttendanceMode {
    #[default]
    Manual,
    CardReader,
    #[serde(other)]
    Unknown,
}

impl AttendanceMode {
    pub fn normalize(self) -> Self {
        match self {
            Self::CardReader => Self::CardReader,
            Self::Manual | Self::Unknown => Self::Manual,
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "card_reader" => Self::CardReader,
            "manual" => Self::Manual,
            _ => Self::Manual,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self.normalize() {
            Self::Manual | Self::Unknown => "manual",
            Self::CardReader => "card_reader",
        }
    }
}

/// Session record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub late_after: String,
}

/// Class record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Class {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    pub day_start: String,
    pub day_end: String,
    pub late_after: String,
    #[serde(default)]
    pub sessions: Vec<Session>,
    #[serde(default)]
    pub days: Vec<i32>,
    pub created_at: DateTime<Utc>,
}

/// Attendance event record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceEvent {
    pub id: EventId,
    pub student_id: StudentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: AttendanceType,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Audit entry for attendance event override, edit, and delete actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceAuditEntry {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    pub student_id: StudentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    pub action: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub actor: String,
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub id: String,
    pub day_start: String,  // "08:30"
    pub day_end: String,    // "15:30"
    pub late_after: String, // "08:45"
    pub quarter: String,    // "1st Quarter"
    #[serde(default)]
    pub attendance_mode: AttendanceMode,
    pub q1_start: Option<String>,
    pub q1_end: Option<String>,
    pub q2_start: Option<String>,
    pub q2_end: Option<String>,
    pub q3_start: Option<String>,
    pub q3_end: Option<String>,
    pub school_id: Option<String>,
    pub school_name: Option<String>,
    pub school_year: Option<String>,
    pub report_month: Option<String>,
    pub grade_level: Option<String>,
    pub section: Option<String>,
    pub adviser_name: Option<String>,
    pub school_head_name: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: "app".to_string(),
            day_start: "08:30".to_string(),
            day_end: "15:30".to_string(),
            late_after: "08:45".to_string(),
            quarter: "1st Quarter".to_string(),
            attendance_mode: AttendanceMode::Manual,
            q1_start: None,
            q1_end: None,
            q2_start: None,
            q2_end: None,
            q3_start: None,
            q3_end: None,
            school_id: None,
            school_name: None,
            school_year: None,
            report_month: None,
            grade_level: None,
            section: None,
            adviser_name: None,
            school_head_name: None,
        }
    }
}

/// Request to create a new student
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStudentRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<StudentGender>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_serial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_id: Option<String>,
}

/// Request to update a student
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStudentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<StudentGender>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_serial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_id: Option<String>,
}

/// Request to create a new class
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateClassRequest {
    pub name: String,
    #[serde(default)]
    pub room: Option<String>,
    pub day_start: String,
    pub day_end: String,
    pub late_after: String,
    #[serde(default)]
    pub sessions: Vec<Session>,
    #[serde(default)]
    pub days: Vec<i32>,
}

/// Request to update a class
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateClassRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub late_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<Vec<Session>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<Vec<i32>>,
}

/// Request to create an attendance event
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventRequest {
    pub student_id: StudentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: AttendanceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

/// Request to update an attendance event
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    pub reason: String,
}

/// Export data structure
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportData {
    pub students: Vec<Student>,
    pub classes: Vec<Class>,
    pub events: Vec<AttendanceEvent>,
    pub settings: Vec<Settings>,
    pub exported_at: i64,
}
