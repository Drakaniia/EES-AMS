use super::*;
use crate::infrastructure::database::init_db;
use crate::sf2::month::{Sf2MonthTemplate, Sf2MonthTemplateRepo};

/// A migrated, empty database in a throwaway directory.
fn test_pool() -> DbPool {
    let dir = tempfile::tempdir().expect("temp dir for the test database");
    let pool = init_db(dir.path().join("attendance.db")).expect("migrate test database");
    std::mem::forget(dir);
    pool
}

/// Create the class and one month row, the way the split does.
fn seed_month(pool: &DbPool, id: &str, report_month: &str) {
    let conn = pool.get().expect("connection");
    conn.execute(
        "INSERT OR IGNORE INTO classes (id, name, day_start, day_end, late_after, created_at)
         VALUES ('class-1', 'Grade 1', '07:00', '13:00', '07:30', 1)",
        [],
    )
    .expect("insert class");

    Sf2MonthTemplateRepo::new(pool.clone())
        .upsert(&Sf2MonthTemplate {
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

/// Excel column letter for a 1-based column index: 6 is `F`.
fn column_letter(index: u32) -> String {
    let mut remaining = index;
    let mut letters = Vec::new();
    while remaining > 0 {
        let offset = (remaining - 1) % 26;
        letters.push((b'A' + u8::try_from(offset).expect("column offset")) as char);
        remaining = (remaining - 1) / 26;
    }
    letters.iter().rev().collect()
}

fn date_mapping(template_id: &str, date: &str, column_index: u32) -> Sf2MonthDateMapping {
    Sf2MonthDateMapping {
        template_id: template_id.to_string(),
        date: date.to_string(),
        column_letter: column_letter(column_index),
        column_index,
        // The worksheet the day is written to. One file holds twelve month
        // worksheets, so a column letter is not an address without it.
        sheet_name: Some("SEPTEMBER 2026".to_string()),
    }
}

fn september_grid(template_id: &str) -> Vec<Sf2MonthDateMapping> {
    vec![
        date_mapping(template_id, "2026-09-01", 6),
        date_mapping(template_id, "2026-09-02", 8),
        date_mapping(template_id, "2026-09-03", 9),
    ]
}

#[test]
fn a_month_grid_is_read_back_in_date_order() {
    let pool = test_pool();
    seed_month(&pool, "month-september", "SEPTEMBER");
    let repo = Sf2MonthDateRepo::new(pool);
    repo.replace_for_template("month-september", &september_grid("month-september"))
        .expect("seed the September grid");

    let stored = repo
        .for_template("month-september")
        .expect("read the September grid");
    assert_eq!(stored.len(), 3);
    assert_eq!(
        stored
            .iter()
            .map(|mapping| mapping.date.as_str())
            .collect::<Vec<_>>(),
        vec!["2026-09-01", "2026-09-02", "2026-09-03"]
    );
    assert_eq!(
        stored[0].column_letter, "F",
        "the grid keeps the column letters the analysis read"
    );
    assert_eq!(
        repo.count_for_template("month-september").expect("count"),
        3
    );

    // A single date resolves to its own column, which is the read the reports
    // grid does for every cell.
    let one = repo
        .for_date("month-september", "2026-09-02")
        .expect("read one date")
        .expect("the date is mapped");
    assert_eq!(one.column_index, 8);
    assert_eq!(one.column_letter, "H");
    assert!(repo
        .for_date("month-september", "2026-09-04")
        .expect("read an unmapped date")
        .is_none());
}

#[test]
fn replacing_one_months_grid_never_touches_another_month() {
    // The landmine of the old model: the mappings belonged to the class, so a
    // re-analysis of one month could clear the calendar of every month.
    let pool = test_pool();
    seed_month(&pool, "month-september", "SEPTEMBER");
    seed_month(&pool, "month-october", "OCTOBER");
    let repo = Sf2MonthDateRepo::new(pool);

    repo.replace_for_template("month-september", &september_grid("month-september"))
        .expect("seed September");
    repo.replace_for_template(
        "month-october",
        &[date_mapping("month-october", "2026-10-01", 6)],
    )
    .expect("seed October");

    repo.replace_for_template(
        "month-september",
        &[date_mapping("month-september", "2026-09-07", 6)],
    )
    .expect("re-derive September");

    assert_eq!(
        repo.count_for_template("month-september").expect("count"),
        1,
        "September holds only the re-derived date"
    );
    assert_eq!(
        repo.count_for_template("month-october").expect("count"),
        1,
        "October is a different file with its own grid"
    );
    assert_eq!(
        repo.for_template("month-october").expect("read October")[0].date,
        "2026-10-01"
    );
}

#[test]
fn an_empty_grid_is_rejected_instead_of_committed() {
    // Committing an empty grid leaves the month with no day columns: the reports
    // grid goes blank and the next workbook write has nothing to write into.
    // The existing mappings have to survive the bad analysis.
    let pool = test_pool();
    seed_month(&pool, "month-september", "SEPTEMBER");
    let repo = Sf2MonthDateRepo::new(pool);
    repo.replace_for_template("month-september", &september_grid("month-september"))
        .expect("seed the September grid");

    let error = repo
        .replace_for_template("month-september", &[])
        .expect_err("an empty grid must be rejected");

    assert!(
        matches!(error, AppError::InvalidInput(_)),
        "expected InvalidInput, got {error:?}"
    );
    assert_eq!(
        error.to_string(),
        "invalid input: The SF2 workbook produced no calendar dates. The existing mappings were left untouched."
    );
    assert_eq!(
        repo.count_for_template("month-september").expect("count"),
        3,
        "the good grid must survive a rejected analysis"
    );
}

#[test]
fn a_month_with_no_grid_reads_as_zero_columns() {
    let pool = test_pool();
    seed_month(&pool, "month-september", "SEPTEMBER");
    let repo = Sf2MonthDateRepo::new(pool);

    assert_eq!(
        repo.count_for_template("month-september").expect("count"),
        0
    );
    assert!(repo
        .for_template("month-september")
        .expect("read")
        .is_empty());
}

#[test]
fn deleting_a_months_grid_is_scoped_to_that_month() {
    let pool = test_pool();
    seed_month(&pool, "month-september", "SEPTEMBER");
    seed_month(&pool, "month-october", "OCTOBER");
    let repo = Sf2MonthDateRepo::new(pool);
    repo.replace_for_template("month-september", &september_grid("month-september"))
        .expect("seed September");
    repo.replace_for_template(
        "month-october",
        &[date_mapping("month-october", "2026-10-01", 6)],
    )
    .expect("seed October");

    assert_eq!(
        repo.delete_for_template("month-september").expect("delete"),
        3
    );
    assert_eq!(
        repo.count_for_template("month-september").expect("count"),
        0
    );
    assert_eq!(
        repo.count_for_template("month-october").expect("count"),
        1,
        "October survives September's delete"
    );
    assert_eq!(
        repo.delete_for_template("month-september").expect("delete"),
        0
    );
}
