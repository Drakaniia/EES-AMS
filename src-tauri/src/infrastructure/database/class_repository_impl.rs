// Class Repository Implementation
// JSON file-based implementation of ClassRepository trait

use async_trait::async_trait;
use crate::domain::entities::class::Class;
use crate::domain::repositories::ClassRepository;
use crate::domain::errors::{DomainError, DomainResult};
use crate::infrastructure::database::schema::ClassRecord;
use crate::infrastructure::database::JsonDatabase;

pub struct ClassRepositoryImpl {
    db: JsonDatabase,
}

impl ClassRepositoryImpl {
    pub fn new(db: JsonDatabase) -> Self {
        ClassRepositoryImpl { db }
    }

    fn record_to_entity(record: &ClassRecord) -> Class {
        Class {
            id: record.id,
            name: record.name.clone(),
            section: record.section.clone(),
            school_year: record.school_year.clone(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        }
    }
}

#[async_trait]
impl ClassRepository for ClassRepositoryImpl {
    async fn create(&self, name: String, section: Option<String>, school_year: Option<String>) -> DomainResult<i64> {
        let mut data = self.db.get_data().lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        data.counters.classes += 1;
        let id = data.counters.classes;

        let class = ClassRecord {
            id,
            name: name.clone(),
            section,
            school_year,
            created_at: now.clone(),
            updated_at: now,
        };

        data.classes.push(class);
        drop(data);
        
        self.db.save()?;
        Ok(id)
    }

    async fn get_all(&self) -> DomainResult<Vec<Class>> {
        let data = self.db.get_data().lock().unwrap();
        let mut classes: Vec<Class> = data.classes.iter()
            .map(Self::record_to_entity)
            .collect();
        classes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(classes)
    }

    async fn get_by_id(&self, id: i64) -> DomainResult<Class> {
        let data = self.db.get_data().lock().unwrap();
        data.classes.iter()
            .find(|c| c.id == id)
            .map(Self::record_to_entity)
            .ok_or_else(|| DomainError::NotFound(format!("Class with id {} not found", id)))
    }

    async fn delete(&self, id: i64) -> DomainResult<()> {
        let mut data = self.db.get_data().lock().unwrap();
        let original_len = data.classes.len();
        data.classes.retain(|c| c.id != id);
        
        if data.classes.len() == original_len {
            return Err(DomainError::NotFound(format!("Class with id {} not found", id)));
        }
        
        drop(data);
        self.db.save()?;
        Ok(())
    }

    async fn exists(&self, name: &str, section: Option<&str>) -> DomainResult<bool> {
        let data = self.db.get_data().lock().unwrap();
        Ok(data.classes.iter().any(|c| c.name == name && c.section.as_deref() == section))
    }
}