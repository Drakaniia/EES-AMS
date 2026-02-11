// Domain Entities Module
// Pure business entities with no infrastructure dependencies

pub mod class;
pub mod student;
pub mod attendance;
pub mod sync_status;

pub use class::Class;
pub use student::Student;
pub use attendance::{Attendance, AttendanceStatus};
pub use sync_status::SyncStatus;