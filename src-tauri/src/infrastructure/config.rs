// Configuration Module
// Application configuration management

pub struct AppConfig {
    pub app_data_dir: std::path::PathBuf,
}

impl AppConfig {
    pub fn new(app_data_dir: std::path::PathBuf) -> Self {
        AppConfig { app_data_dir }
    }
}