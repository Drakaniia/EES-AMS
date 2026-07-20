use super::*;

// ── preview_cell_status ────────────────────────────────────────────────

#[test]
fn present_when_student_has_event() {
    // Student has an "in" event → Present (empty cell)
    assert_eq!(
        preview_cell_status(true, true),
        Sf2PreviewCellStatus::Present
    );
}

#[test]
fn open_when_day_is_future() {
    // Future day without attendance → Present (clickable), same as past days
    // without attendance. No Open status anymore.
    assert_eq!(
        preview_cell_status(false, false),
        Sf2PreviewCellStatus::Present
    );
    assert_eq!(
        preview_cell_status(false, true),
        Sf2PreviewCellStatus::Absent
    );
}

#[test]
fn absent_when_day_has_attendance_but_student_is_not_present() {
    // Day had attendance taken, student absent → Absent (X)
    assert_eq!(
        preview_cell_status(false, true),
        Sf2PreviewCellStatus::Absent
    );
}

#[test]
fn present_when_day_has_no_attendance_taken() {
    // No attendance taken at all → Present (empty) so the cell
    // is clickable (regardless of past/future). Clicking it creates
    // "in" events for all other students and marks this student as Absent (X).
    assert_eq!(
        preview_cell_status(false, false),
        Sf2PreviewCellStatus::Present
    );
}

#[test]
fn present_overrides_everything() {
    // Present always takes priority, regardless of attendance state
    assert_eq!(
        preview_cell_status(true, false),
        Sf2PreviewCellStatus::Present
    );
    assert_eq!(
        preview_cell_status(true, false),
        Sf2PreviewCellStatus::Present
    );
    assert_eq!(
        preview_cell_status(true, true),
        Sf2PreviewCellStatus::Present
    );
}
