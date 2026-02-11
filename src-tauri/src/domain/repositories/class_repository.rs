// Repository Trait: ClassRepository
// Abstract interface for class data access

use async_trait::async_trait;
use crate::domain::entities::class::Class;
use crate::domain::errors::{DomainError, DomainResult};

#[async_trait]
pub trait ClassRepository: Send + Sync {
    /// Create a new class
    async fn create(&self, name: String, section: Option<String>, school_year: Option<String>) -> DomainResult<i64>;

    /// Get all classes, sorted by name
    async fn get_all(&self) -> DomainResult<Vec<Class>>;

    /// Get a class by ID
    async fn get_by_id(&self, id: i64) -> DomainResult<Class>;

    /// Delete a class by ID
    async fn delete(&self, id: i64) -> DomainResult<()>;

    /// Check if a class with the same name and section exists
    async fn exists(&self, name: &str, section: Option<&str>) -> DomainResult<bool>;
}