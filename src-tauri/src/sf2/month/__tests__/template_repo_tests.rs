use super::*;
use crate::infrastructure::database::init_db;
use crate::sf2::month::first_school_day::report_year_for_school_month;
use crate::sf2::month::{
    Sf2MonthDateMapping, Sf2MonthDateRepo, Sf2MonthStudentMapping, Sf2MonthStudentRepo,
    FIRST_SCHOOL_DAY_UNDETERMINED, SF2_SCHOOL_YEAR_MONTHS,
};

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

/// The single class the app has (spec D15: the table stays, the app does not
/// pretend there are several).
fn seed_class(repo: &Sf2MonthTemplateRepo) {
    let conn = repo.pool.get().expect("connection");
    conn.execute(
        "INSERT INTO classes (id, name, day_start, day_end, late_after, created_at)
         VALUES ('class-1', 'Grade 1 - EsPIRITU', '07:00', '13:00', '07:30', 1)",
        [],
    )
    .expect("insert class");
}

fn seed_students(repo: &Sf2MonthTemplateRepo, ids: &[&str]) {
    let conn = repo.pool.get().expect("connection");
    for id in ids {
        conn.execute(
            "INSERT INTO students (id, name, class_id, created_at) VALUES (?1, ?2, 'class-1', 1)",
            rusqlite::params![id, *id],
        )
        .expect("insert student");
    }
}

fn month_template(id: &str, report_month: &str, report_year: i32) -> Sf2MonthTemplate {
    Sf2MonthTemplate {
        id: id.to_string(),
        active_class_id: "class-1".to_string(),
        school_year: "2026-2027".to_string(),
        report_month: report_month.to_string(),
        report_year,
        source_path: format!("C:/sf2-workbooks/SF2-{report_month}-{report_year}.xls"),
        source_hash: format!("hash-{id}"),
        school_id: Some("132839".to_string()),
        school_name: Some("ESPIRITU ELEMENTARY SCHOOL".to_string()),
        grade_level: Some("1".to_string()),
        section: Some("A".to_string()),
        adviser_name: Some("DELA CRUZ, JUAN".to_string()),
        school_head_name: Some("SANTOS, MARIA".to_string()),
        first_school_day: 1,
        first_school_day_override: None,
        imported_at: 1_000,
        last_synced_at: None,
        workbook_x_count: 0,
        workbook_scanned_at: None,
    }
}

#[test]
fn a_month_is_a_row_that_can_be_looked_up_by_class_year_and_month() {
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);

    let september = month_template("month-september", "SEPTEMBER", 2026);
    repo.upsert(&september).expect("insert the September row");
    repo.upsert(&month_template("month-october", "OCTOBER", 2026))
        .expect("insert the October row");

    let found = repo
        .find("class-1", "2026-2027", "SEPTEMBER")
        .expect("look the month up")
        .expect("the row is there");
    assert_eq!(found.id, "month-september");
    assert_eq!(found.report_year, 2026);
    assert_eq!(found.source_path, september.source_path);
    assert_eq!(
        found.school_name.as_deref(),
        Some("ESPIRITU ELEMENTARY SCHOOL")
    );
    assert!(found.is_first_school_day_known());
    assert!(!found.is_first_school_day_overridden());
    assert!(!found.is_workbook_measured());

    // A month that was never stored is `None`, not a defaulted row.
    assert!(repo
        .find("class-1", "2026-2027", "APRIL")
        .expect("look up an absent month")
        .is_none());
    assert!(repo
        .find("class-1", "2025-2026", "SEPTEMBER")
        .expect("a different school year is a different row")
        .is_none());
}

#[test]
fn one_class_and_school_year_holds_twelve_rows_read_in_school_year_order() {
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);

    for month in SF2_SCHOOL_YEAR_MONTHS {
        let report_year = report_year_for_school_month("2026-2027", month, 1999);
        let name = crate::sf2::calendar::sf2_month_name(month);
        repo.upsert(&month_template(
            &format!("month-{month}"),
            name,
            report_year,
        ))
        .expect("insert a month row");
    }

    let months = repo
        .list_for_school_year("class-1", "2026-2027")
        .expect("list the school year");

    assert_eq!(months.len(), 12, "one row per month of the school year");
    // Ordered the way the school year runs, not alphabetically: an alphabetical
    // month order would read APRIL first and look like a bug to the teacher.
    let ordered = months
        .iter()
        .map(|template| (template.report_month.clone(), template.report_year))
        .collect::<Vec<_>>();
    assert_eq!(
        ordered,
        vec![
            ("SEPTEMBER".to_string(), 2026),
            ("OCTOBER".to_string(), 2026),
            ("NOVEMBER".to_string(), 2026),
            ("DECEMBER".to_string(), 2026),
            ("JANUARY".to_string(), 2027),
            ("FEBRUARY".to_string(), 2027),
            ("MARCH".to_string(), 2027),
            ("APRIL".to_string(), 2027),
            ("MAY".to_string(), 2027),
            ("JUNE".to_string(), 2027),
            ("JULY".to_string(), 2027),
            ("AUGUST".to_string(), 2027),
        ]
    );
}

#[test]
fn a_second_row_for_the_same_class_year_and_month_is_an_update_not_a_duplicate() {
    // The UNIQUE constraint is what makes two selected months impossible by
    // construction (E15).
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);
    repo.upsert(&month_template("month-1", "SEPTEMBER", 2026))
        .expect("first insert");

    let mut second = month_template("month-2", "SEPTEMBER", 2026);
    second.source_path = "C:/sf2-workbooks/SF2-SEPTEMBER-2026.xls".to_string();
    second.first_school_day = 3;
    repo.upsert(&second)
        .expect("second insert collides and updates");

    let months = repo
        .list_for_school_year("class-1", "2026-2027")
        .expect("list the school year");
    assert_eq!(months.len(), 1, "one row per (class, school year, month)");
    assert_eq!(
        months[0].id, "month-1",
        "the existing row keeps its identity"
    );
}

#[test]
fn an_undated_month_stores_the_undetermined_sentinel_rather_than_a_guess() {
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);

    let mut template = month_template("month-may", "MAY", 2027);
    template.first_school_day = FIRST_SCHOOL_DAY_UNDETERMINED;
    repo.upsert(&template).expect("insert an undated month");

    let stored = repo
        .find_by_id("month-may")
        .expect("read the row")
        .expect("the row is there");
    assert!(!stored.is_first_school_day_known());
    assert_eq!(stored.first_school_day, 0);
}

#[test]
fn a_derived_first_school_day_never_overwrites_an_override() {
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);
    repo.upsert(&month_template("month-1", "SEPTEMBER", 2026))
        .expect("insert the month");

    // A re-derivation writes while the value is derived.
    assert!(repo
        .derive_first_school_day("month-1", 2)
        .expect("derive the first school day"));
    assert_eq!(
        repo.find_by_id("month-1")
            .expect("read")
            .expect("row")
            .first_school_day,
        2
    );

    // The user overrides it.
    assert!(repo
        .override_first_school_day("month-1", 9)
        .expect("override the first school day"));
    let overridden = repo.find_by_id("month-1").expect("read").expect("row");
    assert_eq!(overridden.first_school_day, 9);
    assert_eq!(overridden.first_school_day_override, Some(9));
    assert!(overridden.is_first_school_day_overridden());

    // Re-derivation is refused, and reports the refusal instead of failing.
    assert!(
        !repo
            .derive_first_school_day("month-1", 2)
            .expect("re-deriving an overridden month is not an error"),
        "an override must survive re-derivation"
    );
    assert_eq!(
        repo.find_by_id("month-1")
            .expect("read")
            .expect("row")
            .first_school_day,
        9
    );

    // Dropping the override puts the derived value back.
    assert!(repo
        .clear_first_school_day_override("month-1", 2)
        .expect("clear the override"));
    let cleared = repo.find_by_id("month-1").expect("read").expect("row");
    assert_eq!(cleared.first_school_day, 2);
    assert_eq!(cleared.first_school_day_override, None);
    assert!(repo
        .derive_first_school_day("month-1", 3)
        .expect("derivable again once the override is gone"));
}

#[test]
fn re_deriving_refuses_to_write_the_undetermined_sentinel_over_a_real_day() {
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);
    repo.upsert(&month_template("month-1", "SEPTEMBER", 2026))
        .expect("insert the month");
    repo.derive_first_school_day("month-1", 4).expect("derive");

    assert!(!repo
        .derive_first_school_day("month-1", FIRST_SCHOOL_DAY_UNDETERMINED)
        .expect("writing the sentinel is refused"));
    assert_eq!(
        repo.find_by_id("month-1")
            .expect("read")
            .expect("row")
            .first_school_day,
        4
    );
}

#[test]
fn a_missing_month_row_is_reported_as_not_written_and_never_created() {
    // Every write on these rows answers "did it write?" with a bool. A month
    // that is not stored is `false`, not a row conjured into existence - and
    // not an error either, because re-deriving a month is a routine operation
    // that runs before the month has been created.
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    for call in [
        repo.override_first_school_day("nope", 1).expect("no error"),
        repo.derive_first_school_day("nope", 1).expect("no error"),
        repo.clear_first_school_day_override("nope", 1)
            .expect("no error"),
        repo.set_last_synced_at("nope", Some(1)).expect("no error"),
        repo.record_workbook_x_count("nope", 1, 1)
            .expect("no error"),
    ] {
        assert!(!call, "a missing month row must not report a write");
    }
    assert!(!repo
        .delete("nope")
        .expect("deleting nothing is not an error"));
    assert!(repo.find_by_id("nope").expect("read").is_none());

    // Refreshing metadata is the one write that has to fail loudly, because the
    // caller passed a row it believes exists.
    let mut template = month_template("nope", "SEPTEMBER", 2026);
    template.id = "nope".to_string();
    let error = repo
        .update(&template)
        .expect_err("updating a month that is not stored must fail");
    assert!(
        matches!(error, AppError::InvalidInput(_)),
        "expected InvalidInput, got {error:?}"
    );
}

#[test]
fn the_measured_x_count_is_only_meaningful_with_a_scan_timestamp() {
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);
    repo.upsert(&month_template("month-1", "SEPTEMBER", 2026))
        .expect("insert the month");

    let before = repo.find_by_id("month-1").expect("read").expect("row");
    assert!(!before.is_workbook_measured());
    assert_eq!(before.workbook_x_count, 0);

    repo.record_workbook_x_count("month-1", 12, 1_700_000_000)
        .expect("record the measurement");
    let measured = repo.find_by_id("month-1").expect("read").expect("row");
    assert!(measured.is_workbook_measured());
    assert_eq!(measured.workbook_x_count, 12);
    assert_eq!(measured.workbook_scanned_at, Some(1_700_000_000));
}

#[test]
fn updating_metadata_leaves_the_first_school_day_and_the_measurement_alone() {
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);
    let mut template = month_template("month-1", "SEPTEMBER", 2026);
    repo.upsert(&template).expect("insert the month");
    repo.override_first_school_day("month-1", 7)
        .expect("override");
    repo.record_workbook_x_count("month-1", 5, 1_700_000_000)
        .expect("measure");

    template.source_path = "C:/elsewhere/SF2-SEPTEMBER-2026.xls".to_string();
    template.school_name = Some("NEW SCHOOL NAME".to_string());
    template.first_school_day = 1;
    template.first_school_day_override = None;
    template.workbook_x_count = 0;
    template.workbook_scanned_at = None;
    repo.update(&template).expect("update the metadata");

    let stored = repo.find_by_id("month-1").expect("read").expect("row");
    assert_eq!(stored.source_path, "C:/elsewhere/SF2-SEPTEMBER-2026.xls");
    assert_eq!(stored.school_name.as_deref(), Some("NEW SCHOOL NAME"));
    assert_eq!(stored.first_school_day, 7, "the override is not metadata");
    assert_eq!(stored.first_school_day_override, Some(7));
    assert_eq!(
        stored.workbook_x_count, 5,
        "the measurement is not metadata"
    );
    assert_eq!(stored.workbook_scanned_at, Some(1_700_000_000));
}

#[test]
fn upserting_an_existing_month_does_not_re_date_it() {
    // A retried split rewrites the file and re-inserts the row. That must not
    // quietly move a month's first attendance day back to a derived value.
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);
    repo.upsert(&month_template("month-1", "SEPTEMBER", 2026))
        .expect("insert the month");
    repo.override_first_school_day("month-1", 9)
        .expect("override");

    let mut fresh = month_template("month-2", "SEPTEMBER", 2026);
    fresh.first_school_day = 1;
    repo.upsert(&fresh)
        .expect("re-insert as part of a retried split");

    let stored = repo
        .find("class-1", "2026-2027", "SEPTEMBER")
        .expect("read")
        .expect("row");
    assert_eq!(stored.first_school_day, 9);
    assert_eq!(stored.first_school_day_override, Some(9));
    assert_eq!(stored.id, "month-1");
}

#[test]
fn a_month_row_for_a_class_that_does_not_exist_is_refused_by_the_database() {
    // `sf2_month_templates.active_class_id` is a foreign key: a month row with
    // no class behind it would never be reachable from the app.
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    let orphan = month_template("month-1", "SEPTEMBER", 2026);
    assert!(
        repo.upsert(&orphan).is_err(),
        "a month row must belong to a real class"
    );
}

#[test]
fn deleting_a_month_row_cascades_to_its_mappings_but_not_to_the_file() {
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);
    seed_students(&repo, &["student-1", "student-2"]);
    repo.upsert(&month_template("month-1", "SEPTEMBER", 2026))
        .expect("insert the month");
    repo.upsert(&month_template("month-2", "OCTOBER", 2026))
        .expect("insert a second month");

    let dates = Sf2MonthDateRepo::new(repo.pool.clone());
    let students = Sf2MonthStudentRepo::new(repo.pool.clone());
    for template_id in ["month-1", "month-2"] {
        dates
            .replace_for_template(
                template_id,
                &[Sf2MonthDateMapping {
                    template_id: template_id.to_string(),
                    date: "2026-09-01".to_string(),
                    column_letter: "F".to_string(),
                    column_index: 6,
                    sheet_name: Some("SEPTEMBER 2026".to_string()),
                }],
            )
            .expect("seed the grid");
        students
            .replace_for_template(
                template_id,
                &[Sf2MonthStudentMapping {
                    template_id: template_id.to_string(),
                    student_id: "student-1".to_string(),
                    workbook_name: "DELA CRUZ, JUAN".to_string(),
                    normalized_name: "dela cruz juan".to_string(),
                    row_index: 8,
                    gender_block: Some("MALE".to_string()),
                    sf2_learner_id: None,
                }],
            )
            .expect("seed the roster");
    }

    assert!(repo.delete("month-1").expect("delete the month"));
    assert!(
        dates.for_template("month-1").expect("read").is_empty(),
        "the deleted month's mappings go with it"
    );
    assert!(
        students.for_template("month-1").expect("read").is_empty(),
        "including the roster"
    );
    assert_eq!(
        dates.for_template("month-2").expect("read").len(),
        1,
        "another month's mappings are untouched"
    );
    assert_eq!(
        students.for_template("month-2").expect("read").len(),
        1,
        "including its roster"
    );
}

// -- the label has to be findable, whichever way it was written ----------
//
// The real defect, as a test. `2026 - 2027` and `2026-2027` are the same school
// year; the month tables match the label by exact equality and the "latest
// school year" query GLOBs for the unspaced form. Before this, a row stored with
// spaces was unreachable by every per-month read, and nothing reported it.

#[test]
fn a_row_written_with_spaces_around_the_dash_is_still_found() {
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);

    let mut spaced = month_template("month-september", "SEPTEMBER", 2026);
    spaced.school_year = "2026 - 2027".to_string();
    repo.upsert(&spaced)
        .expect("insert a September row stored the way the real install has it");

    // Whatever the row was written as, a lookup in either spelling finds it.
    for asked in ["2026-2027", "2026 - 2027"] {
        let found = repo
            .find("class-1", asked, "SEPTEMBER")
            .expect("look the month up")
            .unwrap_or_else(|| panic!("{asked:?} must find the September row"));
        assert_eq!(found.id, "month-september", "asked with {asked:?}");
        assert_eq!(
            found.school_year, "2026-2027",
            "the row must come back in the one canonical form"
        );
    }
}

#[test]
fn a_row_written_with_spaces_is_listed_by_the_canonical_school_year() {
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);

    for (index, month) in ["SEPTEMBER", "OCTOBER"].iter().enumerate() {
        let mut spaced = month_template(&format!("month-{index}"), month, 2026);
        spaced.school_year = "2026 - 2027".to_string();
        repo.upsert(&spaced).expect("insert the spaced row");
    }

    let months = repo
        .list_for_school_year("class-1", "2026-2027")
        .expect("list the school year");

    assert_eq!(
        months.len(),
        2,
        "a month switch asks for '2026-2027' and must see the whole school year, not none of it"
    );
    assert!(months.iter().all(|month| month.school_year == "2026-2027"));
}

#[test]
fn a_row_is_never_stored_with_an_unnormalised_label() {
    // The conflict target is (class, school year, month). Storing `2026 - 2027`
    // beside an existing `2026-2027` would insert a *second* row for the same
    // month and the unique index would not catch it, because the strings differ.
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);

    let mut spaced = month_template("month-september", "SEPTEMBER", 2026);
    spaced.school_year = "2026 - 2027".to_string();
    repo.upsert(&spaced).expect("insert the spaced row");

    let conn = repo.pool.get().expect("connection");
    let stored: String = conn
        .query_row(
            "SELECT school_year FROM sf2_month_templates WHERE id = 'month-september'",
            [],
            |row| row.get(0),
        )
        .expect("read the stored label back");

    assert_eq!(
        stored, "2026-2027",
        "the label on disk is the one every equality lookup and GLOB is written against"
    );
}

#[test]
fn two_spellings_of_one_school_year_upsert_to_one_row() {
    // The same month, written twice, in two spellings. Two rows would be a
    // duplicated month in the reports sidebar and a doubled clear scope.
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);

    let canonical = month_template("month-september", "SEPTEMBER", 2026);
    repo.upsert(&canonical).expect("insert the canonical row");

    let mut spaced = month_template("month-september", "SEPTEMBER", 2026);
    spaced.school_year = "2026 - 2027".to_string();
    spaced.source_hash = "hash-refreshed".to_string();
    repo.upsert(&spaced)
        .expect("refresh it in the other spelling");

    let conn = repo.pool.get().expect("connection");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sf2_month_templates
             WHERE active_class_id = 'class-1' AND report_month = 'SEPTEMBER'",
            [],
            |row| row.get(0),
        )
        .expect("count the rows");

    assert_eq!(
        count, 1,
        "one month is one row, however its label is spelled"
    );
    let months = repo
        .list_for_school_year("class-1", "2026-2027")
        .expect("list the school year");
    assert_eq!(months.len(), 1);
    assert_eq!(
        months[0].source_hash, "hash-refreshed",
        "the refresh took effect"
    );
}

#[test]
fn a_school_year_the_user_has_not_finished_typing_is_not_rejected() {
    // Normalisation must not become validation: the label round-trips even
    // when it is not a school year, so nothing the user typed is ever lost.
    let repo = Sf2MonthTemplateRepo::new(test_pool());
    seed_class(&repo);

    let mut partial = month_template("month-september", "SEPTEMBER", 2026);
    partial.school_year = "2026".to_string();
    repo.upsert(&partial).expect("insert a half-typed label");

    let found = repo
        .find("class-1", "2026", "SEPTEMBER")
        .expect("look the half-typed label up")
        .expect("a half-typed label is still storable and findable");
    assert_eq!(found.school_year, "2026");
}
