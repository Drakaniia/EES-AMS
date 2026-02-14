// External Services Module
// Integrations with external services

pub mod google_sync;
pub mod firebase;
pub mod hybrid_sync;

pub use google_sync::{GoogleSync, GoogleCredentials, TokenData};
pub use firebase::{FirebaseService, SyncResult, ConflictInfo, SyncMetadata};
pub use hybrid_sync::{HybridSyncService, SyncConfig, SyncSource, SyncDirection, ConflictResolutionStrategy};