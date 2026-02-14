// Domain entity for User
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub school_name: String,
    pub position: String,
    pub department: String,
    pub employee_id: String,
    pub organization_type: String,
    pub organization_name: String,
    pub created_at: DateTime<Utc>,
    pub last_login: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UserProfile {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub school_name: String,
    pub position: String,
    pub department: String,
    pub employee_id: String,
    pub organization_type: String,
    pub organization_name: String,
    pub created_at: DateTime<Utc>,
    pub last_login: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AuthResponse {
    pub success: bool,
    pub user: Option<UserProfile>,
    pub token: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub school_name: String,
}
