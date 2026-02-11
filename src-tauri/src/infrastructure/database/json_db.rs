// JSON Database
// Thread-safe JSON file-based database

use crate::domain::errors::{DomainError, DomainResult};
use crate::infrastructure::database::schema::DatabaseSchema;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct JsonDatabase {
    data: Mutex<DatabaseSchema>,
    db_path: PathBuf,
}

impl JsonDatabase {
    pub fn new(app_data_dir: PathBuf) -> DomainResult<Self> {
        let db_path = app_data_dir.join("attendance-data.json");
        
        let data = if db_path.exists() {
            let content = fs::read_to_string(&db_path)
                .map_err(|e| DomainError::InfrastructureError(format!("Failed to read database: {}", e)))?;
            serde_json::from_str(&content)
                .map_err(|e| DomainError::InfrastructureError(format!("Failed to parse database: {}", e)))?
        } else {
            DatabaseSchema::default()
        };

        let db = JsonDatabase {
            data: Mutex::new(data),
            db_path,
        };

        db.save()?;
        Ok(db)
    }

    pub fn save(&self) -> DomainResult<()> {
        let data = self.data.lock().unwrap();
        let json = serde_json::to_string_pretty(&*data)
            .map_err(|e| DomainError::InfrastructureError(format!("Failed to serialize database: {}", e)))?;
        fs::write(&self.db_path, json)
            .map_err(|e| DomainError::InfrastructureError(format!("Failed to write database: {}", e)))?;
        Ok(())
    }
}