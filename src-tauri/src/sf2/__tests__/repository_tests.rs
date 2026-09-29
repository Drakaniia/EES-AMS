use super::*;
use crate::infrastructure::database::init_db;

/// A migrated, empty database in a throwaway directory.
///
/// The directory handle is leaked on purpose: the pool keeps the `.db` file
/// open for the whole test and Windows will not delete a file that is still
/// open.
fn test_pool() -> DbPool {
    let dir = tempfile::tempdir().expect("temp dir for the test database");
    let pool = init_db(dir.path().join("attendance.db")).expect("migrate test database");
    std::mem::forget(dir);
    pool
}

fn sample_template() -> Sf2TemplateRecord {
    Sf2TemplateRecord {
        id: "template-1".to_string(),
        source_path: "C:/templates/sf2.xls".to_string(),
        source_hash: "hash-1".to_string(),
        school_id: "S-1".to_string(),
        school_name: "Test School".to_string(),
        school_year: "2026-2027".to_string(),
        report_month: "2026-07".to_string(),
        grade_level: "3".to_string(),
        section: "A".to_string(),
        adviser_name: "Dela Cruz".to_string(),
        school_head_name: "Santos".to_string(),
        layout_fingerprint: "fingerprint-1".to_string(),
        active_class_id: "class-1".to_string(),
        imported_at: 1_000,
        last_synced_at: None,
    }
}

fn sample_student(template_id: &str) -> Sf2StudentMappingRecord {
    Sf2StudentMappingRecord {
        template_id: template_id.to_string(),
        student_id: "student-1".to_string(),
        workbook_name: "DELA CRUZ, JUAN".to_string(),
        normalized_name: "dela cruz juan".to_string(),
        row_index: 8,
        gender_block: Some("MALE".to_string()),
    }
}

fn sample_date(template_id: &str) -> Sf2DateMappingRecord {
    Sf2DateMappingRecord {
        template_id: template_id.to_string(),
        sheet_name: "JULY 2026".to_string(),
        date: "2026-07-01".to_string(),
        column_letter: "F".to_string(),
        column_index: 6,
    }
}

/// Create a template that already holds one student mapping and one date
/// mapping - the "good" state a degenerate analysis must not be able to erase.
fn seed_template(repo: &Sf2Repository) -> Sf2TemplateRecord {
    {
        let conn = repo.pool.get().expect("connection");
        conn.execute(
            "INSERT INTO students (id, name, class_id, created_at) VALUES (?1, ?2, ?3, ?4)",
            params!["student-1", "Juan Dela Cruz", "class-1", 1_i64],
        )
        .expect("insert student");
    }

    let template = sample_template();
    repo.upsert_template_with_mappings(
        &template,
        &[sample_student("template-1")],
        &[sample_date("template-1")],
    )
    .expect("seed template with one student and one date");
    template
}

#[test]
fn update_rejects_empty_date_analysis_and_keeps_existing_mappings() {
    let pool = test_pool();
    let repo = Sf2Repository::new(pool);
    let template = seed_template(&repo);

    // The Excel analysis came back with no dates (the month sheet was not
    // visible). Committing that result would delete every date mapping for the
    // template, which is what makes the reports grid go blank and the next
    // workbook sync a total clear with nothing to write back.
    let error = repo
        .update_template_with_mappings(&template, &[sample_student("template-1")], &[])
        .expect_err("an empty date analysis must be rejected");

    assert!(
        matches!(error, AppError::InvalidInput(_)),
        "expected InvalidInput, got {error:?}"
    );
    assert_eq!(
        error.to_string(),
        "invalid input: The SF2 workbook produced no calendar dates. The existing mappings were left untouched."
    );

    assert_eq!(
        repo.date_mappings_for_template(&template.id)
            .expect("read date mappings")
            .len(),
        1,
        "the existing date mappings must survive a rejected analysis"
    );
    assert_eq!(
        repo.student_mappings_for_template(&template.id)
            .expect("read student mappings")
            .len(),
        1
    );
}

#[test]
fn update_rejects_empty_roster_and_keeps_existing_mappings() {
    let pool = test_pool();
    let repo = Sf2Repository::new(pool);
    let template = seed_template(&repo);

    let error = repo
        .update_template_with_mappings(&template, &[], &[sample_date("template-1")])
        .expect_err("an empty roster must be rejected");

    assert!(
        matches!(error, AppError::InvalidInput(_)),
        "expected InvalidInput, got {error:?}"
    );
    assert_eq!(
        error.to_string(),
        "invalid input: The SF2 workbook produced no learners. The existing mappings were left untouched."
    );

    assert_eq!(
        repo.student_mappings_for_template(&template.id)
            .expect("read student mappings")
            .len(),
        1,
        "the existing student mappings must survive a rejected analysis"
    );
    assert_eq!(
        repo.date_mappings_for_template(&template.id)
            .expect("read date mappings")
            .len(),
        1
    );
}

#[test]
fn update_replaces_mappings_when_the_analysis_is_usable() {
    let pool = test_pool();
    let repo = Sf2Repository::new(pool);
    let template = seed_template(&repo);

    let second_date = Sf2DateMappingRecord {
        template_id: "template-1".to_string(),
        sheet_name: "JULY 2026".to_string(),
        date: "2026-07-02".to_string(),
        column_letter: "G".to_string(),
        column_index: 7,
    };

    repo.update_template_with_mappings(
        &template,
        &[sample_student("template-1")],
        &[sample_date("template-1"), second_date],
    )
    .expect("a non-empty analysis is still applied");

    let dates = repo
        .date_mappings_for_template(&template.id)
        .expect("read date mappings");
    assert_eq!(dates.len(), 2);
    assert!(dates.iter().any(|mapping| mapping.date == "2026-07-02"));
}
