use super::*;
use crate::infrastructure::database::init_db;
use crate::sf2::month::{Sf2MonthStudentMapping, Sf2MonthTemplate, Sf2MonthTemplateRepo};

/// A migrated, empty database in a throwaway directory.
fn test_pool() -> DbPool {
    let dir = tempfile::tempdir().expect("temp dir for the test database");
    let pool = init_db(dir.path().join("attendance.db")).expect("migrate test database");
    std::mem::forget(dir);
    pool
}

fn seed_class_and_students(repo: &Sf2MonthStudentRepo, ids: &[&str]) {
    let conn = repo.pool.get().expect("connection");
    conn.execute(
        "INSERT OR IGNORE INTO classes (id, name, day_start, day_end, late_after, created_at)
         VALUES ('class-1', 'Grade 1', '07:00', '13:00', '07:30', 1)",
        [],
    )
    .expect("insert class");
    for id in ids {
        conn.execute(
            "INSERT OR IGNORE INTO students (id, name, class_id, created_at)
             VALUES (?1, ?2, 'class-1', 1)",
            rusqlite::params![id, *id],
        )
        .expect("insert student");
    }
}

fn seed_month(repo: &Sf2MonthTemplateRepo, id: &str, report_month: &str) {
    repo.upsert(&Sf2MonthTemplate {
        id: id.to_string(),
        active_class_id: "class-1".to_string(),
        school_year: "2026-2027".to_string(),
        report_month: report_month.to_string(),
        report_year: 2026,
        source_path: format!("C:/sf2-workbooks/SF2-{report_month}-2026.xls"),
        source_hash: format!("hash-{id}"),
        school_id: None,
        school_name: None,
        grade_level: None,
        section: None,
        adviser_name: None,
        school_head_name: None,
        first_school_day: 1,
        first_school_day_override: None,
        imported_at: 1_000,
        last_synced_at: None,
        workbook_x_count: 0,
        workbook_scanned_at: None,
    })
    .expect("insert the month row");
}

fn mapping(
    template_id: &str,
    student_id: &str,
    name: &str,
    row_index: u32,
    sf2_learner_id: Option<&str>,
) -> Sf2MonthStudentMapping {
    Sf2MonthStudentMapping {
        template_id: template_id.to_string(),
        student_id: student_id.to_string(),
        workbook_name: name.to_string(),
        normalized_name: normalize_learner_name(name),
        row_index,
        gender_block: Some("MALE".to_string()),
        sf2_learner_id: sf2_learner_id.map(str::to_string),
    }
}

fn workbook_learner(
    row_index: u32,
    name: &str,
    sf2_learner_id: Option<&str>,
) -> Sf2WorkbookLearner {
    Sf2WorkbookLearner {
        row_index,
        name: name.to_string(),
        gender_block: Some("MALE".to_string()),
        sf2_learner_id: sf2_learner_id.map(str::to_string),
    }
}

// ── the DepEd learner-ID cell ──────────────────────────────────────────────

#[test]
fn a_real_learner_id_in_column_two_is_kept() {
    // Column 2 of the DepEd SF2 form is the learner-ID slot. A workbook that
    // has a value there is the one case where an ID exists to keep.
    assert_eq!(
        deped_learner_id_from_cells("13672845021", "1"),
        Some("13672845021".to_string())
    );
    // Surrounding whitespace is not part of the identity.
    assert_eq!(
        deped_learner_id_from_cells("  13672845021 ", " 1 "),
        Some("13672845021".to_string())
    );
    // A school that uses letters keeps them.
    assert_eq!(
        deped_learner_id_from_cells("LRN-0001", "1"),
        Some("LRN-0001".to_string())
    );
}

#[test]
fn the_bundled_templates_merged_no_cell_is_not_an_identity() {
    // Verified against `TEMPLATE_AUTOMATED_SF2.xls`: the learner row is
    // `A8:B8` (item number) merged with `C8:E8` (name), so reading column 2
    // returns the item number. Storing that would give every month file the
    // same positional identity - the exact row-index fragility this column
    // exists to remove.
    for (learner_id_cell, item_number_cell) in [("1", "1"), ("12", "12"), (" 7 ", "7")] {
        assert_eq!(
            deped_learner_id_from_cells(learner_id_cell, item_number_cell),
            None,
            "cell {learner_id_cell:?} is the item number, not an ID"
        );
    }
}

#[test]
fn an_absent_or_implausible_id_cell_yields_nothing() {
    assert_eq!(deped_learner_id_from_cells("", "1"), None);
    assert_eq!(deped_learner_id_from_cells("   ", "1"), None);
    // A name that landed in the column is not an ID.
    assert_eq!(deped_learner_id_from_cells("DELA CRUZ, JUAN", "1"), None);
    assert_eq!(deped_learner_id_from_cells(&"9".repeat(64), "1"), None);
}

// ── the matcher the roster sync and the split will wire ────────────────────

#[test]
fn a_learner_id_match_wins_over_a_name_match() {
    // Two learners on record; the workbook names one of them and gives the
    // other's ID. The ID is the school's own record and outranks the spelling.
    let existing = vec![
        mapping(
            "month-september",
            "student-1",
            "DELA CRUZ, JUAN",
            8,
            Some("111"),
        ),
        mapping(
            "month-september",
            "student-2",
            "MARIA SANTOS",
            9,
            Some("222"),
        ),
    ];

    let matched =
        match_roster_learner(&existing, &workbook_learner(9, "MARIA SANTOS", Some("222")))
            .expect("the learner ID identifies her");

    assert_eq!(matched.student_id, "student-2");
    assert_eq!(matched.matched_by, Sf2LearnerMatchKind::LearnerId);
    assert_eq!(matched.row_index, 9);
    assert_eq!(matched.template_id, "month-september");
}

#[test]
fn a_renamed_learner_still_matches_by_id() {
    // Edge case E7: the name is spelled differently between two month files
    // and the row moved. The identity is the ID.
    let existing = vec![mapping(
        "month-september",
        "student-1",
        "DELA CRUZ, JUAN",
        8,
        Some("13672845021"),
    )];

    let matched = match_roster_learner(
        &existing,
        &workbook_learner(21, "JUAN DELA CRUZ", Some("13672845021")),
    )
    .expect("matched on the ID alone");

    assert_eq!(matched.student_id, "student-1");
    assert_eq!(matched.matched_by, Sf2LearnerMatchKind::LearnerId);
}

#[test]
fn without_an_id_the_name_is_the_identity() {
    let existing = vec![
        mapping("month-september", "student-1", "DELA CRUZ, JUAN", 8, None),
        mapping("month-september", "student-2", "MARIA SANTOS", 9, None),
    ];

    let matched = match_roster_learner(&existing, &workbook_learner(30, "MARIA SANTOS", None))
        .expect("matched on the normalized name");
    assert_eq!(matched.student_id, "student-2");
    assert_eq!(matched.matched_by, Sf2LearnerMatchKind::NormalizedName);

    // Name matching is normalized, so case and runs of whitespace are not part
    // of the identity.
    let normalized =
        match_roster_learner(&existing, &workbook_learner(31, "  maria    santos ", None))
            .expect("matched on the normalized name");
    assert_eq!(normalized.student_id, "student-2");
    assert_eq!(normalized.matched_by, Sf2LearnerMatchKind::NormalizedName);
}

#[test]
fn the_row_is_the_last_resort_and_says_so() {
    // A workbook with no IDs and a reshuffled roster can only fall back to the
    // row. That is exactly the fragility §6.3 removes, so the matcher reports
    // which rule it used and the caller can log it.
    let existing = vec![mapping(
        "month-september",
        "student-1",
        "DELA CRUZ, JUAN",
        8,
        None,
    )];

    let matched = match_roster_learner(&existing, &workbook_learner(8, "NEW NAME", None))
        .expect("fell back to the row");
    assert_eq!(matched.student_id, "student-1");
    assert_eq!(matched.matched_by, Sf2LearnerMatchKind::RowIndex);

    // A learner on a row nobody holds matches nobody.
    assert!(match_roster_learner(&existing, &workbook_learner(40, "NEW NAME", None)).is_none());
    assert!(match_roster_learner(&[], &workbook_learner(8, "ANY", Some("1"))).is_none());
}

#[test]
fn an_id_that_answers_to_nobody_falls_through_to_the_name() {
    // A workbook carrying an ID the database has never seen must not stop the
    // match: the same learner may be matched by name in a file that was never
    // given an ID.
    let existing = vec![mapping(
        "month-september",
        "student-1",
        "DELA CRUZ, JUAN",
        8,
        None,
    )];

    let matched = match_roster_learner(
        &existing,
        &workbook_learner(8, "DELA CRUZ, JUAN", Some("99999999999")),
    )
    .expect("the unknown ID falls through to the name");
    assert_eq!(matched.matched_by, Sf2LearnerMatchKind::NormalizedName);

    // A blank ID is not an ID at all.
    let blank = match_roster_learner(&existing, &workbook_learner(8, "OTHER", Some("   ")))
        .expect("a blank ID cannot match anybody");
    assert_eq!(blank.matched_by, Sf2LearnerMatchKind::RowIndex);
}

// ── the roster store ───────────────────────────────────────────────────────

#[test]
fn a_month_roster_round_trips_with_its_learner_id() {
    let repo = Sf2MonthStudentRepo::new(test_pool());
    seed_class_and_students(&repo, &["student-1", "student-2"]);
    let templates = Sf2MonthTemplateRepo::new(repo.pool.clone());
    seed_month(&templates, "month-september", "SEPTEMBER");

    repo.replace_for_template(
        "month-september",
        &[
            mapping(
                "month-september",
                "student-1",
                "DELA CRUZ, JUAN",
                8,
                Some("111"),
            ),
            mapping(
                "month-september",
                "student-2",
                "MARIA SANTOS",
                9,
                Some("222"),
            ),
        ],
    )
    .expect("seed the roster");

    let stored = repo
        .for_template("month-september")
        .expect("read the roster");
    assert_eq!(stored.len(), 2);
    assert_eq!(
        stored[0].row_index, 8,
        "the roster is in workbook row order"
    );
    assert_eq!(stored[0].sf2_learner_id.as_deref(), Some("111"));
    assert_eq!(stored[1].normalized_name, "MARIA SANTOS");

    assert_eq!(
        repo.for_learner_id("222").expect("find by learner ID")[0].student_id,
        "student-2"
    );
    assert!(repo
        .for_learner_id("does-not-exist")
        .expect("find an unknown learner ID")
        .is_empty());
    assert_eq!(
        repo.for_normalized_name("month-september", "DELA CRUZ,JUAN")
            .expect("find by name")
            .expect("the mapping is there")
            .row_index,
        8
    );
}

#[test]
fn an_empty_roster_is_rejected_instead_of_committed() {
    let repo = Sf2MonthStudentRepo::new(test_pool());
    seed_class_and_students(&repo, &["student-1"]);
    let templates = Sf2MonthTemplateRepo::new(repo.pool.clone());
    seed_month(&templates, "month-september", "SEPTEMBER");
    repo.replace_for_template(
        "month-september",
        &[mapping(
            "month-september",
            "student-1",
            "DELA CRUZ, JUAN",
            8,
            None,
        )],
    )
    .expect("seed the roster");

    let error = repo
        .replace_for_template("month-september", &[])
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
        repo.for_template("month-september").expect("read").len(),
        1,
        "the existing roster survives a rejected analysis"
    );
}

#[test]
fn replacing_one_months_roster_never_touches_another_month() {
    let repo = Sf2MonthStudentRepo::new(test_pool());
    seed_class_and_students(&repo, &["student-1", "student-2"]);
    let templates = Sf2MonthTemplateRepo::new(repo.pool.clone());
    seed_month(&templates, "month-september", "SEPTEMBER");
    seed_month(&templates, "month-october", "OCTOBER");
    repo.replace_for_template(
        "month-september",
        &[mapping(
            "month-september",
            "student-1",
            "DELA CRUZ, JUAN",
            8,
            None,
        )],
    )
    .expect("seed September");
    repo.replace_for_template(
        "month-october",
        &[mapping(
            "month-october",
            "student-1",
            "DELA CRUZ, JUAN",
            8,
            None,
        )],
    )
    .expect("seed October");

    repo.replace_for_template(
        "month-september",
        &[mapping(
            "month-september",
            "student-2",
            "MARIA SANTOS",
            8,
            None,
        )],
    )
    .expect("re-derive September");

    assert_eq!(
        repo.for_template("month-september").expect("read")[0].student_id,
        "student-2"
    );
    assert_eq!(
        repo.for_template("month-october").expect("read")[0].student_id,
        "student-1",
        "October keeps its own roster"
    );
}

#[test]
fn the_split_backfill_writes_learner_ids_onto_students() {
    let repo = Sf2MonthStudentRepo::new(test_pool());
    seed_class_and_students(&repo, &["student-1", "student-2"]);

    let written = repo
        .set_student_learner_ids(&[
            ("student-1".to_string(), "13672845021".to_string()),
            ("student-2".to_string(), "13672845022".to_string()),
        ])
        .expect("backfill both learners");
    assert_eq!(written, 2);

    let conn = repo.pool.get().expect("connection");
    let mut stored = conn
        .prepare("SELECT id, sf2_learner_id FROM students ORDER BY id")
        .expect("prepare");
    let rows = stored
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .expect("query");
    let ids = rows.collect::<rusqlite::Result<Vec<_>>>().expect("collect");
    assert_eq!(
        ids,
        vec![
            ("student-1".to_string(), Some("13672845021".to_string())),
            ("student-2".to_string(), Some("13672845022".to_string())),
        ]
    );
}

#[test]
fn a_learner_id_belonging_to_somebody_else_is_refused() {
    // Two mappings claiming one identity is a roster problem for the split to
    // report, not a reason to move one learner's ID onto another student.
    let repo = Sf2MonthStudentRepo::new(test_pool());
    seed_class_and_students(&repo, &["student-1", "student-2"]);

    repo.set_student_learner_ids(&[("student-1".to_string(), "111".to_string())])
        .expect("first learner takes the ID");

    let written = repo
        .set_student_learner_ids(&[("student-2".to_string(), "111".to_string())])
        .expect("the duplicate is refused, not an error");
    assert_eq!(written, 0, "the second claim is not written");

    let conn = repo.pool.get().expect("connection");
    let second: Option<String> = conn
        .query_row(
            "SELECT sf2_learner_id FROM students WHERE id = 'student-2'",
            [],
            |row| row.get(0),
        )
        .expect("read");
    assert_eq!(second, None);
}

#[test]
fn re_backfilling_the_same_id_is_idempotent() {
    let repo = Sf2MonthStudentRepo::new(test_pool());
    seed_class_and_students(&repo, &["student-1"]);
    let pairs = vec![("student-1".to_string(), "111".to_string())];

    assert_eq!(
        repo.set_student_learner_ids(&pairs)
            .expect("first backfill"),
        1
    );
    assert_eq!(
        repo.set_student_learner_ids(&pairs)
            .expect("second backfill"),
        1,
        "re-reading the same workbook changes nothing"
    );
    // An empty or blank ID is never written.
    assert_eq!(
        repo.set_student_learner_ids(&[("student-1".to_string(), "  ".to_string())])
            .expect("a blank ID is skipped"),
        0
    );
    assert_eq!(repo.set_student_learner_ids(&[]).expect("nothing to do"), 0);
}
