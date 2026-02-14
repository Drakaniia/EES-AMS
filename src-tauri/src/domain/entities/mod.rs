// Domain Entities Module
// Pure business entities with no infrastructure dependencies

pub mod attendance;
pub mod class;
pub mod student;
pub mod sync_status;
pub mod user;

pub use attendance::{Attendance, AttendanceStatus};
pub use class::Class;
pub use student::Student;
pub use sync_status::SyncStatus;
