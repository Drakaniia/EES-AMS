use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Teacher {
    pub id: String,
    pub employee_id: String,
    pub first_name: String,
    pub last_name: String,
    pub department: String,
    pub position: String,
    pub sfc_card_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
