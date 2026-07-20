use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const SF2_PRESENT_MARK: &str = "";
const SF2_ABSENT_MARK: &str = "X";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2StudentMapping {
    pub student_id: String,
    pub sheet_name: String,
    pub row_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2AttendanceEvent {
    pub student_id: String,
    pub event_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2CellMark {
    pub sheet_name: String,
    pub cell_address: String,
    pub value: String,
}

pub fn normalize_learner_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(", ", ",")
        .trim()
        .to_uppercase()
}

pub fn is_learner_name(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    let normalized = normalize_learner_name(trimmed);
    if normalized == "NAME (LAST NAME,FIRST NAME,MIDDLE NAME)"
        || normalized.contains("TOTAL PER DAY")
        || normalized.contains("COMBINED TOTAL")
        || normalized.contains("<===")
        || normalized.contains("===")
    {
        return false;
    }

    normalized.contains(',') && normalized.chars().any(|c| c.is_alphabetic())
}

/// Check if a day has any attendance taken based on present events.
/// A day has "attendance taken" when at least one "in" event exists.
pub fn day_has_attendance_taken(present_events: &[Sf2AttendanceEvent]) -> bool {
    present_events.iter().any(|event| event.event_type == "in")
}

#[cfg(test)]
#[path = "__tests__/logic_tests.rs"]
mod tests;

pub fn attendance_marks_for_closed_day(
    students: &[Sf2StudentMapping],
    present_events: &[Sf2AttendanceEvent],
    column_letter: &str,
) -> Vec<Sf2CellMark> {
    let present_student_ids: HashSet<&str> = present_events
        .iter()
        .filter(|event| event.event_type == "in")
        .map(|event| event.student_id.as_str())
        .collect();

    students
        .iter()
        .map(|student| Sf2CellMark {
            sheet_name: student.sheet_name.clone(),
            cell_address: format!("{column_letter}{}", student.row_index),
            value: if present_student_ids.contains(student.student_id.as_str()) {
                SF2_PRESENT_MARK
            } else {
                SF2_ABSENT_MARK
            }
            .to_string(),
        })
        .collect()
}
