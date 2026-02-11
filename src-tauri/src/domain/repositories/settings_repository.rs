// Repository Trait: SettingsRepository
// Abstract interface for settings data access

use async_trait::async_trait;
use crate::domain::errors::{DomainError, DomainResult};

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    /// Get a setting value by key
    async fn get(&self, key: &str) -> DomainResult<Option<String>>;

    /// Set a setting value
    async fn set(&self, key: String, value: String) -> DomainResult<()>;

    /// Delete a setting by key
    async fn delete(&self, key: &str) -> DomainResult<()>;

    /// Get all settings
    async fn get_all(&self) -> DomainResult<std::collections::HashMap<String, String>>;
}