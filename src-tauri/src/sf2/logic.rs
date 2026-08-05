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

/// Check if a day has any attendance taken.
/// A day has "attendance taken" when at least one event exists - either an "in"
/// (present) record or an explicit "absent" record.
pub fn day_has_attendance_taken(day_events: &[Sf2AttendanceEvent]) -> bool {
    !day_events.is_empty()
}

#[cfg(test)]
#[path = "__tests__/logic_tests.rs"]
mod tests;

/// Generate Excel marks for a day's attendance.
///
/// With explicit absent records, the X mark is written ONLY for students who
/// have an explicit absent event. Everyone else (recorded present or untouched)
/// stays blank - present by default, matching the SF2 opt-out model.
pub fn attendance_marks_for_day(
    students: &[Sf2StudentMapping],
    absent_student_ids: &HashSet<String>,
    column_letter: &str,
) -> Vec<Sf2CellMark> {
    students
        .iter()
        .map(|student| Sf2CellMark {
            sheet_name: student.sheet_name.clone(),
            cell_address: format!("{column_letter}{}", student.row_index),
            value: if absent_student_ids.contains(&student.student_id) {
                SF2_ABSENT_MARK
            } else {
                SF2_PRESENT_MARK
            }
            .to_string(),
        })
        .collect()
}
