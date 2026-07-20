use super::*;

// ── unmapped_roster_issue ─────────────────────────────────────────────

#[test]
fn unmapped_roster_issue_single_student() {
    let names = vec!["Juan".to_string()];
    let msg = unmapped_roster_issue(&names);
    assert!(msg.contains("Juan"));
    assert!(msg.contains("is"));
    assert!(msg.contains("not mapped"));
}

#[test]
fn unmapped_roster_issue_two_students() {
    let names = vec!["Juan".to_string(), "Maria".to_string()];
    let msg = unmapped_roster_issue(&names);
    assert!(msg.contains("Juan"));
    assert!(msg.contains("Maria"));
    assert!(msg.contains("are"));
    assert!(msg.contains("not mapped"));
}

#[test]
fn unmapped_roster_issue_shows_first_five() {
    let names = (1..=7).map(|i| format!("Student{i}")).collect::<Vec<_>>();
    let msg = unmapped_roster_issue(&names);
    assert!(msg.contains("Student1"));
    assert!(msg.contains("Student5"));
    assert!(msg.contains(", and 2 more"));
    assert!(msg.contains("are"));
}

#[test]
fn unmapped_roster_issue_exactly_five() {
    let names = (1..=5).map(|i| format!("Student{i}")).collect::<Vec<_>>();
    let msg = unmapped_roster_issue(&names);
    assert!(msg.contains("Student5"));
    assert!(!msg.contains("more"), "should not have 'more' suffix");
    assert!(msg.contains("are"));
}

#[test]
fn unmapped_roster_issue_zero_students() {
    let names: Vec<String> = vec![];
    let msg = unmapped_roster_issue(&names);
    assert!(msg.starts_with(" are"));
    assert!(msg.contains("not mapped"));
}

#[test]
fn unmapped_roster_issue_exactly_one_more_after_five() {
    let names = (1..=6).map(|i| format!("Student{i}")).collect::<Vec<_>>();
    let msg = unmapped_roster_issue(&names);
    assert!(msg.contains(", and 1 more"));
}
