use super::*;

// ── attendance_changed_since ────────────────────────────────────────

#[test]
fn no_events_after_a_sync_still_requires_sync() {
    // A rebuilt/reset database has no events but keeps the old last_synced_at.
    // Reporting "in sync" here left the workbook holding marks the app had no
    // record of, permanently, because nothing ever rewrote it.
    assert!(
        attendance_changed_since(Some(1000), None),
        "an empty database must still reconcile the workbook back to agreement"
    );
}

#[test]
fn no_events_and_never_synced_skips_sync() {
    // Nothing has ever been written and there is nothing to write, so opening
    // the workbook must stay instant.
    assert!(
        !attendance_changed_since(None, None),
        "a never-synced workbook with no events has nothing to reconcile"
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
