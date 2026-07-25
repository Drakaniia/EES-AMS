use super::*;

// ── attendance_changed_since ────────────────────────────────────────

#[test]
fn no_events_means_no_sync_needed() {
    assert!(
        !attendance_changed_since(Some(1000), None),
        "with no attendance events, the workbook is already current"
    );
}

#[test]
fn never_synced_with_events_requires_sync() {
    assert!(
        attendance_changed_since(None, Some(500)),
        "if the workbook was never synced but has events, we must sync"
    );
}

#[test]
fn event_after_last_sync_requires_sync() {
    assert!(
        attendance_changed_since(Some(1000), Some(1001)),
        "an event newer than the last sync means the workbook is stale"
    );
}

#[test]
fn event_equal_to_last_sync_skips_sync() {
    assert!(
        !attendance_changed_since(Some(1000), Some(1000)),
        "an event exactly at the last sync time is already written"
    );
}

#[test]
fn event_before_last_sync_skips_sync() {
    assert!(
        !attendance_changed_since(Some(1000), Some(999)),
        "events older than the last sync are already reflected"
    );
}
