// Infrastructure Layer Module
// Contains external dependencies and data persistence

pub mod database;
pub mod external;
pub mod config;

pub use database::*;
pub use external::*;