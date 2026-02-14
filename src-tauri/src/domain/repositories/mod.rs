// Repository Traits Module
// Abstract interfaces for data access

pub mod class_repository;
pub mod student_repository;
pub mod attendance_repository;
pub mod settings_repository;
pub mod user_repository;

pub use class_repository::ClassRepository;
pub use student_repository::StudentRepository;
pub use attendance_repository::AttendanceRepository;
pub use settings_repository::SettingsRepository;
pub use user_repository::UserRepository;