// Repository Trait: SettingsRepository
// Abstract interface for settings data access

use async_trait::async_trait;
use crate::domain::errors::DomainResult;

#[async_trait]
#[allow(dead_code)]
pub trait SettingsRepository: Send + Sync {
    /// Get a setting value by key
    async fn get(&self, key: &str) -> DomainResult<Option<String>>;

    /// Set a setting value
    async fn set(&self, key: String, value: String) -> DomainResult<()>;

    /// Delete a setting by key
    #[allow(dead_code)]
    async fn delete(&self, key: &str) -> DomainResult<()>;

    /// Get all settings
    #[allow(dead_code)]
    async fn get_all(&self) -> DomainResult<std::collections::HashMap<String, String>>;
}