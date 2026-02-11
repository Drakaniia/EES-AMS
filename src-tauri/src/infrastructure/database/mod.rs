// Database Module
// JSON file-based database implementation

pub mod schema;
pub mod json_db;
pub mod class_repository_impl;
pub mod student_repository_impl;
pub mod attendance_repository_impl;
pub mod settings_repository_impl;

pub use json_db::JsonDatabase;
pub use class_repository_impl::ClassRepositoryImpl;
pub use student_repository_impl::StudentRepositoryImpl;
pub use attendance_repository_impl::AttendanceRepositoryImpl;
pub use settings_repository_impl::SettingsRepositoryImpl;