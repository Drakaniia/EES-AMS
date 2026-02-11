// Domain Services Module
// Contains business logic and use cases

pub mod class_service;
pub mod student_service;
pub mod attendance_service;

pub use class_service::{ClassService, ClassServiceImpl};
pub use student_service::{StudentService, StudentServiceImpl};
pub use attendance_service::{AttendanceService, AttendanceServiceImpl};