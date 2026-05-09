use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfcCard {
    pub id: String,
    pub card_number: String,
    pub teacher_id: Option<String>,
    pub is_active: bool,
    pub registered_at: String,
}
