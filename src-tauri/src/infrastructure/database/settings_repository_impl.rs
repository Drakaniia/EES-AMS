// Settings Repository Implementation
// JSON file-based implementation of SettingsRepository trait

use async_trait::async_trait;
use crate::domain::repositories::SettingsRepository;
use crate::domain::errors::{DomainError, DomainResult};
use crate::infrastructure::database::JsonDatabase;

pub struct SettingsRepositoryImpl {
    db: JsonDatabase,
}

impl SettingsRepositoryImpl {
    pub fn new(db: JsonDatabase) -> Self {
        SettingsRepositoryImpl { db }
    }
}

#[async_trait]
impl SettingsRepository for SettingsRepositoryImpl {
    async fn get(&self, key: &str) -> DomainResult<Option<String>> {
        let data = self.db.get_data().lock().unwrap();
        Ok(data.settings.get(key).cloned())
    }

    async fn set(&self, key: String, value: String) -> DomainResult<()> {
        let mut data = self.db.get_data().lock().unwrap();
        data.settings.insert(key, value);
        drop(data);
        self.db.save()?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> DomainResult<()> {
        let mut data = self.db.get_data().lock().unwrap();
        data.settings.remove(key);
        drop(data);
        self.db.save()?;
        Ok(())
    }

    async fn get_all(&self) -> DomainResult<std::collections::HashMap<String, String>> {
        let data = self.db.get_data().lock().unwrap();
        Ok(data.settings.clone())
    }
}