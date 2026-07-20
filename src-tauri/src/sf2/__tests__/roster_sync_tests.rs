use super::*;

// ── roster_sync_formula_marks ──────────────────────────────────────────
//
// Verifies that `roster_sync_formula_marks()` computes correct TOTAL Per Day
// formulas and Enrolment summary marks that a roster sync should write to the
// workbook.  Previously `sync_bundled_template_roster` computed `male_count` /
// `female_count` but never called `total_formula_marks` or `summary_formula_marks`
// to update the workbook's TOTAL Per Day rows (29, 49, 50) or enrolment summary
// (AR53, AS53, AT53), leaving stale counts after roster changes.

#[test]
fn roster_sync_formula_marks_computes_correct_marks_for_standard_roster() {
    // Standard bundled template: male rows 8-28, MALE TOTAL at 29
    //                            female rows 30-48, FEMALE TOTAL at 49
    //                            Combined TOTAL at 50
    let male_count = 15usize;
    let female_count = 10usize;
    let male_total_row = 29u32;
    let female_total_row = 49u32;
    let combined_total_row = 50u32;

    let date_mappings = vec![
        Sf2DateMappingRecord {
            template_id: "test".to_string(),
            sheet_name: "JUNE 2026".to_string(),
            date: "2026-06-01".to_string(),
            column_letter: "F".to_string(),
            column_index: 6,
        },
        Sf2DateMappingRecord {
            template_id: "test".to_string(),
            sheet_name: "JUNE 2026".to_string(),
            date: "2026-06-02".to_string(),
            column_letter: "G".to_string(),
            column_index: 7,
        },
    ];

    let (total_marks, summary_formula_marks, summary_static_marks) = roster_sync_formula_marks(
        male_count,
        female_count,
        male_total_row,
        female_total_row,
        combined_total_row,
        &date_mappings,
    );

    // ── TOTAL Per Day formulas ────────────────────────────────────────
    // 2 dates × 3 marks (male, female, combined) = 6 total formula marks
    assert_eq!(
        total_marks.len(),
        6,
        "should have 6 total formula marks (2 dates × 3 rows)"
    );

    // MALE TOTAL (F29): =15-COUNTIF(F8:F28,"X")
    let male_f = total_marks
        .iter()
        .find(|m| m.cell_address == "F29")
        .unwrap();
    assert_eq!(male_f.value, "=15-COUNTIF(F8:F28,\"X\")");
    assert_eq!(male_f.sheet_name, "JUNE 2026");

    // FEMALE TOTAL (F49): =10-COUNTIF(F30:F48,"X")
    let female_f = total_marks
        .iter()
        .find(|m| m.cell_address == "F49")
        .unwrap();
    assert_eq!(female_f.value, "=10-COUNTIF(F30:F48,\"X\")");
    assert_eq!(female_f.sheet_name, "JUNE 2026");

    // Combined TOTAL (F50): =F29+F49
    let combined_f = total_marks
        .iter()
        .find(|m| m.cell_address == "F50")
        .unwrap();
    assert_eq!(combined_f.value, "=F29+F49");
    assert_eq!(combined_f.sheet_name, "JUNE 2026");

    // ── Summary section (static enrolment at Row 53) ───────────────────
    // AR53=15 (male), AS53=10 (female), AT53=25 (total)
    let ar53 = summary_static_marks
        .iter()
        .find(|m| m.cell_address == "AR53")
        .unwrap();
    assert_eq!(ar53.value, "15");
    assert_eq!(ar53.sheet_name, "JUNE 2026");

    let as53 = summary_static_marks
        .iter()
        .find(|m| m.cell_address == "AS53")
        .unwrap();
    assert_eq!(as53.value, "10");
    assert_eq!(as53.sheet_name, "JUNE 2026");

    let at53 = summary_static_marks
        .iter()
        .find(|m| m.cell_address == "AT53")
        .unwrap();
    assert_eq!(at53.value, "25");
    assert_eq!(at53.sheet_name, "JUNE 2026");

    // Verify summary formula marks exist (12 per sheet: 4 rows × 3 cols)
    assert_eq!(
        summary_formula_marks.len(),
        12,
        "should have 12 summary formula marks (4 rows × 3 cols)"
    );

    // Row 63 ADA references correct total rows
    let ar63 = summary_formula_marks
        .iter()
        .find(|m| m.cell_address == "AR63")
        .unwrap();
    assert_eq!(
        ar63.value, "=IFERROR(AVERAGE(F29:AL29),0)",
        "male ADA should reference row 29"
    );

    let as63 = summary_formula_marks
        .iter()
        .find(|m| m.cell_address == "AS63")
        .unwrap();
    assert_eq!(
        as63.value, "=IFERROR(AVERAGE(F49:AL49),0)",
        "female ADA should reference row 49"
    );

    let at63 = summary_formula_marks
        .iter()
        .find(|m| m.cell_address == "AT63")
        .unwrap();
    assert_eq!(
        at63.value, "=IFERROR(AVERAGE(F50:AL50),0)",
        "combined ADA should reference row 50"
    );
}

#[test]
fn roster_sync_formula_marks_empty_date_mappings_returns_empty() {
    let date_mappings: Vec<Sf2DateMappingRecord> = vec![];

    let (total_marks, summary_formula_marks, summary_static_marks) =
        roster_sync_formula_marks(15, 10, 29, 49, 50, &date_mappings);

    assert!(
        total_marks.is_empty(),
        "no total marks when date_mappings is empty"
    );
    assert!(
        summary_formula_marks.is_empty(),
        "no summary formula marks when date_mappings is empty"
    );
    assert!(
        summary_static_marks.is_empty(),
        "no summary static marks when date_mappings is empty"
    );
}

#[test]
fn roster_sync_formula_marks_expanded_roster_uses_shifted_total_rows() {
    // Expanded roster: MALE TOTAL at 33, FEMALE TOTAL at 56, Combined at 57
    let male_count = 25usize;
    let female_count = 22usize;
    let male_total_row = 33u32;
    let female_total_row = 56u32;
    let combined_total_row = 57u32;

    let date_mappings = vec![Sf2DateMappingRecord {
        template_id: "test".to_string(),
        sheet_name: "JULY 2026".to_string(),
        date: "2026-07-15".to_string(),
        column_letter: "P".to_string(),
        column_index: 16,
    }];

    let (total_marks, _summary_formula, _summary_static) = roster_sync_formula_marks(
        male_count,
        female_count,
        male_total_row,
        female_total_row,
        combined_total_row,
        &date_mappings,
    );

    assert_eq!(
        total_marks.len(),
        3,
        "should have 3 total marks (1 date × 3 rows)"
    );

    // MALE TOTAL at P33 = =25-COUNTIF(P8:P32,"X")
    let male_mark = total_marks
        .iter()
        .find(|m| m.cell_address == "P33")
        .unwrap();
    assert_eq!(male_mark.value, "=25-COUNTIF(P8:P32,\"X\")");

    // FEMALE TOTAL at P56 = =22-COUNTIF(P34:P55,"X")
    let female_mark = total_marks
        .iter()
        .find(|m| m.cell_address == "P56")
        .unwrap();
    assert_eq!(female_mark.value, "=22-COUNTIF(P34:P55,\"X\")");
}
