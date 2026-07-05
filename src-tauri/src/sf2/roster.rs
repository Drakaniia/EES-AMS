use crate::domain::error::{AppError, Result};
use crate::domain::models::{
    Class, CreateClassRequest, CreateStudentRequest, Settings, Student, StudentGender,
    UpdateStudentRequest,
};
use crate::infrastructure::database::{ClassRepository, StudentRepository};
use crate::sf2::logic::{is_learner_name, normalize_learner_name, Sf2CellMark};
use crate::sf2::models::{
    Sf2StudentMappingRecord, Sf2WorkbookAnalysis, Sf2WorkbookLearner,
};

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
}

// ── Template roster slot definitions ──────────────────────────────────────────

pub(crate) fn template_roster_slots() -> Vec<TemplateRosterSlot> {
    let mut slots = Vec::new();
    for row_index in 8..=28 {
        slots.push(TemplateRosterSlot {
            row_index,
            gender_block: "MALE",
        });
    }
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

// ── Roster assignment ─────────────────────────────────────────────────────────

pub(crate) fn template_roster_assignments(
    students: &[Student],
) -> Result<Vec<TemplateRosterAssignment>> {
    let row_slots = template_roster_slots();
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
    if male_students.len() > male_slots.len() {
        return Err(AppError::InvalidInput(format!(
            "The bundled SF2 template has {} male learner rows, but this class has {} male learners",
            male_slots.len(),
            male_students.len()
        )));
    }
    if female_students.len() > female_slots.len() {
        return Err(AppError::InvalidInput(format!(
            "The bundled SF2 template has {} female learner rows, but this class has {} female learners",
            female_slots.len(),
            female_students.len()
        )));
    }

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
            let value = assignment.student.name.trim().to_string();
            marks.push(Sf2CellMark {
                sheet_name: sheet_name.clone(),
                cell_address: format!("{SF2_NAME_COLUMN}{}", assignment.slot.row_index),
                value,
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
    let existing_students = student_repo.list_by_class(Some(class_id))?;
    let mut existing_by_name: HashMap<String, Student> = existing_students
        .into_iter()
        .map(|student| (normalize_learner_name(&student.name), student))
        .collect();
    let mut seen_names = HashSet::new();
    let mut student_mappings = Vec::new();
    let mut students_created = 0;
    let mut students_reused = 0;

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
        } else {
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
    })
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
        .unwrap_or_else(|| "08:30".to_string());
    let day_end = settings
        .map(|s| s.day_end.clone())
        .unwrap_or_else(|| "15:30".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{Student, StudentId, StudentGender};
    use crate::sf2::logic::normalize_learner_name;
    use chrono::Utc;

    fn make_template(source_hash: &str) -> crate::sf2::models::Sf2TemplateRecord {
        crate::sf2::models::Sf2TemplateRecord {
            id: String::new(),
            source_path: String::new(),
            source_hash: source_hash.to_string(),
            school_id: String::new(),
            school_name: String::new(),
            school_year: String::new(),
            report_month: String::new(),
            grade_level: String::new(),
            section: String::new(),
            adviser_name: String::new(),
            school_head_name: String::new(),
            layout_fingerprint: String::new(),
            active_class_id: String::new(),
            imported_at: 0,
        }
    }

    fn make_student(id: &str, name: &str, gender: Option<StudentGender>) -> Student {
        let uuid = uuid::Uuid::parse_str(id).unwrap_or_else(|_| uuid::Uuid::new_v4());
        Student {
            id: StudentId(uuid),
            name: name.to_string(),
            gender,
            card_serial: None,
            class_id: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn template_owns_roster_bundled_prefix() {
        let t = make_template("bundled-abc123");
        assert!(template_owns_roster(&t));
    }

    #[test]
    fn template_owns_roster_imported_hash() {
        let t = make_template("e3b0c44298fc1c149afbf4c8996fb924");
        assert!(!template_owns_roster(&t));
    }

    #[test]
    fn template_owns_roster_empty_hash() {
        let t = make_template("");
        assert!(!template_owns_roster(&t));
    }

    #[test]
    fn template_owns_roster_bundled_with_hyphen_and_hash() {
        let t = make_template("bundled-hash");
        assert!(template_owns_roster(&t));
    }

    #[test]
    fn template_owns_roster_bundled_with_hyphen_prefix() {
        let t = make_template("bundled");
        assert!(!template_owns_roster(&t));
    }

    #[test]
    fn template_owns_roster_no_bundled_prefix() {
        let t = make_template("not-bundled-hash");
        assert!(!template_owns_roster(&t));
    }

    #[test]
    fn template_roster_slots_total_count() {
        let slots = template_roster_slots();
        assert_eq!(slots.len(), 40);
    }

    #[test]
    fn template_roster_slots_first_male() {
        let slots = template_roster_slots();
        assert_eq!(slots[0].row_index, 8);
        assert_eq!(slots[0].gender_block, "MALE");
    }

    #[test]
    fn template_roster_slots_last_male() {
        let slots = template_roster_slots();
        assert_eq!(slots[20].row_index, 28);
        assert_eq!(slots[20].gender_block, "MALE");
    }

    #[test]
    fn template_roster_slots_first_female() {
        let slots = template_roster_slots();
        assert_eq!(slots[21].row_index, 30);
        assert_eq!(slots[21].gender_block, "FEMALE");
    }

    #[test]
    fn template_roster_slots_last_female() {
        let slots = template_roster_slots();
        assert_eq!(slots[39].row_index, 48);
        assert_eq!(slots[39].gender_block, "FEMALE");
    }

    #[test]
    fn template_roster_slots_all_male_before_female() {
        let slots = template_roster_slots();
        for i in 0..21 {
            assert_eq!(slots[i].gender_block, "MALE", "slot {i} should be MALE");
        }
        for i in 21..40 {
            assert_eq!(slots[i].gender_block, "FEMALE", "slot {i} should be FEMALE");
        }
    }

    #[test]
    fn unique_normalized_name_first_use() {
        let mut seen = HashSet::new();
        let result = unique_normalized_name(&mut seen, "Juan dela Cruz", "1");
        assert_eq!(result, normalize_learner_name("Juan dela Cruz"));
    }

    #[test]
    fn unique_normalized_name_duplicate_appends_suffix() {
        let mut seen = HashSet::new();
        let first = unique_normalized_name(&mut seen, "Maria Santos", "1");
        let second = unique_normalized_name(&mut seen, "Maria Santos", "2");
        assert_eq!(first, normalize_learner_name("Maria Santos"));
        assert_eq!(second, format!("{}#{}", normalize_learner_name("Maria Santos"), "2"));
    }

    #[test]
    fn unique_normalized_name_multiple_duplicates() {
        let mut seen = HashSet::new();
        let a = unique_normalized_name(&mut seen, "John Smith", "10");
        let b = unique_normalized_name(&mut seen, "John Smith", "20");
        let c = unique_normalized_name(&mut seen, "John Smith", "30");
        assert_eq!(a, normalize_learner_name("John Smith"));
        assert_eq!(b, format!("{}#{}", normalize_learner_name("John Smith"), "20"));
        assert_eq!(c, format!("{}#{}", normalize_learner_name("John Smith"), "30"));
    }

    #[test]
    fn unique_normalized_name_different_names_no_conflict() {
        let mut seen = HashSet::new();
        let a = unique_normalized_name(&mut seen, "Alice", "1");
        let b = unique_normalized_name(&mut seen, "Bob", "2");
        assert_eq!(a, normalize_learner_name("Alice"));
        assert_eq!(b, normalize_learner_name("Bob"));
    }

    #[test]
    fn unique_normalized_name_name_with_accents() {
        let mut seen = HashSet::new();
        let result = unique_normalized_name(&mut seen, "María José", "1");
        assert_eq!(result, normalize_learner_name("María José"));
    }

    #[test]
    fn reject_duplicate_roster_names_no_duplicates() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Juan", Some(StudentGender::Male)),
            make_student("00000000-0000-0000-0000-000000000002", "Maria", Some(StudentGender::Female)),
        ];
        assert!(reject_duplicate_roster_names(&students).is_ok());
    }

    #[test]
    fn reject_duplicate_roster_names_duplicate_found() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Juan dela Cruz", Some(StudentGender::Male)),
            make_student("00000000-0000-0000-0000-000000000002", "Juan Dela Cruz", Some(StudentGender::Male)),
        ];
        let result = reject_duplicate_roster_names(&students);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Duplicate"));
        assert!(err.contains("Juan"));
    }

    #[test]
    fn reject_duplicate_roster_names_multiple_duplicate_groups() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Alice", Some(StudentGender::Female)),
            make_student("00000000-0000-0000-0000-000000000002", "Alice", Some(StudentGender::Female)),
            make_student("00000000-0000-0000-0000-000000000003", "Bob", Some(StudentGender::Male)),
            make_student("00000000-0000-0000-0000-000000000004", "Bob", Some(StudentGender::Male)),
        ];
        let result = reject_duplicate_roster_names(&students);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Alice"));
        assert!(err.contains("Bob"));
    }

    #[test]
    fn reject_duplicate_roster_names_single_student() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Only One", Some(StudentGender::Male)),
        ];
        assert!(reject_duplicate_roster_names(&students).is_ok());
    }

    #[test]
    fn reject_duplicate_roster_names_empty_list() {
        let students: Vec<Student> = vec![];
        assert!(reject_duplicate_roster_names(&students).is_ok());
    }

    #[test]
    fn reject_duplicate_roster_names_same_normalized_different_case() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "JUAN", Some(StudentGender::Male)),
            make_student("00000000-0000-0000-0000-000000000002", "juan", Some(StudentGender::Male)),
        ];
        let result = reject_duplicate_roster_names(&students);
        assert!(result.is_err(), "same normalized name should be rejected");
    }

    #[test]
    fn template_roster_assignments_basic() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Juan", Some(StudentGender::Male)),
            make_student("00000000-0000-0000-0000-000000000002", "Maria", Some(StudentGender::Female)),
        ];
        let result = template_roster_assignments(&students);
        assert!(result.is_ok());
        let assignments = result.unwrap();
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0].slot.gender_block, "MALE");
        assert_eq!(assignments[1].slot.gender_block, "FEMALE");
    }

    #[test]
    fn template_roster_assignments_all_male() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Juan", Some(StudentGender::Male)),
            make_student("00000000-0000-0000-0000-000000000002", "Pedro", Some(StudentGender::Male)),
        ];
        let result = template_roster_assignments(&students);
        assert!(result.is_ok());
        let assignments = result.unwrap();
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0].slot.gender_block, "MALE");
        assert_eq!(assignments[1].slot.gender_block, "MALE");
    }

    #[test]
    fn template_roster_assignments_missing_gender_error() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Unknown", None),
        ];
        let result = template_roster_assignments(&students);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Set Male/Female"));
        assert!(err.contains("Unknown"));
    }

    #[test]
    fn template_roster_assignments_too_many_males_error() {
        let students: Vec<Student> = (0..22)
            .map(|i| make_student(
                &format!("00000000-0000-0000-0000-{:012}", i),
                &format!("Male Student {i}"),
                Some(StudentGender::Male),
            ))
            .collect();
        let result = template_roster_assignments(&students);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("21 male learner rows"));
        assert!(err.contains("22"));
    }

    #[test]
    fn template_roster_assignments_too_many_females_error() {
        let students: Vec<Student> = (0..20)
            .map(|i| make_student(
                &format!("00000000-0000-0000-0000-{:012}", i),
                &format!("Female Student {i}"),
                Some(StudentGender::Female),
            ))
            .collect();
        let result = template_roster_assignments(&students);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("19 female learner rows"));
        assert!(err.contains("20"));
    }

    #[test]
    fn template_roster_assignments_males_use_first_slots() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Juan", Some(StudentGender::Male)),
        ];
        let result = template_roster_assignments(&students).unwrap();
        assert_eq!(result[0].slot.row_index, 8);
        assert_eq!(result[0].student.name, "Juan");
    }

    #[test]
    fn template_roster_assignments_females_use_female_slots() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Maria", Some(StudentGender::Female)),
        ];
        let result = template_roster_assignments(&students).unwrap();
        assert_eq!(result[0].slot.row_index, 30);
        assert_eq!(result[0].student.name, "Maria");
    }

    #[test]
    fn template_roster_assignments_max_male_capacity() {
        let students: Vec<Student> = (0..21)
            .map(|i| make_student(
                &format!("00000000-0000-0000-0000-{:012}", i),
                &format!("Male {i}"),
                Some(StudentGender::Male),
            ))
            .collect();
        let result = template_roster_assignments(&students);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 21);
    }

    #[test]
    fn template_roster_assignments_max_female_capacity() {
        let students: Vec<Student> = (0..19)
            .map(|i| make_student(
                &format!("00000000-0000-0000-0000-{:012}", i),
                &format!("Female {i}"),
                Some(StudentGender::Female),
            ))
            .collect();
        let result = template_roster_assignments(&students);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 19);
    }

    #[test]
    fn template_roster_assignments_mixed_full_roster() {
        let mut students = Vec::new();
        for i in 0..15 {
            students.push(make_student(
                &format!("00000000-0000-0000-0000-{:012}", i),
                &format!("Male {i}"),
                Some(StudentGender::Male),
            ));
        }
        for i in 0..12 {
            students.push(make_student(
                &format!("00000000-0000-0000-0000-{:012}", 100 + i),
                &format!("Female {i}"),
                Some(StudentGender::Female),
            ));
        }
        let result = template_roster_assignments(&students).unwrap();
        assert_eq!(result.len(), 27);
        for i in 0..15 {
            assert_eq!(result[i].slot.gender_block, "MALE", "assignment {i} should be MALE");
        }
        for i in 15..27 {
            assert_eq!(result[i].slot.gender_block, "FEMALE", "assignment {i} should be FEMALE");
        }
        assert_eq!(result[0].slot.row_index, 8);
        assert_eq!(result[14].slot.row_index, 22);
        assert_eq!(result[15].slot.row_index, 30);
        assert_eq!(result[26].slot.row_index, 41);
    }

    #[test]
    fn student_mappings_from_roster_assignments_basic() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Juan", Some(StudentGender::Male)),
            make_student("00000000-0000-0000-0000-000000000002", "Maria", Some(StudentGender::Female)),
        ];
        let assignments = template_roster_assignments(&students).unwrap();
        let mappings = student_mappings_from_roster_assignments("template-1", &assignments);
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].template_id, "template-1");
        assert_eq!(mappings[0].workbook_name, "Juan");
        assert_eq!(mappings[0].row_index, 8);
        assert_eq!(mappings[0].gender_block.as_deref(), Some("MALE"));
        assert_eq!(mappings[1].template_id, "template-1");
        assert_eq!(mappings[1].workbook_name, "Maria");
        assert_eq!(mappings[1].row_index, 30);
        assert_eq!(mappings[1].gender_block.as_deref(), Some("FEMALE"));
    }

    #[test]
    fn student_mappings_from_roster_assignments_student_id_mapped() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Juan", Some(StudentGender::Male)),
        ];
        let assignments = template_roster_assignments(&students).unwrap();
        let mappings = student_mappings_from_roster_assignments("t1", &assignments);
        assert_eq!(mappings[0].student_id, "00000000-0000-0000-0000-000000000001");
    }

    #[test]
    fn student_mappings_from_roster_assignments_duplicate_normalized_names() {
        let students = vec![
            make_student("00000000-0000-0000-0000-000000000001", "Juan", Some(StudentGender::Male)),
            make_student("00000000-0000-0000-0000-000000000002", "JUAN", Some(StudentGender::Male)),
        ];
        let assignments = template_roster_assignments(&students).unwrap();
        let mappings = student_mappings_from_roster_assignments("t1", &assignments);
        assert_eq!(mappings.len(), 2);
        assert_ne!(mappings[0].normalized_name, mappings[1].normalized_name);
    }

    #[test]
    fn student_mappings_from_roster_assignments_empty() {
        let students: Vec<Student> = vec![];
        let assignments = template_roster_assignments(&students).unwrap();
        let mappings = student_mappings_from_roster_assignments("t1", &assignments);
        assert!(mappings.is_empty());
    }
}
