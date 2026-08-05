use super::*;

// ── preview_cell_status ────────────────────────────────────────────────

#[test]
fn present_when_student_has_no_absent_record_on_closed_day() {
    // Day has attendance, student has no explicit absent record → Present
    // (empty cell), present by default.
    assert_eq!(
        preview_cell_status(false, true),
        Sf2PreviewCellStatus::Present
    );
}

#[test]
fn absent_when_student_has_explicit_absent_record() {
    // Explicit absent record → Absent (X), regardless of other records.
    assert_eq!(
        preview_cell_status(true, true),
        Sf2PreviewCellStatus::Absent
    );
}

#[test]
fn open_when_day_has_no_attendance_at_all() {
    // No records on the day → Open (nothing taken yet, cell editable).
    assert_eq!(
        preview_cell_status(false, false),
        Sf2PreviewCellStatus::Open
    );
}
