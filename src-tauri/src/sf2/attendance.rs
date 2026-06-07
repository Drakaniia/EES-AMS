use crate::domain::models::{AttendanceEvent, Student};
use crate::sf2::logic::Sf2AttendanceEvent;
use chrono::Local;
use std::collections::HashSet;

pub(super) fn present_events_for_day(
    events: &[AttendanceEvent],
    students: &[Student],
    class_id: &str,
    date: &str,
) -> Vec<Sf2AttendanceEvent> {
    let student_ids: HashSet<String> = students
        .iter()
        .map(|student| student.id.to_string())
        .collect();
    events
        .iter()
        .filter(|event| {
            event_belongs_to_class(event, &student_ids, class_id) && local_event_date(event) == date
        })
        .map(|event| Sf2AttendanceEvent {
            student_id: event.student_id.to_string(),
            event_type: "in".to_string(),
        })
        .collect()
}

pub(super) fn present_student_ids(
    events: &[AttendanceEvent],
    students: &[Student],
    class_id: &str,
    date: &str,
) -> HashSet<String> {
    present_events_for_day(events, students, class_id, date)
        .into_iter()
        .map(|event| event.student_id)
        .collect()
}

fn event_belongs_to_class(
    event: &AttendanceEvent,
    class_student_ids: &HashSet<String>,
    class_id: &str,
) -> bool {
    event.class_id.as_deref() == Some(class_id)
        || class_student_ids.contains(&event.student_id.to_string())
}

fn local_event_date(event: &AttendanceEvent) -> String {
    event
        .timestamp
        .with_timezone(&Local)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}
