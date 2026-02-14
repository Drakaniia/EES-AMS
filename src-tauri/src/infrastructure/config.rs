// Configuration Module
// Application configuration management

#![allow(dead_code)]

use std::env;

pub struct AppConfig {
    pub app_data_dir: std::path::PathBuf,
    pub firebase: FirebaseConfig,
    pub google_drive: GoogleDriveConfig,
    pub database: DatabaseConfig,
    pub sync: SyncConfig,
}

pub struct FirebaseConfig {
    pub project_id: String,
    pub service_account_key_path: String,
    pub api_key: String,
    pub auth_domain: String,
    pub database_url: String,
}

pub struct GoogleDriveConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

pub struct DatabaseConfig {
    pub path: String,
}

pub struct SyncConfig {
    pub interval_minutes: u64,
    pub auto_sync_enabled: bool,
}

impl AppConfig {
    pub fn new(app_data_dir: std::path::PathBuf) -> Self {
        // Load environment variables from .env file
        dotenv::dotenv().ok();

        AppConfig {
            app_data_dir,
            firebase: FirebaseConfig {
                project_id: env::var("FIREBASE_PROJECT_ID")
                    .unwrap_or_else(|_| "default-project".to_string()),
                service_account_key_path: env::var("FIREBASE_SERVICE_ACCOUNT_KEY_PATH")
                    .unwrap_or_else(|_| "./firebase-service-account.json".to_string()),
                api_key: env::var("FIREBASE_API_KEY").unwrap_or_else(|_| "".to_string()),
                auth_domain: env::var("FIREBASE_AUTH_DOMAIN")
                    .unwrap_or_else(|_| "default.firebaseapp.com".to_string()),
                database_url: env::var("FIREBASE_DATABASE_URL").unwrap_or_else(|_| "".to_string()),
            },
            google_drive: GoogleDriveConfig {
                client_id: env::var("GOOGLE_DRIVE_CLIENT_ID").unwrap_or_else(|_| "".to_string()),
                client_secret: env::var("GOOGLE_DRIVE_CLIENT_SECRET")
                    .unwrap_or_else(|_| "".to_string()),
                redirect_url: env::var("GOOGLE_DRIVE_REDIRECT_URL")
                    .unwrap_or_else(|_| "http://localhost:8080/callback".to_string()),
            },
            database: DatabaseConfig {
                path: env::var("DATABASE_PATH").unwrap_or_else(|_| "./data".to_string()),
            },
            sync: SyncConfig {
                interval_minutes: env::var("SYNC_INTERVAL_MINUTES")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
                auto_sync_enabled: env::var("AUTO_SYNC_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
        }
    }

    pub fn is_firebase_configured(&self) -> bool {
        !self.firebase.project_id.is_empty() && !self.firebase.service_account_key_path.is_empty()
    }

    pub fn is_google_drive_configured(&self) -> bool {
        !self.google_drive.client_id.is_empty() && !self.google_drive.client_secret.is_empty()
    }
}
