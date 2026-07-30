use super::*;
use crate::domain::models::{Student, StudentGender, StudentId};
use crate::sf2::models::Sf2WorkbookAnalysis;
use crate::sf2::roster::{
    bundled_template_total_rows, roster_expansion_needed, roster_name_marks,
    student_mappings_from_roster_assignments, template_roster_assignments,
};
use crate::sf2::sf2_metadata::{
    date_mappings_from_analysis, metadata_from_import_analysis,
};
use chrono::Utc;
use std::collections::HashSet;

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

fn make_minimal_analysis() -> Sf2WorkbookAnalysis {
    Sf2WorkbookAnalysis {
        file_format: -4143,
        has_vb_project: false,
        school_id: "123456".to_string(),
        school_name: "Test School".to_string(),
        school_year: "2025-2026".to_string(),
        report_month: "JULY".to_string(),
        grade_level: "Grade 1".to_string(),
        section: "Section A".to_string(),
        adviser_name: "Ms. Teacher".to_string(),
        school_head_name: "Mr. Principal".to_string(),
        learners: Vec::new(),
        dates: Vec::new(),
        sheets: vec![crate::sf2::models::Sf2WorkbookSheet {
            name: "JULY 2026".to_string(),
            visible: -1,
            used_range: "A1:AL50".to_string(),
        }],
    }
}

// ── Pre-batch computation tests ──────────────────────────────────────────
//
// These tests characterize the pure-Rust computations that will be moved
// into the batch_operations closure. They remain the same before and after
// the optimization, proving the refactoring preserves behavior.

#[test]
fn import_precomputes_roster_assignments_correctly() {
    // The import flow computes roster_assignments from DB students (pure Rust).
    // This must produce the same assignments whether done before or inside batch.
    let students = vec![
        make_student("00000000-0000-0000-0000-000000000001", "Juan", Some(StudentGender::Male)),
        make_student("00000000-0000-0000-0000-000000000002", "Maria", Some(StudentGender::Female)),
    ];
    let assignments = template_roster_assignments(&students).unwrap();
    assert_eq!(assignments.len(), 2);
    assert_eq!(assignments[0].student.name, "Juan");
    assert_eq!(assignments[0].slot.row_index, 8);
    assert_eq!(assignments[0].slot.gender_block, "MALE");
    assert_eq!(assignments[1].student.name, "Maria");
    assert_eq!(assignments[1].slot.row_index, 30);
    assert_eq!(assignments[1].slot.gender_block, "FEMALE");
}

#[test]
fn import_precomputes_gender_counts_and_total_rows() {
    // Used to compute male_count, female_count, total rows (all pre-batch)
    let students = vec![
        make_student("1", "Male1", Some(StudentGender::Male)),
        make_student("2", "Male2", Some(StudentGender::Male)),
        make_student("3", "Female1", Some(StudentGender::Female)),
    ];
    let male_count = students.iter().filter(|s| s.gender == Some(StudentGender::Male)).count();
    let female_count = students.iter().filter(|s| s.gender == Some(StudentGender::Female)).count();
    assert_eq!(male_count, 2);
    assert_eq!(female_count, 1);

    let (extra_male, extra_female) = roster_expansion_needed(male_count, female_count);
    assert_eq!(extra_male, 0, "2 males < 21 → no expansion");
    assert_eq!(extra_female, 0, "1 female < 19 → no expansion");

    let (male_total, female_total, combined_total) =
        bundled_template_total_rows(male_count, female_count);
    assert_eq!(male_total, 29);
    assert_eq!(female_total, 49);
    assert_eq!(combined_total, 50);
}

#[test]
fn import_precomputes_expanded_counts_correctly() {
    // When students exceed standard capacity, expansion must be computed pre-batch
    let male_count = 25;
    let female_count = 22;
    let (extra_male, extra_female) = roster_expansion_needed(male_count, female_count);
    assert_eq!(extra_male, 4);
    assert_eq!(extra_female, 3);

    let (male_total, female_total, combined_total) =
        bundled_template_total_rows(male_count, female_count);
    assert_eq!(male_total, 33, "8 + 25 = 33");
    assert_eq!(female_total, 30 + 4 + 22, "30 + extra_male(4) + 22 = 56");
    assert_eq!(combined_total, 57);
}

#[test]
fn import_computes_roster_name_marks_from_analysis() {
    // roster_name_marks is computed INSIDE the batch closure from analysis
    let analysis = make_minimal_analysis();
    let students = vec![
        make_student("1", "Juan", Some(StudentGender::Male)),
    ];
    let assignments = template_roster_assignments(&students).unwrap();
    let marks = roster_name_marks(&analysis, &assignments);

    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].sheet_name, "JULY 2026");
    assert_eq!(marks[0].cell_address, "C8");
    assert_eq!(marks[0].value, "Juan");
}

#[test]
fn import_metadata_from_analysis_has_configure_calendar() {
    // metadata_from_import_analysis sets configure_calendar=true for valid months.
    // This metadata is written inside the batch closure.
    let analysis = make_minimal_analysis();
    let metadata = metadata_from_import_analysis(&analysis).unwrap();
    assert!(metadata.configure_calendar, "imported workbook should configure calendar");
    assert_eq!(metadata.school_id, "123456");
    assert_eq!(metadata.school_name, "Test School");
    assert_eq!(metadata.school_year, "2025-2026");
    assert_eq!(metadata.report_month, "JULY");
    assert_eq!(metadata.grade_level, "Grade 1");
    assert_eq!(metadata.section, "Section A");
    assert_eq!(metadata.adviser_name, "Ms. Teacher");
    assert_eq!(metadata.school_head_name, "Mr. Principal");
}

#[test]
fn import_date_mappings_from_analysis_produces_correct_structure() {
    // date_mappings_from_analysis is computed inside the batch closure
    let mut analysis = make_minimal_analysis();
    analysis.dates = vec![
        crate::sf2::models::Sf2WorkbookDate {
            sheet_name: "JULY 2026".to_string(),
            date: "2026-07-01".to_string(),
            column_letter: "F".to_string(),
            column_index: 6,
        },
    ];
    let mappings = date_mappings_from_analysis("template-1", &analysis);
    assert_eq!(mappings.len(), 1);
    assert_eq!(mappings[0].template_id, "template-1");
    assert_eq!(mappings[0].sheet_name, "JULY 2026");
    assert_eq!(mappings[0].date, "2026-07-01");
    assert_eq!(mappings[0].column_letter, "F");
    assert_eq!(mappings[0].column_index, 6);
}

#[test]
fn import_student_mappings_from_roster_assignments_produces_correct_mappings() {
    // Student mappings are computed AFTER the batch (from roster_assignments).
    // Must remain correct after the refactoring.
    let students = vec![
        make_student("00000000-0000-0000-0000-000000000001", "Juan", Some(StudentGender::Male)),
    ];
    let assignments = template_roster_assignments(&students).unwrap();
    let mappings = student_mappings_from_roster_assignments("template-1", &assignments);
    assert_eq!(mappings.len(), 1);
    assert_eq!(mappings[0].template_id, "template-1");
    assert_eq!(mappings[0].student_id, "00000000-0000-0000-0000-000000000001");
    assert_eq!(mappings[0].workbook_name, "Juan");
    assert_eq!(mappings[0].row_index, 8);
    assert_eq!(mappings[0].gender_block.as_deref(), Some("MALE"));
}

#[test]
fn import_mapped_rows_and_occupied_set_match() {
    // mapped_rows (for clear_unused_learner_marks) and occupied_rows (for
    // hide_empty_learner_rows) must be consistent — computed from same assignments.
    let students = vec![
        make_student("1", "A", Some(StudentGender::Male)),
        make_student("2", "B", Some(StudentGender::Male)),
        make_student("3", "C", Some(StudentGender::Female)),
    ];
    let assignments = template_roster_assignments(&students).unwrap();
    let mapped_rows: Vec<u32> = assignments.iter().map(|a| a.slot.row_index).collect();
    let occupied: HashSet<u32> = assignments.iter().map(|a| a.slot.row_index).collect();

    assert_eq!(mapped_rows.len(), 3);
    assert_eq!(occupied.len(), 3);
    assert_eq!(mapped_rows, vec![8, 9, 30]);
    for &row in &mapped_rows {
        assert!(occupied.contains(&row), "occupied set must contain all mapped rows");
    }
}
