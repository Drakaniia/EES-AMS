// Repository interface for User operations
use async_trait::async_trait;
use crate::domain::entities::user::{User, UserProfile};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: &User) -> Result<i64, String>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, String>;
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, String>;
    async fn update_last_login(&self, id: i64) -> Result<(), String>;
    async fn update_profile(&self, id: i64, profile: &UserProfile) -> Result<(), String>;
    async fn delete(&self, id: i64) -> Result<(), String>;
    async fn get_user_profile(&self, id: i64) -> Result<Option<UserProfile>, String>;
}