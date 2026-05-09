use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttendanceStatus {
    Present,
    Absent,
    Late,
    HalfDay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceRecord {
    pub id: String,
    pub teacher_id: String,
    pub date: String,
    pub time_in: Option<String>,
    pub time_out: Option<String>,
    pub status: AttendanceStatus,
    pub sfc_card_id: Option<String>,
    pub created_at: String,
}
