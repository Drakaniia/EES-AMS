// Domain Entity: Attendance
// Represents an attendance record in the system

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Attendance status options
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
pub enum AttendanceStatus {
    Present,
    Absent,
    Late,
    Excused,
}

impl AttendanceStatus {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "present" => AttendanceStatus::Present,
            "absent" => AttendanceStatus::Absent,
            "late" => AttendanceStatus::Late,
            "excused" => AttendanceStatus::Excused,
            _ => AttendanceStatus::Absent,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            AttendanceStatus::Present => "present",
            AttendanceStatus::Absent => "absent",
            AttendanceStatus::Late => "late",
            AttendanceStatus::Excused => "excused",
        }
    }
}

/// Represents an attendance record
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct Attendance {
    pub id: i64,
    pub student_id: i64,
    pub class_id: i64,
    pub date: String,
    pub status: AttendanceStatus,
    pub notes: Option<String>,
    pub synced: bool,
    pub created_at: String,
}

impl Attendance {
    pub fn new(
        id: i64,
        student_id: i64,
        class_id: i64,
        date: String,
        status: AttendanceStatus,
        notes: Option<String>,
    ) -> Self {
        Attendance {
            id,
            student_id,
            class_id,
            date,
            status,
            notes,
            synced: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn mark_as_synced(&mut self) {
        self.synced = true;
    }

    pub fn update_status(&mut self, status: AttendanceStatus, notes: Option<String>) {
        self.status = status;
        self.notes = notes;
        self.synced = false;
    }
}

/// Attendance statistics for a class on a specific date
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AttendanceStats {
    pub total_students: i32,
    pub present_count: i32,
    pub absent_count: i32,
    pub late_count: i32,
    pub excused_count: i32,
    pub attendance_rate: i32,
}

impl AttendanceStats {
    pub fn new(total: i32) -> Self {
        AttendanceStats {
            total_students: total,
            present_count: 0,
            absent_count: 0,
            late_count: 0,
            excused_count: 0,
            attendance_rate: 0,
        }
    }

    pub fn calculate_rate(&mut self) {
        if self.total_students > 0 {
            let present = self.present_count + self.late_count;
            self.attendance_rate =
                ((present as f32 / self.total_students as f32) * 100.0).round() as i32;
        }
    }
}
