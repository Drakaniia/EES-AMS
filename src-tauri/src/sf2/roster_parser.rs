use crate::domain::error::{AppError, Result};
use crate::domain::models::{
    Class, CreateClassRequest, CreateStudentRequest, Settings, Student, StudentGender, StudentId,
    UpdateStudentRequest,
};
use crate::infrastructure::database::{ClassRepository, StudentRepository};
use crate::sf2::logic::{is_learner_name, normalize_learner_name, Sf2CellMark};
use crate::sf2::models::{Sf2StudentMappingRecord, Sf2WorkbookAnalysis, Sf2WorkbookLearner};

use std::collections::{HashMap, HashSet};

pub(crate) const SF2_NAME_COLUMN: &str = "C";

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub(crate) struct TemplateRosterSlot {
    pub row_index: u32,
    pub gender_block: &'static str,
}

#[derive(Debug, Clone)]
pub(crate) struct TemplateRosterAssignment {
    pub student: Student,
    pub slot: TemplateRosterSlot,
}

#[derive(Debug)]
pub(crate) struct WorkbookLearnerSync {
    pub(crate) student_mappings: Vec<Sf2StudentMappingRecord>,
    pub(crate) students_created: usize,
    pub(crate) students_reused: usize,
    pub(crate) students_updated: usize,
}

// ── Template roster slot definitions ──────────────────────────────────────────

pub(crate) fn template_roster_slots() -> Vec<TemplateRosterSlot> {
    let mut slots = Vec::new();
    // Male student rows: 8 to 28 (21 slots)
    // MALE TOTAL formula is at row 29 (not in slots)
    for row_index in 8..=28 {
        slots.push(TemplateRosterSlot {
            row_index,
            gender_block: "MALE",
        });
    }
    // Female student rows: 30 to 48 (19 slots)
    // FEMALE TOTAL formula is at row 49 (not in slots)
    for row_index in 30..=48 {
        slots.push(TemplateRosterSlot {
            row_index,
            gender_block: "FEMALE",
        });
    }
    slots
}

pub(crate) fn template_owns_roster(template: &crate::sf2::models::Sf2TemplateRecord) -> bool {
    template.source_hash.starts_with("bundled-")
}

/// Calculate expanded roster slots for bundled templates when the number of students
/// exceeds the standard 21 male + 19 female slot capacity.
///
/// Returns slot definitions with dynamically calculated row indices based on how many
/// extra rows would need to be inserted before the MALE TOTAL and FEMALE TOTAL rows.
pub(crate) fn expanded_roster_slots(
    male_count: usize,
    female_count: usize,
) -> Vec<TemplateRosterSlot> {
    let extra_male = male_count.saturating_sub(21) as u32;
    let mut slots = Vec::with_capacity(male_count + female_count);

    // Male rows: 8 to (8 + male_count - 1)
    for i in 0..male_count {
        slots.push(TemplateRosterSlot {
            row_index: 8 + i as u32,
            gender_block: "MALE",
        });
    }

    // Female rows start after MALE TOTAL row (originally at 29, shifted by extra_male)
    let female_start = 30 + extra_male;
    for i in 0..female_count {
        slots.push(TemplateRosterSlot {
            row_index: female_start + i as u32,
            gender_block: "FEMALE",
        });
    }

    slots
}

/// Returns the number of extra male and female rows needed to accommodate the given
/// student counts beyond the standard 21 male / 19 female slot capacity.
pub(crate) fn roster_expansion_needed(male_count: usize, female_count: usize) -> (u32, u32) {
    (
        male_count.saturating_sub(21) as u32,
        female_count.saturating_sub(19) as u32,
    )
}

/// Compute MALE TOTAL, FEMALE TOTAL, and Combined TOTAL row positions for a
/// bundled-template SF2 workbook, derived from the fixed slot layout instead of
/// hardcoded result values.
///
/// # Layout
///
/// | Section          | Rows                        | Count            |
/// |------------------|-----------------------------|------------------|
/// | Male slots       | 8 … (7 + male_capacity)     | `male_capacity`  |
/// | MALE TOTAL       | 8 + male_capacity           | —                |
/// | Female slots     | (30 + extra_male) …         | `female_capacity`|
/// |                  | (29 + extra_male + female_capacity) |          |
/// | FEMALE TOTAL     | 30 + extra_male + female_capacity   | —        |
/// | Combined TOTAL   | FEMALE TOTAL + 1            | —                |
///
/// Where:
/// - `male_capacity = max(21, male_count)`
/// - `female_capacity = max(19, female_count)`
/// - `extra_male = male_capacity - 21`
///
/// This is equivalent to `29 + extra_male` / `49 + extra_male + extra_female`
/// but derived from the first-slot positions (8 for male, 30 for female) rather
/// than hardcoded result values, making it self-documenting and robust against
/// layout changes.
pub(crate) fn bundled_template_total_rows(
    male_count: usize,
    female_count: usize,
) -> (u32, u32, u32) {
    let male_capacity = male_count.max(21) as u32;
    let female_capacity = female_count.max(19) as u32;
    let extra_male = male_capacity.saturating_sub(21u32);

    let male_total_row = 8u32 + male_capacity;
    let female_total_row = 30u32 + extra_male + female_capacity;
    let combined_total_row = female_total_row + 1;

    (male_total_row, female_total_row, combined_total_row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_total_rows_standard_21_19() {
        // 21 male, 19 female → no expansion needed
        let (m, f, c) = bundled_template_total_rows(21, 19);
        assert_eq!(m, 29, "male total");
        assert_eq!(f, 49, "female total");
        assert_eq!(c, 50, "combined total");
    }

    #[test]
    fn bundled_total_rows_under_capacity() {
        // 14 male, 10 female → still uses standard 21/19 slots
        let (m, f, c) = bundled_template_total_rows(14, 10);
        assert_eq!(m, 29, "male total should be 29 even with 14 students");
        assert_eq!(f, 49, "female total should be 49 even with 10 students");
        assert_eq!(c, 50, "combined total");
    }

    #[test]
    fn bundled_total_rows_expanded_male_only() {
        // 25 male, 19 female → 4 extra male rows
        let (m, f, c) = bundled_template_total_rows(25, 19);
        assert_eq!(m, 33, "male total: 8 + 25 = 33");
        assert_eq!(f, 53, "female total: 30 + 4 + 19 = 53");
        assert_eq!(c, 54, "combined total");
    }

    #[test]
    fn bundled_total_rows_expanded_both() {
        // 30 male, 30 female → 9 extra male, 11 extra female
        let (m, f, c) = bundled_template_total_rows(30, 30);
        assert_eq!(m, 38, "male total: 8 + 30 = 38");
        assert_eq!(f, 69, "female total: 30 + 9 + 30 = 69");
        assert_eq!(c, 70, "combined total");
    }

    #[test]
    fn bundled_total_rows_no_female() {
        // 21 male, 0 female → minimum 19 female slots
        let (m, f, c) = bundled_template_total_rows(21, 0);
        assert_eq!(m, 29, "male total");
        assert_eq!(
            f, 49,
            "female total: even with 0 students, 19 slots minimum"
        );
        assert_eq!(c, 50, "combined total");
    }

    #[test]
    fn bundled_total_rows_no_male() {
        // 0 male, 19 female → minimum 21 male slots
        let (m, f, c) = bundled_template_total_rows(0, 19);
        assert_eq!(m, 29, "male total: even with 0 students, 21 slots minimum");
        assert_eq!(f, 49, "female total");
        assert_eq!(c, 50, "combined total");
    }

    #[test]
    fn bundled_total_rows_max_expansion() {
        // 40 male, 35 female → extreme case
        let (m, f, c) = bundled_template_total_rows(40, 35);
        assert_eq!(m, 48, "male total: 8 + 40 = 48");
        assert_eq!(
            f,
            30 + 19 + 35,
            "female total: 30 + extra_male(19) + 35 = 84"
        );
        assert_eq!(c, 30 + 19 + 35 + 1, "combined total: 85");
    }
}

// ── Roster assignment ─────────────────────────────────────────────────────────

pub(crate) fn template_roster_assignments(
    students: &[Student],
) -> Result<Vec<TemplateRosterAssignment>> {
    let mut male_students = Vec::new();
    let mut female_students = Vec::new();
    let mut missing_gender = Vec::new();

    for student in students {
        match student.gender {
            Some(StudentGender::Male) => male_students.push(student),
            Some(StudentGender::Female) => female_students.push(student),
            None => missing_gender.push(student.name.trim().to_string()),
        }
    }

    if !missing_gender.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "Set Male/Female for these students before creating or updating the SF2 workbook: {}",
            missing_gender.join(", ")
        )));
    }

    // Use expanded slots if student count exceeds standard capacity
    let (extra_male, extra_female) =
        roster_expansion_needed(male_students.len(), female_students.len());
    let row_slots = if extra_male > 0 || extra_female > 0 {
        expanded_roster_slots(male_students.len(), female_students.len())
    } else {
        template_roster_slots()
    };

    let male_slots = row_slots
        .iter()
        .copied()
        .filter(|slot| slot.gender_block == StudentGender::Male.sf2_block())
        .collect::<Vec<_>>();
    let female_slots = row_slots
        .iter()
        .copied()
        .filter(|slot| slot.gender_block == StudentGender::Female.sf2_block())
        .collect::<Vec<_>>();

    let mut assignments = Vec::with_capacity(students.len());
    assignments.extend(
        male_students
            .into_iter()
            .zip(male_slots)
            .map(|(student, slot)| TemplateRosterAssignment {
                student: student.clone(),
                slot,
            }),
    );
    assignments.extend(
        female_students
            .into_iter()
            .zip(female_slots)
            .map(|(student, slot)| TemplateRosterAssignment {
                student: student.clone(),
                slot,
            }),
    );
    Ok(assignments)
}

// ── Student mappings from roster assignments ──────────────────────────────────

pub(crate) fn student_mappings_from_roster_assignments(
    template_id: &str,
    assignments: &[TemplateRosterAssignment],
) -> Vec<Sf2StudentMappingRecord> {
    let mut seen_normalized_names = HashSet::new();
    assignments
        .iter()
        .map(|assignment| {
            let student = &assignment.student;
            let slot = assignment.slot;
            let normalized_name = unique_normalized_name(
                &mut seen_normalized_names,
                &student.name,
                &student.id.to_string(),
            );
            Sf2StudentMappingRecord {
                template_id: template_id.to_string(),
                student_id: student.id.to_string(),
                workbook_name: student.name.clone(),
                normalized_name,
                row_index: slot.row_index,
                gender_block: Some(slot.gender_block.to_string()),
            }
        })
        .collect()
}

// ── Unique normalized name deduplication ──────────────────────────────────────

pub(crate) fn unique_normalized_name(
    seen: &mut HashSet<String>,
    name: &str,
    suffix: &str,
) -> String {
    let normalized = normalize_learner_name(name);
    if seen.insert(normalized.clone()) {
        return normalized;
    }

    let unique = format!("{normalized}#{suffix}");
    seen.insert(unique.clone());
    unique
}

// ── Duplicate name rejection ──────────────────────────────────────────────────

pub(crate) fn reject_duplicate_roster_names(students: &[Student]) -> Result<()> {
    let mut names_by_normalized: HashMap<String, Vec<String>> = HashMap::new();
    for student in students {
        names_by_normalized
            .entry(normalize_learner_name(&student.name))
            .or_default()
            .push(student.name.clone());
    }

    let duplicates = names_by_normalized
        .into_values()
        .filter(|names| names.len() > 1)
        .map(|names| names.join(", "))
        .collect::<Vec<_>>();

    if duplicates.is_empty() {
        return Ok(());
    }

    Err(AppError::InvalidInput(format!(
        "Duplicate learner names must be corrected before creating an SF2 workbook: {}",
        duplicates.join("; ")
    )))
}

// ── Roster name marks for Excel ───────────────────────────────────────────────

pub(crate) fn roster_name_marks(
    analysis: &Sf2WorkbookAnalysis,
    assignments: &[TemplateRosterAssignment],
) -> Vec<Sf2CellMark> {
    let sheet_names = analysis
        .sheets
        .iter()
        .filter(|sheet| sheet.visible != 0)
        .map(|sheet| sheet.name.clone())
        .collect::<Vec<_>>();

    let mut marks = Vec::with_capacity(sheet_names.len() * assignments.len());
    for sheet_name in sheet_names {
        for assignment in assignments {
            marks.push(Sf2CellMark {
                sheet_name: sheet_name.clone(),
                cell_address: format!("{SF2_NAME_COLUMN}{}", assignment.slot.row_index),
                value: assignment.student.name.trim().to_string(),
            });
        }
    }
    marks
}

// ── Students from draft learner names ─────────────────────────────────────────

pub(crate) fn roster_students_for_draft(
    student_repo: &StudentRepository,
    class_id: &str,
    learner_names: &[String],
) -> Result<(Vec<Student>, usize, usize)> {
    let existing_students = student_repo.list_by_class(Some(class_id))?;
    let mut existing_by_name: HashMap<String, Student> = existing_students
        .iter()
        .cloned()
        .map(|student| (normalize_learner_name(&student.name), student))
        .collect();

    let mut requested_names = Vec::new();
    let mut seen_names = HashSet::new();
    for name in learner_names.iter().map(|name| name.trim()) {
        if name.is_empty() || !is_learner_name(name) {
            continue;
        }

        let normalized = normalize_learner_name(name);
        if seen_names.insert(normalized) {
            requested_names.push(name.to_string());
        }
    }

    if requested_names.is_empty() {
        let reused = existing_students.len();
        return Ok((existing_students, 0, reused));
    }

    let mut students = Vec::with_capacity(requested_names.len());
    let mut students_created = 0;
    let mut students_reused = 0;

    for name in requested_names {
        let normalized = normalize_learner_name(&name);
        let student = if let Some(student) = existing_by_name.get(&normalized) {
            students_reused += 1;
            student.clone()
        } else {
            let created = student_repo.create(CreateStudentRequest {
                name: name.clone(),
                gender: None,
                card_serial: None,
                class_id: Some(class_id.to_string()),
            })?;
            existing_by_name.insert(normalized, created.clone());
            students_created += 1;
            created
        };
        students.push(student);
    }

    Ok((students, students_created, students_reused))
}

// ── Workbook learner mappings sync ────────────────────────────────────────────

pub(crate) fn sync_workbook_learner_mappings(
    student_repo: &StudentRepository,
    class_id: &str,
    template_id: &str,
    learners: &[Sf2WorkbookLearner],
) -> Result<WorkbookLearnerSync> {
    sync_workbook_learner_mappings_with_old(student_repo, class_id, template_id, learners, &[])
}

/// Sync workbook learner mappings with optional old template mappings for name update on re-import.
///
/// When `old_mappings` is provided (non-empty on re-import), learners that don't match any
/// existing student by normalized name are matched by row index against the old template's
/// mappings. If a row match is found, the existing student's name is UPDATED to match the
/// workbook name instead of creating a duplicate student.
pub(crate) fn sync_workbook_learner_mappings_with_old(
    student_repo: &StudentRepository,
    class_id: &str,
    template_id: &str,
    learners: &[Sf2WorkbookLearner],
    old_mappings: &[Sf2StudentMappingRecord],
) -> Result<WorkbookLearnerSync> {
    let existing_students = student_repo.list_by_class(Some(class_id))?;
    // Build a lookup by student ID BEFORE consuming existing_students via into_iter()
    let old_student_by_id: HashMap<StudentId, Student> = existing_students
        .iter()
        .map(|s| (s.id, s.clone()))
        .collect();
    let mut existing_by_name: HashMap<String, Student> = existing_students
        .into_iter()
        .map(|student| (normalize_learner_name(&student.name), student))
        .collect();

    // Build a row index → old mapping lookup for re-import name updates
    let old_by_row: HashMap<u32, &Sf2StudentMappingRecord> =
        old_mappings.iter().map(|m| (m.row_index, m)).collect();

    let mut seen_names = HashSet::new();
    let mut student_mappings = Vec::new();
    let mut students_created = 0;
    let mut students_reused = 0;
    let mut students_updated = 0;

    for learner in learners
        .iter()
        .filter(|learner| is_learner_name(&learner.name))
    {
        let normalized_name = normalize_learner_name(&learner.name);
        if !seen_names.insert(normalized_name.clone()) {
            continue;
        }
        let learner_gender = StudentGender::from_sf2_block(learner.gender_block.as_deref());

        let student = if let Some(student) = existing_by_name.get(&normalized_name) {
            // Name match: reuse existing student (existing behavior)
            students_reused += 1;
            let mut student = student.clone();
            if let Some(gender) = learner_gender {
                if student.gender != Some(gender) {
                    student = student_repo.update(
                        student.id,
                        UpdateStudentRequest {
                            name: None,
                            gender: Some(gender),
                            card_serial: None,
                            class_id: None,
                        },
                    )?;
                    existing_by_name.insert(normalized_name.clone(), student.clone());
                }
            }
            student
        } else if let Some(old_mapping) = old_by_row.get(&learner.row_index) {
            // No name match but row matches an old template mapping:
            // this is a renamed student — update the existing student's name
            match uuid::Uuid::parse_str(&old_mapping.student_id) {
                Ok(uuid) => {
                    let old_student_id = StudentId(uuid);
                    // Find the old student by ID from the pre-built lookup
                    if let Some(old_student) = old_student_by_id.get(&old_student_id) {
                        let updated = student_repo.update(
                            old_student.id,
                            UpdateStudentRequest {
                                name: Some(learner.name.trim().to_string()),
                                gender: learner_gender,
                                card_serial: None,
                                class_id: None,
                            },
                        )?;
                        let updated_normalized = normalize_learner_name(&updated.name);
                        existing_by_name.insert(updated_normalized, updated.clone());
                        students_updated += 1;
                        updated
                    } else {
                        // Old student not found — create new
                        let created = student_repo.create(CreateStudentRequest {
                            name: learner.name.clone(),
                            gender: learner_gender,
                            card_serial: None,
                            class_id: Some(class_id.to_string()),
                        })?;
                        existing_by_name.insert(normalized_name.clone(), created.clone());
                        students_created += 1;
                        created
                    }
                }
                Err(_) => {
                    // If we can't parse the student ID, create a new student instead
                    let created = student_repo.create(CreateStudentRequest {
                        name: learner.name.clone(),
                        gender: learner_gender,
                        card_serial: None,
                        class_id: Some(class_id.to_string()),
                    })?;
                    existing_by_name.insert(normalized_name.clone(), created.clone());
                    students_created += 1;
                    created
                }
            }
        } else {
            // No match at all: create new student
            let created = student_repo.create(CreateStudentRequest {
                name: learner.name.clone(),
                gender: learner_gender,
                card_serial: None,
                class_id: Some(class_id.to_string()),
            })?;
            existing_by_name.insert(normalized_name.clone(), created.clone());
            students_created += 1;
            created
        };

        student_mappings.push(Sf2StudentMappingRecord {
            template_id: template_id.to_string(),
            student_id: student.id.to_string(),
            workbook_name: learner.name.clone(),
            normalized_name,
            row_index: learner.row_index,
            gender_block: learner.gender_block.clone(),
        });
    }

    Ok(WorkbookLearnerSync {
        student_mappings,
        students_created,
        students_reused,
        students_updated,
    })
}

// ── Clear unused learner marks for SF2 workbooks ────────────────────────────

/// The standard column letters for the SF2 learner info section (learner number, LRN, learner name).
const SF2_LEARNER_INFO_COLUMNS: [&str; 3] = ["A", "B", "C"];

/// Generate marks to clear unused learner rows (columns A, B, C) for all visible sheets.
/// Used after roster sync to remove leftover/empty learner rows in the learner range.
///
/// For bundled templates, `expanded_male_count` and `expanded_female_count` should be provided
/// when the template has been dynamically expanded beyond the standard 21 male + 19 female slots.
/// When omitted (None), the standard range (rows 8-28 male, 30-48 female) is used.
pub(crate) fn clear_unused_learner_marks(
    analysis: &Sf2WorkbookAnalysis,
    mapped_rows: &[u32],
    expanded_male_count: Option<usize>,
    expanded_female_count: Option<usize>,
) -> Vec<Sf2CellMark> {
    let sheet_names: Vec<String> = analysis
        .sheets
        .iter()
        .filter(|sheet| sheet.visible != 0)
        .map(|sheet| sheet.name.clone())
        .collect();

    let all_possible_rows = if let (Some(male_count), Some(female_count)) =
        (expanded_male_count, expanded_female_count)
    {
        // Expanded bundled template: use dynamic row range
        let extra_male = male_count.saturating_sub(21) as u32;
        let mut rows = Vec::new();
        rows.extend(8u32..(8 + male_count as u32));
        let female_start = 30 + extra_male;
        rows.extend(female_start..(female_start + female_count as u32));
        rows
    } else {
        // Standard bundled or imported workbook: use standard DepEd range
        let mut rows = Vec::new();
        rows.extend(8u32..=28);
        rows.extend(30u32..=48);
        rows
    };

    let mapped: HashSet<u32> = mapped_rows.iter().copied().collect();
    let unused_rows: Vec<&u32> = all_possible_rows
        .iter()
        .filter(|r| !mapped.contains(r))
        .collect();

    if unused_rows.is_empty() {
        return Vec::new();
    }

    let mut marks =
        Vec::with_capacity(sheet_names.len() * unused_rows.len() * SF2_LEARNER_INFO_COLUMNS.len());
    for sheet_name in sheet_names {
        for col in &SF2_LEARNER_INFO_COLUMNS {
            for row in &unused_rows {
                marks.push(Sf2CellMark {
                    sheet_name: sheet_name.clone(),
                    cell_address: format!("{col}{row}"),
                    value: String::new(),
                });
            }
        }
    }
    marks
}

// ── Find or create class ──────────────────────────────────────────────────────

pub(crate) fn find_or_create_class(
    class_repo: &ClassRepository,
    class_name: &str,
    settings: Option<&Settings>,
) -> Result<Class> {
    if let Some(existing) = class_repo
        .list()?
        .into_iter()
        .find(|class: &Class| class.name.eq_ignore_ascii_case(class_name))
    {
        return Ok(existing);
    }

    let day_start = settings
        .map(|s| s.day_start.clone())
        .unwrap_or_else(|| "08:00".to_string());
    let day_end = settings
        .map(|s| s.day_end.clone())
        .unwrap_or_else(|| "15:00".to_string());
    let late_after = settings
        .map(|s| s.late_after.clone())
        .unwrap_or_else(|| "08:45".to_string());

    class_repo.create(CreateClassRequest {
        name: class_name.to_string(),
        room: Some("N/A".to_string()),
        day_start,
        day_end,
        late_after,
        sessions: Vec::new(),
        days: vec![1, 2, 3, 4, 5],
    })
}
