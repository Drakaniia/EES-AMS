// Domain Entity: SyncStatus
// Represents the current synchronization status

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Represents the current synchronization status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SyncStatus {
    pub is_online: bool,
    pub last_sync_time: Option<String>,
    pub pending_records: i32,
    pub is_syncing: bool,
    pub error: Option<String>,
}

impl SyncStatus {
    pub fn new() -> Self {
        SyncStatus {
            is_online: true,
            last_sync_time: None,
            pending_records: 0,
            is_syncing: false,
            error: None,
        }
    }

    pub fn set_syncing(&mut self, syncing: bool) {
        self.is_syncing = syncing;
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }

    pub fn update_sync_time(&mut self, time: String) {
        self.last_sync_time = Some(time);
    }

    pub fn set_pending(&mut self, count: i32) {
        self.pending_records = count;
    }
}
