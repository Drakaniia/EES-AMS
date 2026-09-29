use crate::domain::error::Result;
use crate::domain::models::{
    CreateStudentRequest, Student, StudentGender, StudentId, UpdateStudentRequest,
};
use crate::infrastructure::database::StudentRepository;
use crate::sf2::logic::{is_learner_name, normalize_learner_name};
use crate::sf2::models::{Sf2StudentMappingRecord, Sf2WorkbookLearner};
use crate::sf2::month::{
    match_roster_learner, Sf2LearnerMatch, Sf2LearnerMatchKind, Sf2MonthStudentMapping,
};
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
/// Learners are identified in the order the spec sets out (spec §6.3):
/// `sf2_learner_id` → `normalized_name` → `row_index`, which is exactly what
/// [`match_roster_learner`] implements. A learner ID is the school's own record
/// and survives a rename *and* a re-sort; a name survives a row move; a row index
/// says nothing about the learner at all, and matching on it alone is what
/// re-points a student's X marks at somebody else when the roster is reshuffled
/// between two months (E7). So the row is the last resort, and it is logged when
/// it is reached.
///
/// When `old_mappings` is provided (non-empty on re-import), learners that don't
/// match any existing student by name fall through to the previous template's
/// mappings. If a row match is found, the existing student's name is UPDATED to
/// match the workbook name instead of creating a duplicate student.
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

    // The previous template's mappings, in the shape the shared matcher reads.
    let reference = month_mappings_for(old_mappings, &old_student_by_id);

    let mut claimed_students: HashSet<String> = HashSet::new();
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
        } else if let Some(matched) = unclaimed_match(&reference, learner, &claimed_students) {
            // No name match in the class, but the learner is already a student -
            // found by DepEd ID, by the name they had in the previous file, or
            // (last, and logged) by the row they occupied.
            if matched.matched_by == Sf2LearnerMatchKind::RowIndex {
                log::warn!(
                    "roster sync: `{}` at row {} matched an existing student by position only; \
                     a reshuffled roster can attach one student's X marks to another",
                    learner.name.trim(),
                    learner.row_index
                );
            }
            match reuse_matched_student(
                student_repo,
                &matched,
                &old_student_by_id,
                &mut existing_by_name,
                learner,
                learner_gender,
            )? {
                ReusedStudent::Found { student, renamed } => {
                    if renamed {
                        students_updated += 1;
                    } else {
                        students_reused += 1;
                    }
                    student
                }
                ReusedStudent::Missing => {
                    let created = create_learner(
                        student_repo,
                        class_id,
                        learner,
                        learner_gender,
                        &mut existing_by_name,
                        &normalized_name,
                    )?;
                    students_created += 1;
                    created
                }
            }
        } else {
            // No match at all: create new student
            let created = create_learner(
                student_repo,
                class_id,
                learner,
                learner_gender,
                &mut existing_by_name,
                &normalized_name,
            )?;
            students_created += 1;
            created
        };
        claimed_students.insert(student.id.to_string());

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

/// The previous template's mappings, in the shape [`match_roster_learner`] reads,
/// with each learner's DepEd ID filled in from the student record.
///
/// The legacy mappings have no ID of their own - the DepEd ID was never read
/// from the workbook before spec §6.3 - so without this the `sf2_learner_id`
/// branch of the matcher could never fire on the re-import path and the promised
/// order `sf2_learner_id` -> `normalized_name` -> `row_index` was really just
/// `normalized_name` -> `row_index`. The ID lives on `students` (v20), and the
/// split backfills it, so the student's own record is where the reference has to
/// read it from.
///
/// A student with no ID contributes `None`, and the matcher falls through to the
/// name - which is the correct outcome on the bundled template, whose merged
/// "No."/ID cell means the school never gives us a real one.
fn month_mappings_for(
    old_mappings: &[Sf2StudentMappingRecord],
    students_by_id: &HashMap<StudentId, Student>,
) -> Vec<Sf2MonthStudentMapping> {
    old_mappings
        .iter()
        .map(|mapping| {
            let sf2_learner_id = uuid::Uuid::parse_str(&mapping.student_id)
                .ok()
                .and_then(|uuid| students_by_id.get(&StudentId(uuid)))
                .and_then(|student| student.sf2_learner_id.clone());
            Sf2MonthStudentMapping {
                template_id: mapping.template_id.clone(),
                student_id: mapping.student_id.clone(),
                workbook_name: mapping.workbook_name.clone(),
                normalized_name: mapping.normalized_name.clone(),
                row_index: mapping.row_index,
                gender_block: mapping.gender_block.clone(),
                sf2_learner_id,
            }
        })
        .collect()
}

/// The first matcher result that is not already spoken for in this run.
///
/// `sf2_month_student_mappings` keys on `(template_id, student_id)`, so two
/// workbook rows that both resolve to one student would collide. A reshuffled
/// roster can produce exactly that, and dropping the second match here is what
/// keeps it from becoming a duplicate student or a failed insert.
fn unclaimed_match(
    reference: &[Sf2MonthStudentMapping],
    learner: &Sf2WorkbookLearner,
    claimed_students: &HashSet<String>,
) -> Option<Sf2LearnerMatch> {
    match_roster_learner(reference, learner)
        .filter(|matched| !claimed_students.contains(&matched.student_id))
}

enum ReusedStudent {
    Found { student: Student, renamed: bool },
    Missing,
}

/// Reuse the student a matched mapping points at, or report that they are gone.
///
/// A name that differs is the workbook being authoritative about a learner the
/// app already knows - the same learner, spelled the way the school now spells
/// it - so the student's name follows the workbook, which is what the pre-split
/// code did for a row match.
fn reuse_matched_student(
    student_repo: &StudentRepository,
    matched: &Sf2LearnerMatch,
    old_student_by_id: &HashMap<StudentId, Student>,
    existing_by_name: &mut HashMap<String, Student>,
    learner: &Sf2WorkbookLearner,
    learner_gender: Option<StudentGender>,
) -> Result<ReusedStudent> {
    // A mapping whose id is not a UUID cannot name a student, so the learner is
    // treated as unknown and created rather than guessed at.
    let Ok(uuid) = uuid::Uuid::parse_str(&matched.student_id) else {
        return Ok(ReusedStudent::Missing);
    };
    let Some(old_student) = old_student_by_id.get(&StudentId(uuid)) else {
        return Ok(ReusedStudent::Missing);
    };
    let updated = student_repo.update(
        old_student.id,
        UpdateStudentRequest {
            name: Some(learner.name.trim().to_string()),
            gender: learner_gender,
            card_serial: None,
            class_id: None,
        },
    )?;
    existing_by_name.insert(normalize_learner_name(&updated.name), updated.clone());
    Ok(ReusedStudent::Found {
        student: updated,
        renamed: true,
    })
}

/// Create the student for a learner the app has no record of.
fn create_learner(
    student_repo: &StudentRepository,
    class_id: &str,
    learner: &Sf2WorkbookLearner,
    learner_gender: Option<StudentGender>,
    existing_by_name: &mut HashMap<String, Student>,
    normalized_name: &str,
) -> Result<Student> {
    let created = student_repo.create(CreateStudentRequest {
        name: learner.name.clone(),
        gender: learner_gender,
        card_serial: None,
        class_id: Some(class_id.to_string()),
    })?;
    existing_by_name.insert(normalized_name.to_string(), created.clone());
    Ok(created)
}
