use super::*;

#[test]
fn day_has_attendance_taken_true_when_any_in_event_exists() {
    let events = vec![Sf2AttendanceEvent {
        student_id: "s1".to_string(),
        event_type: "in".to_string(),
    }];
    assert!(day_has_attendance_taken(&events));
}

#[test]
fn day_has_attendance_taken_true_when_only_absent_event_exists() {
    // A day with only an explicit absent record still counts as having
    // attendance taken - the absent X must be written.
    let events = vec![Sf2AttendanceEvent {
        student_id: "s1".to_string(),
        event_type: "absent".to_string(),
    }];
    assert!(day_has_attendance_taken(&events));
}

#[test]
fn day_has_attendance_taken_false_when_no_events() {
    let events: Vec<Sf2AttendanceEvent> = vec![];
    assert!(!day_has_attendance_taken(&events));
}

#[test]
fn attendance_marks_for_day_marks_only_absent_students() {
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
    let absent_ids: HashSet<String> = ["s1".to_string()].into_iter().collect();

    let marks = attendance_marks_for_day(&students, &absent_ids, "F");

    assert_eq!(marks.len(), 2);
    assert_eq!(marks[0].value, "X"); // s1 has an explicit absent record
    assert_eq!(marks[1].value, ""); // s2 untouched → present by default
}

#[test]
fn attendance_marks_for_day_with_no_absents_writes_empty() {
    let students = vec![Sf2StudentMapping {
        student_id: "s1".to_string(),
        sheet_name: "JULY 2026".to_string(),
        row_index: 8,
    }];
    let absent_ids: HashSet<String> = HashSet::new();

    let marks = attendance_marks_for_day(&students, &absent_ids, "F");

    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].value, ""); // present = empty string
    assert_eq!(marks[0].cell_address, "F8");
    assert_eq!(marks[0].sheet_name, "JULY 2026");
}
