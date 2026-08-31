use crate::domain::error::Result;
use crate::domain::models::{
    CreateStudentRequest, Student, StudentGender, StudentId, UpdateStudentRequest,
};
use crate::infrastructure::database::StudentRepository;
use crate::sf2::logic::{is_learner_name, normalize_learner_name};
use crate::sf2::models::{Sf2StudentMappingRecord, Sf2WorkbookLearner};
use crate::sf2::roster_parser::WorkbookLearnerSync;

use std::collections::{HashMap, HashSet};

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
