pub(crate) use crate::sf2::roster_parser::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{Student, StudentId, StudentGender};
    use crate::sf2::logic::normalize_learner_name;
    use crate::sf2::models::Sf2WorkbookAnalysis;
    use chrono::Utc;
    use std::collections::HashSet;

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
    fn template_roster_assignments_exceeds_male_capacity_uses_expanded_slots() {
        let students: Vec<Student> = (0..22)
            .map(|i| make_student(
                &format!("00000000-0000-0000-0000-{:012}", i),
                &format!("Male Student {i}"),
                Some(StudentGender::Male),
            ))
            .collect();
        let result = template_roster_assignments(&students);
        assert!(result.is_ok(), "should dynamically expand male slots: {:?}", result.err());
        let assignments = result.unwrap();
        assert_eq!(assignments.len(), 22);
        // 22 male students expand to rows 8-29 (instead of erroring)
        assert_eq!(assignments[0].slot.row_index, 8);
        assert_eq!(assignments[21].slot.row_index, 29);
    }

    #[test]
    fn template_roster_assignments_exceeds_female_capacity_uses_expanded_slots() {
        let students: Vec<Student> = (0..20)
            .map(|i| make_student(
                &format!("00000000-0000-0000-0000-{:012}", i),
                &format!("Female Student {i}"),
                Some(StudentGender::Female),
            ))
            .collect();
        let result = template_roster_assignments(&students);
        assert!(result.is_ok(), "should dynamically expand female slots: {:?}", result.err());
        let assignments = result.unwrap();
        assert_eq!(assignments.len(), 20);
        // 20 female students expand to rows 30-49 (instead of erroring)
        // Female section still starts at 30 since no male expansion needed
        assert_eq!(assignments[0].slot.row_index, 30);
        assert_eq!(assignments[19].slot.row_index, 49);
    }

    #[test]
    fn template_roster_assignments_exceeds_both_expands_rows() {
        let mut students: Vec<Student> = Vec::new();
        for i in 0..25 {
            students.push(make_student(
                &format!("00000000-0000-0000-0000-{:012}", i),
                &format!("Male {i}"),
                Some(StudentGender::Male),
            ));
        }
        for i in 0..22 {
            students.push(make_student(
                &format!("00000000-0000-0000-0000-{:012}", 100 + i),
                &format!("Female {i}"),
                Some(StudentGender::Female),
            ));
        }
        let result = template_roster_assignments(&students);
        assert!(result.is_ok(), "should dynamically expand both: {:?}", result.err());
        let assignments = result.unwrap();
        assert_eq!(assignments.len(), 47);
        // 25 males expand rows: 8..=32, then MALE TOTAL at 33 (was 29 + 4 extra)
        // Female start = 30 + 4 (extra male) = 34
        for i in 0..25 {
            assert_eq!(assignments[i].slot.gender_block, "MALE", "assignment {i} should be MALE");
        }
        for i in 25..47 {
            assert_eq!(assignments[i].slot.gender_block, "FEMALE", "assignment {i} should be FEMALE");
        }
        assert_eq!(assignments[0].slot.row_index, 8);
        assert_eq!(assignments[24].slot.row_index, 32); // last male row
        assert_eq!(assignments[25].slot.row_index, 34); // first female row (30 + 4 extra male)
        assert_eq!(assignments[46].slot.row_index, 55); // last female row (34 + 22 - 1 = 55)
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

    // ── clear_unused_learner_marks ────────────────────────────────────────────

    #[test]
    fn clear_unused_learner_marks_no_mapped_rows_clears_all_learner_slots() {
        let analysis = Sf2WorkbookAnalysis {
            file_format: 0,
            has_vb_project: false,
            school_id: String::new(),
            school_name: String::new(),
            school_year: String::new(),
            report_month: String::new(),
            grade_level: String::new(),
            section: String::new(),
            adviser_name: String::new(),
            school_head_name: String::new(),
            learners: Vec::new(),
            dates: Vec::new(),
            sheets: vec![
                crate::sf2::models::Sf2WorkbookSheet {
                    name: "JANUARY 2025".to_string(),
                    visible: -1,
                    used_range: String::new(),
                },
            ],
        };
        let mapped_rows: Vec<u32> = vec![];
        let marks = clear_unused_learner_marks(&analysis, &mapped_rows, None, None);
        // 21 male rows (8-28) + 19 female rows (30-48) = 40 learner slots
        // Each unused row clears 3 columns (A, B, C) = 120 marks
        // Note: rows 29 (MALE TOTAL) and 49 (FEMALE TOTAL) are NOT learner slots
        assert_eq!(marks.len(), 40 * 3);
        for mark in &marks {
            assert_eq!(mark.sheet_name, "JANUARY 2025");
            let col = mark
                .cell_address
                .trim_end_matches(|c: char| c.is_ascii_digit());
            assert!(
                ["A", "B", "C"].contains(&col),
                "column should be A, B, or C, got {col}"
            );
            assert!(mark.value.is_empty());
        }
        // Verify total rows are NOT cleared (they're not learner slots)
        assert!(!marks.iter().any(|m| m.cell_address == "A29"
            || m.cell_address == "B29"
            || m.cell_address == "C29"));
        assert!(!marks.iter().any(|m| m.cell_address == "A49"
            || m.cell_address == "B49"
            || m.cell_address == "C49"));
        // Verify all learner rows are present (male 8-28, female 30-48)
        let mut cleared_rows: Vec<u32> = marks
            .iter()
            .filter_map(|m| Some(m.cell_address.trim_start_matches(['A', 'B', 'C'])))
            .filter_map(|s| s.parse::<u32>().ok())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        cleared_rows.sort();
        assert_eq!(cleared_rows.len(), 40);
        assert_eq!(cleared_rows[0], 8);
        assert_eq!(cleared_rows[20], 28);
        assert_eq!(cleared_rows[21], 30);
        assert_eq!(cleared_rows[39], 48);
    }

    #[test]
    fn clear_unused_learner_marks_all_mapped_returns_empty() {
        let analysis = Sf2WorkbookAnalysis {
            file_format: 0,
            has_vb_project: false,
            school_id: String::new(),
            school_name: String::new(),
            school_year: String::new(),
            report_month: String::new(),
            grade_level: String::new(),
            section: String::new(),
            adviser_name: String::new(),
            school_head_name: String::new(),
            learners: Vec::new(),
            dates: Vec::new(),
            sheets: vec![
                crate::sf2::models::Sf2WorkbookSheet {
                    name: "JANUARY 2025".to_string(),
                    visible: -1,
                    used_range: String::new(),
                },
            ],
        };
        let all_rows: Vec<u32> = (8..=48).filter(|r| !matches!(r, 29 | 49)).collect();
        let marks = clear_unused_learner_marks(&analysis, &all_rows, None, None);
        assert!(marks.is_empty());
    }

    #[test]
    fn clear_unused_learner_marks_some_mapped_clears_only_unmapped() {
        let analysis = Sf2WorkbookAnalysis {
            file_format: 0,
            has_vb_project: false,
            school_id: String::new(),
            school_name: String::new(),
            school_year: String::new(),
            report_month: String::new(),
            grade_level: String::new(),
            section: String::new(),
            adviser_name: String::new(),
            school_head_name: String::new(),
            learners: Vec::new(),
            dates: Vec::new(),
            sheets: vec![
                crate::sf2::models::Sf2WorkbookSheet {
                    name: "JANUARY 2025".to_string(),
                    visible: -1,
                    used_range: String::new(),
                },
            ],
        };
        // Map 5 male students (rows 8-12) and 3 female students (rows 30-32)
        let mapped_rows = vec![8u32, 9, 10, 11, 12, 30, 31, 32];
        let marks = clear_unused_learner_marks(&analysis, &mapped_rows, None, None);
        // 40 total clearable slots - 8 mapped = 32 unused slots
        // Each unused slot clears 3 columns (A, B, C) = 96 marks
        assert_eq!(marks.len(), 32 * 3);
        // These mapped rows should NOT have clear marks in any column
        let mapped_set: std::collections::HashSet<u32> = mapped_rows.iter().copied().collect();
        let cleared_rows: std::collections::HashSet<u32> = marks
            .iter()
            .filter_map(|m| {
                m.cell_address
                    .trim_start_matches(['A', 'B', 'C'])
                    .parse::<u32>()
                    .ok()
            })
            .collect();
        for mapped in &mapped_set {
            assert!(
                !cleared_rows.contains(mapped),
                "mapped row {mapped} should not have any clear marks"
            );
        }
    }

    #[test]
    fn clear_unused_learner_marks_clears_columns_abc_for_unused_rows() {
        let analysis = Sf2WorkbookAnalysis {
            file_format: 0,
            has_vb_project: false,
            school_id: String::new(),
            school_name: String::new(),
            school_year: String::new(),
            report_month: String::new(),
            grade_level: String::new(),
            section: String::new(),
            adviser_name: String::new(),
            school_head_name: String::new(),
            learners: Vec::new(),
            dates: Vec::new(),
            sheets: vec![
                crate::sf2::models::Sf2WorkbookSheet {
                    name: "JANUARY 2025".to_string(),
                    visible: -1,
                    used_range: String::new(),
                },
            ],
        };
        // Map only the first 2 male rows (8, 9) and first 2 female rows (30, 31)
        // The remaining 36 learner slots should be cleared in columns A, B, and C
        let mapped_rows = vec![8u32, 9, 30, 31];
        let marks = clear_unused_learner_marks(&analysis, &mapped_rows, None, None);

        // 36 unused rows × 3 columns (A, B, C) = 108 marks
        assert_eq!(marks.len(), 36 * 3, "should clear A, B, C for all 36 unused rows");

        // Group marks by row to verify each unused row has A, B, C cleared
        let mut marks_by_row: std::collections::HashMap<u32, Vec<&str>> =
            std::collections::HashMap::new();
        for mark in &marks {
            let row_str = mark
                .cell_address
                .trim_start_matches(|c: char| c.is_ascii_alphabetic());
            if let Ok(row) = row_str.parse::<u32>() {
                let col = mark
                    .cell_address
                    .trim_end_matches(|c: char| c.is_ascii_digit());
                marks_by_row.entry(row).or_default().push(col);
            }
        }

        // Mapped rows should NOT have clear marks in any column
        for mapped in &[8u32, 9, 30, 31] {
            assert!(
                !marks_by_row.contains_key(mapped),
                "mapped row {mapped} should not have any clear marks"
            );
        }

        // Unused rows should have exactly 3 column letters (A, B, C) cleared
        for (&row, cols) in &marks_by_row {
            assert!(
                (8u32..=28).contains(&row) || (30u32..=48).contains(&row),
                "row {row} is not in the learner range"
            );
            let mut sorted_cols = cols.clone();
            sorted_cols.sort();
            assert_eq!(
                sorted_cols,
                vec!["A", "B", "C"],
                "unused row {row} should have A, B, C cleared, got {:?}",
                sorted_cols
            );
        }

        // All marks should be empty (clearing values)
        for mark in &marks {
            assert!(mark.value.is_empty(), "clear marks should have empty value");
        }
    }

    #[test]
    fn clear_unused_learner_marks_non_bundled_scenario_clears_unmapped_rows() {
        // Simulate the non-bundled import scenario:
        // - 15 male students mapped to rows 8-22
        // - 12 female students mapped to rows 30-41
        // - Male rows 23-28 (6 rows) and female rows 42-48 (7 rows) should be cleared
        // This mirrors what happens after sync_workbook_learner_mappings.
        let analysis = Sf2WorkbookAnalysis {
            file_format: 0,
            has_vb_project: false,
            school_id: String::new(),
            school_name: String::new(),
            school_year: String::new(),
            report_month: String::new(),
            grade_level: String::new(),
            section: String::new(),
            adviser_name: String::new(),
            school_head_name: String::new(),
            learners: Vec::new(),
            dates: Vec::new(),
            sheets: vec![
                crate::sf2::models::Sf2WorkbookSheet {
                    name: "JULY 2026".to_string(),
                    visible: -1,
                    used_range: String::new(),
                },
            ],
        };

        // 15 male + 12 female = 27 mapped rows
        let mut mapped_rows: Vec<u32> = (8..=22).collect(); // male rows 8-22
        mapped_rows.extend(30..=41); // female rows 30-41

        let marks = clear_unused_learner_marks(&analysis, &mapped_rows, None, None);

        // Total unused: (21 male - 15) + (19 female - 12) = 6 + 7 = 13 unused rows
        // Each unused row clears 3 columns (A, B, C) = 39 marks
        assert_eq!(marks.len(), 13 * 3, "13 unused rows × 3 columns = 39 marks");

        // Verify unused male rows (23-28) are cleared
        let cleared_rows: std::collections::HashSet<u32> = marks
            .iter()
            .filter_map(|m| {
                m.cell_address
                    .trim_start_matches(['A', 'B', 'C'])
                    .parse::<u32>()
                    .ok()
            })
            .collect();

        for row in 23u32..=28 {
            assert!(cleared_rows.contains(&row), "unused male row {row} should be cleared");
        }
        for row in 42u32..=48 {
            assert!(cleared_rows.contains(&row), "unused female row {row} should be cleared");
        }

        // Mapped rows should NOT be in the cleared set
        for row in &[8u32, 9, 10, 22, 30, 31, 41] {
            assert!(!cleared_rows.contains(row), "mapped row {row} should not be cleared");
        }

        // Each cleared row should have A, B, C marks
        for row in &cleared_rows {
            let row_marks: Vec<&str> = marks
                .iter()
                .filter(|m| m.cell_address.ends_with(&row.to_string()))
                .map(|m| m.cell_address.trim_end_matches(|c: char| c.is_ascii_digit()))
                .collect();
            assert_eq!(row_marks.len(), 3, "row {row} should have 3 column marks");
            let mut sorted = row_marks.clone();
            sorted.sort();
            assert_eq!(sorted, vec!["A", "B", "C"], "row {row} should clear A, B, C");
        }
    }

    #[test]
    fn clear_unused_learner_marks_hidden_sheets_skipped() {
        let analysis = Sf2WorkbookAnalysis {
            file_format: 0,
            has_vb_project: false,
            school_id: String::new(),
            school_name: String::new(),
            school_year: String::new(),
            report_month: String::new(),
            grade_level: String::new(),
            section: String::new(),
            adviser_name: String::new(),
            school_head_name: String::new(),
            learners: Vec::new(),
            dates: Vec::new(),
            sheets: vec![
                crate::sf2::models::Sf2WorkbookSheet {
                    name: "JANUARY 2025".to_string(),
                    visible: -1,
                    used_range: String::new(),
                },
                crate::sf2::models::Sf2WorkbookSheet {
                    name: "FEBRUARY 2025".to_string(),
                    visible: -1,
                    used_range: String::new(),
                },
                crate::sf2::models::Sf2WorkbookSheet {
                    name: "Hidden Sheet".to_string(),
                    visible: 0,
                    used_range: String::new(),
                },
            ],
        };
        let mapped_rows: Vec<u32> = vec![];
        let marks = clear_unused_learner_marks(&analysis, &mapped_rows, None, None);
        // 2 visible sheets × 40 rows each × 3 columns (A, B, C) = 240 marks
        assert_eq!(marks.len(), 240);
    }
}
