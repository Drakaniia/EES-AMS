use super::*;

// ── date_mappings_are_current_for_report_month ──────────────────────

#[test]
fn current_month_all_mappings_returns_true() {
    let template = Sf2TemplateRecord {
        id: "test".to_string(),
        source_path: String::new(),
        source_hash: String::new(),
        school_id: String::new(),
        school_name: String::new(),
        school_year: "2026-2027".to_string(),
        report_month: "JULY".to_string(),
        grade_level: String::new(),
        section: String::new(),
        adviser_name: String::new(),
        school_head_name: String::new(),
        layout_fingerprint: String::new(),
        active_class_id: "class-1".to_string(),
        imported_at: 0,
        last_synced_at: None,
    };
    let mappings = vec![Sf2DateMappingRecord {
        template_id: "test".to_string(),
        sheet_name: "JULY 2026".to_string(),
        date: "2026-07-01".to_string(),
        column_letter: "F".to_string(),
        column_index: 6,
    }];
    assert!(
        date_mappings_are_current_for_report_month(&template, &mappings),
        "mappings for the current report month should return true"
    );
}

#[test]
fn current_month_with_other_month_mappings_returns_true() {
    // When we have mappings from multiple months cached in the DB,
    // having at least one mapping for the current month should return true.
    // This is the NEW expected behavior after caching per-month mappings.
    let template = Sf2TemplateRecord {
        id: "test".to_string(),
        source_path: String::new(),
        source_hash: String::new(),
        school_id: String::new(),
        school_name: String::new(),
        school_year: "2026-2027".to_string(),
        report_month: "JULY".to_string(),
        grade_level: String::new(),
        section: String::new(),
        adviser_name: String::new(),
        school_head_name: String::new(),
        layout_fingerprint: String::new(),
        active_class_id: "class-1".to_string(),
        imported_at: 0,
        last_synced_at: None,
    };
    let mappings = vec![
        // July mapping (current month)
        Sf2DateMappingRecord {
            template_id: "test".to_string(),
            sheet_name: "JULY 2026".to_string(),
            date: "2026-07-01".to_string(),
            column_letter: "F".to_string(),
            column_index: 6,
        },
        // December mapping (other month, cached from previous switch)
        Sf2DateMappingRecord {
            template_id: "test".to_string(),
            sheet_name: "DECEMBER 2026".to_string(),
            date: "2026-12-01".to_string(),
            column_letter: "F".to_string(),
            column_index: 6,
        },
    ];
    assert!(
        date_mappings_are_current_for_report_month(&template, &mappings),
        "having at least one mapping for the current month should return true, even if other months exist"
    );
}

#[test]
fn only_other_month_mappings_returns_false() {
    let template = Sf2TemplateRecord {
        id: "test".to_string(),
        source_path: String::new(),
        source_hash: String::new(),
        school_id: String::new(),
        school_name: String::new(),
        school_year: "2026-2027".to_string(),
        report_month: "JULY".to_string(),
        grade_level: String::new(),
        section: String::new(),
        adviser_name: String::new(),
        school_head_name: String::new(),
        layout_fingerprint: String::new(),
        active_class_id: "class-1".to_string(),
        imported_at: 0,
        last_synced_at: None,
    };
    let mappings = vec![Sf2DateMappingRecord {
        template_id: "test".to_string(),
        sheet_name: "DECEMBER 2026".to_string(),
        date: "2026-12-01".to_string(),
        column_letter: "F".to_string(),
        column_index: 6,
    }];
    assert!(
        !date_mappings_are_current_for_report_month(&template, &mappings),
        "only mappings for a different month should return false"
    );
}

#[test]
fn empty_mappings_returns_false() {
    let template = Sf2TemplateRecord {
        id: "test".to_string(),
        source_path: String::new(),
        source_hash: String::new(),
        school_id: String::new(),
        school_name: String::new(),
        school_year: "2026-2027".to_string(),
        report_month: "JULY".to_string(),
        grade_level: String::new(),
        section: String::new(),
        adviser_name: String::new(),
        school_head_name: String::new(),
        layout_fingerprint: String::new(),
        active_class_id: "class-1".to_string(),
        imported_at: 0,
        last_synced_at: None,
    };
    let mappings: Vec<Sf2DateMappingRecord> = vec![];
    assert!(
        !date_mappings_are_current_for_report_month(&template, &mappings),
        "empty mappings should return false"
    );
}

#[test]
fn invalid_report_month_returns_false() {
    let template = Sf2TemplateRecord {
        id: "test".to_string(),
        source_path: String::new(),
        source_hash: String::new(),
        school_id: String::new(),
        school_name: String::new(),
        school_year: "2026-2027".to_string(),
        report_month: "INVALID".to_string(),
        grade_level: String::new(),
        section: String::new(),
        adviser_name: String::new(),
        school_head_name: String::new(),
        layout_fingerprint: String::new(),
        active_class_id: "class-1".to_string(),
        imported_at: 0,
        last_synced_at: None,
    };
    let mappings = vec![Sf2DateMappingRecord {
        template_id: "test".to_string(),
        sheet_name: "JULY 2026".to_string(),
        date: "2026-07-01".to_string(),
        column_letter: "F".to_string(),
        column_index: 6,
    }];
    assert!(
        !date_mappings_are_current_for_report_month(&template, &mappings),
        "invalid report month should return false"
    );
}
