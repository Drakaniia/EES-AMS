// Command Handlers Module
// Application-layer handlers that coordinate between Tauri commands and domain services

pub mod class_handler;
pub mod student_handler;
pub mod attendance_handler;
pub mod google_handler;
pub mod update_handler;
pub mod auth_handler;

pub use class_handler::ClassHandler;
pub use student_handler::StudentHandler;
pub use attendance_handler::AttendanceHandler;
pub use google_handler::GoogleHandler;
// pub use auth_handler::AuthHandler;
