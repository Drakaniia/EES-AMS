use super::*;

// ── preview_cell_status ────────────────────────────────────────────────

#[test]
fn present_when_student_has_event() {
    // Student has an "in" event → Present (empty cell)
    assert_eq!(
        preview_cell_status(true, false, true),
        Sf2PreviewCellStatus::Present
    );
}

#[test]
fn open_when_day_is_future() {
    // Future day → Open regardless of attendance
    assert_eq!(
        preview_cell_status(false, true, true),
        Sf2PreviewCellStatus::Open
    );
    assert_eq!(
        preview_cell_status(false, true, false),
        Sf2PreviewCellStatus::Open
    );
}

#[test]
fn absent_when_day_has_attendance_but_student_is_not_present() {
    // Day had attendance taken, student absent → Absent (X)
    assert_eq!(
        preview_cell_status(false, false, true),
        Sf2PreviewCellStatus::Absent
    );
}

#[test]
fn present_when_day_has_no_attendance_taken() {
    // Past day, no attendance taken at all → Present (empty) so the cell
    // is clickable. Clicking it creates "in" events for all other students
    // and marks this student as Absent (X).
    assert_eq!(
        preview_cell_status(false, false, false),
        Sf2PreviewCellStatus::Present
    );
}

#[test]
fn present_overrides_everything() {
    // Present always takes priority, even if future or no attendance
    assert_eq!(
        preview_cell_status(true, true, false),
        Sf2PreviewCellStatus::Present
    );
    assert_eq!(
        preview_cell_status(true, false, false),
        Sf2PreviewCellStatus::Present
    );
    assert_eq!(
        preview_cell_status(true, true, true),
        Sf2PreviewCellStatus::Present
    );
}
