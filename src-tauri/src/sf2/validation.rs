use crate::domain::error::{AppError, Result};
use crate::domain::models::Student;
use crate::infrastructure::database::{ClassRepository, DbPool, StudentRepository};
use crate::sf2::logic::{is_learner_name, normalize_learner_name};
use crate::sf2::models::{
    Sf2ImportValidation, Sf2ValidationDuplicate, Sf2ValidationLearner, Sf2ValidationNameMismatch,
    Sf2ValidationStudent, Sf2WorkbookAnalysis, Sf2WorkbookLearner,
};
use crate::sf2::naming::class_name;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(super) fn import_validation_from_analysis(
    pool: DbPool,
    workbook_path: &Path,
    analysis: &Sf2WorkbookAnalysis,
) -> Result<Sf2ImportValidation> {
    let class_name = class_name(&analysis.grade_level, &analysis.section);
    let class_repo = ClassRepository::new(pool.clone());
    let class = class_repo
        .list()?
        .into_iter()
        .find(|class| class.name.eq_ignore_ascii_case(&class_name));
    let current_students = match class.as_ref() {
        Some(class) => StudentRepository::new(pool).list_by_class(Some(&class.id))?,
        None => Vec::new(),
    };

    Ok(validate_student_list(
        &workbook_path.to_string_lossy(),
        class.as_ref().map(|class| class.id.as_str()),
        &class_name,
        &current_students,
        &analysis.learners,
    ))
}

pub(super) fn validate_student_list(
    source_path: &str,
    class_id: Option<&str>,
    class_name: &str,
    current_students: &[Student],
    learners: &[Sf2WorkbookLearner],
) -> Sf2ImportValidation {
    let current = current_students
        .iter()
        .map(validation_student)
        .collect::<Vec<_>>();
    let valid_learners = learners
        .iter()
        .filter(|learner| is_learner_name(&learner.name))
        .map(validation_learner)
        .collect::<Vec<_>>();
    let current_names = current
        .iter()
        .map(|student| student.normalized_name.as_str())
        .collect::<HashSet<_>>();
    let sf2_names = valid_learners
        .iter()
        .map(|learner| learner.normalized_name.as_str())
        .collect::<HashSet<_>>();

    let missing_from_sf2 = current
        .iter()
        .filter(|student| !sf2_names.contains(student.normalized_name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let missing_from_current = valid_learners
        .iter()
        .filter(|learner| !current_names.contains(learner.normalized_name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let possible_name_mismatches =
        possible_name_mismatches(&missing_from_sf2, &missing_from_current);
    let duplicate_current_students = duplicate_current_students(&current);
    let duplicate_sf2_learners = duplicate_sf2_learners(&valid_learners);
    let missing_learner_info = learners
        .iter()
        .filter(|learner| {
            learner.name.trim().is_empty()
                || (is_learner_name(&learner.name) && learner.gender_block.is_none())
        })
        .map(validation_learner)
        .collect::<Vec<_>>();
    let has_discrepancies = !missing_from_sf2.is_empty()
        || !missing_from_current.is_empty()
        || !possible_name_mismatches.is_empty()
        || !duplicate_current_students.is_empty()
        || !duplicate_sf2_learners.is_empty()
        || !missing_learner_info.is_empty();

    Sf2ImportValidation {
        source_path: source_path.to_string(),
        class_id: class_id.map(str::to_string),
        class_name: class_name.to_string(),
        current_student_count: current.len(),
        sf2_learner_count: valid_learners.len(),
        missing_from_sf2,
        missing_from_current,
        possible_name_mismatches,
        duplicate_current_students,
        duplicate_sf2_learners,
        missing_learner_info,
        has_discrepancies,
    }
}

pub(super) fn ensure_import_validation_allows(
    validation: &Sf2ImportValidation,
    proceed_anyway: bool,
) -> Result<()> {
    if validation.has_discrepancies && !proceed_anyway {
        return Err(AppError::InvalidInput(
            "Student List Mismatch Detected. Review the validation report and explicitly proceed before importing this SF2 workbook."
                .to_string(),
        ));
    }

    Ok(())
}

fn validation_student(student: &Student) -> Sf2ValidationStudent {
    Sf2ValidationStudent {
        student_id: student.id.to_string(),
        name: student.name.clone(),
        normalized_name: normalize_learner_name(&student.name),
        gender: student
            .gender
            .map(|gender| gender.as_db_value().to_string()),
    }
}

fn validation_learner(learner: &Sf2WorkbookLearner) -> Sf2ValidationLearner {
    Sf2ValidationLearner {
        row_index: learner.row_index,
        name: learner.name.clone(),
        normalized_name: normalize_learner_name(&learner.name),
        gender_block: learner.gender_block.clone(),
    }
}

fn possible_name_mismatches(
    current: &[Sf2ValidationStudent],
    learners: &[Sf2ValidationLearner],
) -> Vec<Sf2ValidationNameMismatch> {
    let mut mismatches = Vec::new();
    for student in current {
        for learner in learners {
            let Some(reason) =
                name_mismatch_reason(&student.normalized_name, &learner.normalized_name)
            else {
                continue;
            };
            mismatches.push(Sf2ValidationNameMismatch {
                current_student: student.clone(),
                sf2_learner: learner.clone(),
                reason,
            });
        }
    }

    mismatches
}

fn name_mismatch_reason(current: &str, learner: &str) -> Option<String> {
    if current == learner {
        return None;
    }

    if token_signature(current) == token_signature(learner) {
        return Some("Same name tokens in a different order".to_string());
    }

    let current_compact = compact_name(current);
    let learner_compact = compact_name(learner);
    let longest = current_compact.len().max(learner_compact.len());
    if longest >= 6 && edit_distance(&current_compact, &learner_compact) <= 2 {
        return Some("Very similar spelling".to_string());
    }

    None
}

fn token_signature(value: &str) -> Vec<String> {
    let mut tokens = value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    tokens.sort();
    tokens
}

fn compact_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_len = right.chars().count();
    let mut previous = (0..=right_len).collect::<Vec<_>>();
    let mut current = vec![0; right_len + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right.chars().enumerate() {
            let substitution_cost = usize::from(left_char != right_char);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution_cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_len]
}

fn duplicate_current_students(students: &[Sf2ValidationStudent]) -> Vec<Sf2ValidationDuplicate> {
    let mut grouped: HashMap<&str, Vec<&Sf2ValidationStudent>> = HashMap::new();
    for student in students {
        grouped
            .entry(student.normalized_name.as_str())
            .or_default()
            .push(student);
    }

    grouped
        .into_iter()
        .filter_map(|(normalized_name, students)| {
            (students.len() > 1).then(|| Sf2ValidationDuplicate {
                normalized_name: normalized_name.to_string(),
                names: students
                    .iter()
                    .map(|student| student.name.clone())
                    .collect(),
                student_ids: students
                    .iter()
                    .map(|student| student.student_id.clone())
                    .collect(),
                row_indexes: Vec::new(),
            })
        })
        .collect()
}

fn duplicate_sf2_learners(learners: &[Sf2ValidationLearner]) -> Vec<Sf2ValidationDuplicate> {
    let mut grouped: HashMap<&str, Vec<&Sf2ValidationLearner>> = HashMap::new();
    for learner in learners {
        grouped
            .entry(learner.normalized_name.as_str())
            .or_default()
            .push(learner);
    }

    grouped
        .into_iter()
        .filter_map(|(normalized_name, learners)| {
            (learners.len() > 1).then(|| Sf2ValidationDuplicate {
                normalized_name: normalized_name.to_string(),
                names: learners
                    .iter()
                    .map(|learner| learner.name.clone())
                    .collect(),
                student_ids: Vec::new(),
                row_indexes: learners.iter().map(|learner| learner.row_index).collect(),
            })
        })
        .collect()
}
