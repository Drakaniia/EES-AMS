//! SQLite database infrastructure.
mod audit;
mod classes;
mod events;
mod migrations;
mod rows;
mod settings;
mod students;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

pub use audit::{record_audit_event, AuditEventInput, AuditRepository};
pub use classes::ClassRepository;
pub use events::EventRepository;
pub use migrations::{init_db, migrate_db, CURRENT_SCHEMA_VERSION};
pub use settings::SettingsRepository;
pub use students::StudentRepository;

/// Database connection pool type
pub type DbPool = Pool<SqliteConnectionManager>;