// Class Service
// Business logic for class operations

use crate::domain::entities::class::Class;
use crate::domain::repositories::ClassRepository;
use crate::domain::errors::{DomainError, DomainResult};
use async_trait::async_trait;

#[async_trait]
#[allow(dead_code)]
pub trait ClassService: Send + Sync {
    async fn create_class(&self, name: String, section: Option<String>, school_year: Option<String>) -> DomainResult<Class>;
    async fn get_all_classes(&self) -> DomainResult<Vec<Class>>;
    #[allow(dead_code)]
    async fn get_class_by_id(&self, id: i64) -> DomainResult<Class>;
    async fn delete_class(&self, id: i64) -> DomainResult<()>;
}

pub struct ClassServiceImpl<R: ClassRepository> {
    class_repo: R,
}

impl<R: ClassRepository> ClassServiceImpl<R> {
    pub fn new(class_repo: R) -> Self {
        ClassServiceImpl { class_repo }
    }
}

#[async_trait]
impl<R: ClassRepository + Send + Sync> ClassService for ClassServiceImpl<R> {
    async fn create_class(&self, name: String, section: Option<String>, school_year: Option<String>) -> DomainResult<Class> {
        if name.trim().is_empty() {
            return Err(DomainError::ValidationError("Class name cannot be empty".to_string()));
        }

        let section_ref = section.as_deref();
        if self.class_repo.exists(&name, section_ref).await? {
            return Err(DomainError::AlreadyExists(format!(
                "Class '{}'{} already exists",
                name,
                section_ref.map(|s| format!(" - {}", s)).unwrap_or_default()
            )));
        }

        let id = self.class_repo.create(name.clone(), section, school_year).await?;
        Ok(Class::new(id, name, None, None))
    }

    async fn get_all_classes(&self) -> DomainResult<Vec<Class>> {
        self.class_repo.get_all().await
    }

    async fn get_class_by_id(&self, id: i64) -> DomainResult<Class> {
        self.class_repo.get_by_id(id).await
    }

    async fn delete_class(&self, id: i64) -> DomainResult<()> {
        // Verify class exists
        self.class_repo.get_by_id(id).await?;
        self.class_repo.delete(id).await
    }
}