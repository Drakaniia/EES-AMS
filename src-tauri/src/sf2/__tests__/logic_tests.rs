use super::*;

#[test]
fn day_has_attendance_taken_true_when_any_in_event_exists() {
    let events = vec![
        Sf2AttendanceEvent {
            student_id: "s1".to_string(),
            event_type: "in".to_string(),
        },
    ];
    assert!(day_has_attendance_taken(&events));
}

#[test]
fn day_has_attendance_taken_false_when_no_events() {
    let events: Vec<Sf2AttendanceEvent> = vec![];
    assert!(!day_has_attendance_taken(&events));
}

#[test]
fn attendance_marks_for_closed_day_with_empty_present_events_marks_all_absent() {
    let students = vec![
        Sf2StudentMapping {
            student_id: "s1".to_string(),
            sheet_name: "JULY 2026".to_string(),
            row_index: 8,
        },
        Sf2StudentMapping {
            student_id: "s2".to_string(),
            sheet_name: "JULY 2026".to_string(),
            row_index: 9,
        },
    ];
    let present_events: Vec<Sf2AttendanceEvent> = vec![];

    let marks = attendance_marks_for_closed_day(&students, &present_events, "F");

    assert_eq!(marks.len(), 2);
    assert_eq!(marks[0].value, "X");
    assert_eq!(marks[1].value, "X");
}

#[test]
fn attendance_marks_for_closed_day_with_present_student_writes_empty() {
    let students = vec![
        Sf2StudentMapping {
            student_id: "s1".to_string(),
            sheet_name: "JULY 2026".to_string(),
            row_index: 8,
        },
    ];
    let present_events = vec![
        Sf2AttendanceEvent {
            student_id: "s1".to_string(),
            event_type: "in".to_string(),
        },
    ];

    let marks = attendance_marks_for_closed_day(&students, &present_events, "F");

    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].value, "");  // present = empty string
    assert_eq!(marks[0].cell_address, "F8");
    assert_eq!(marks[0].sheet_name, "JULY 2026");
}
