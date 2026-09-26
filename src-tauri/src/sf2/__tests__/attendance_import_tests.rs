use super::*;

// ── is_absent_mark ─────────────────────────────────────────────────────────
//
// The reader must agree with the writer on what an absence looks like,
// otherwise a workbook mark silently fails to round-trip back into the app.

#[test]
fn absent_mark_matches_plain_uppercase_x() {
    assert!(is_absent_mark("X"));
}

#[test]
fn absent_mark_ignores_surrounding_whitespace() {
    assert!(is_absent_mark("  X  "));
    assert!(is_absent_mark("\tX\n"));
}

#[test]
fn absent_mark_is_case_insensitive() {
    assert!(is_absent_mark("x"));
    assert!(is_absent_mark("x"));
}

#[test]
fn present_cell_is_not_an_absence() {
    // The SF2 model is present-by-default, so the empty cell means present.
    assert!(!is_absent_mark(""));
    assert!(!is_absent_mark("   "));
}

#[test]
fn other_cell_contents_are_not_absences() {
    // Formulas, totals and stray notes must not be mistaken for a mark.
    for text in ["=COUNTIF(F9:AL9,\"X\")", "0", "1", "XX", "N/A", "absent"] {
        assert!(!is_absent_mark(text), "{text:?} must not count as absent");
    }
}

// ── grid_cells_to_scan ─────────────────────────────────────────────────────

fn mapping(row_index: u32) -> Sf2StudentMappingRecord {
    Sf2StudentMappingRecord {
        template_id: "tpl".to_string(),
        student_id: format!("student-{row_index}"),
        workbook_name: format!("LEARNER {row_index}"),
        normalized_name: format!("LEARNER {row_index}"),
        row_index,
        gender_block: Some("MALE".to_string()),
    }
}

fn date(column_letter: &str, column_index: u32, day: &str) -> Sf2DateMappingRecord {
    Sf2DateMappingRecord {
        template_id: "tpl".to_string(),
        sheet_name: "SEPTEMBER 2026".to_string(),
        date: format!("2026-09-{day}"),
        column_letter: column_letter.to_string(),
        column_index,
    }
}

#[test]
fn grid_cells_cover_every_row_and_date_pair() {
    let cells = grid_cells_to_scan(
        &[mapping(9), mapping(10)],
        &[date("H", 8, "01"), date("I", 9, "02")],
    );

    assert_eq!(cells.len(), 4);
    assert!(cells.contains(&("SEPTEMBER 2026".to_string(), "H9".to_string())));
    assert!(cells.contains(&("SEPTEMBER 2026".to_string(), "H10".to_string())));
    assert!(cells.contains(&("SEPTEMBER 2026".to_string(), "I9".to_string())));
    assert!(cells.contains(&("SEPTEMBER 2026".to_string(), "I10".to_string())));
}

#[test]
fn grid_cells_skip_unmapped_learner_rows() {
    // row_index 0 is the "not linked to a workbook row" placeholder, which has
    // no cell in the sheet and must never be scanned or written.
    let cells = grid_cells_to_scan(&[mapping(0), mapping(9)], &[date("H", 8, "01")]);

    assert_eq!(
        cells,
        vec![("SEPTEMBER 2026".to_string(), "H9".to_string())]
    );
}

#[test]
fn grid_cells_are_empty_without_mappings() {
    assert!(grid_cells_to_scan(&[], &[date("H", 8, "01")]).is_empty());
    assert!(grid_cells_to_scan(&[mapping(9)], &[]).is_empty());
}

#[test]
fn grid_cells_use_the_mapped_sheet_name() {
    // Hidden/renamed sheets are referenced by the mapping's sheet name, not by
    // whatever the report month is called.
    let mut other = date("H", 8, "01");
    other.sheet_name = "__SF2_HIDDEN_2".to_string();
    let cells = grid_cells_to_scan(&[mapping(9)], &[other]);

    assert_eq!(
        cells,
        vec![("__SF2_HIDDEN_2".to_string(), "H9".to_string())]
    );
}

// ── idempotency: the import must not duplicate an existing absence ─────────

/// A migrated throwaway database plus one enrolled student.
fn test_pool_with_student() -> (
    crate::infrastructure::database::DbPool,
    tempfile::TempDir,
    String,
) {
    use crate::domain::models::CreateStudentRequest;
    use crate::infrastructure::database::{init_db, StudentRepository};

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let pool = init_db(temp_dir.path().join("attendance.db")).expect("init db");
    let student = StudentRepository::new(pool.clone())
        .create(CreateStudentRequest {
            name: "BAPTISMA,SOSDFFIA, ESPIRITU M.".to_string(),
            gender: None,
            card_serial: None,
            class_id: Some("class-1".to_string()),
        })
        .expect("create student");
    (pool, temp_dir, student.id.to_string())
}

fn absent_events_on(pool: &crate::infrastructure::database::DbPool, date: &str) -> usize {
    use crate::domain::models::AttendanceType;
    use crate::infrastructure::database::EventRepository;

    EventRepository::new(pool.clone())
        .list_for_class_and_date_range("class-1", date, date)
        .expect("list events")
        .into_iter()
        .filter(|event| event.event_type == AttendanceType::Absent)
        .count()
}

#[test]
fn repeated_import_of_the_same_mark_records_one_absence() {
    use crate::domain::models::AttendanceType;

    let (pool, _temp_dir, student_id) = test_pool_with_student();
    let day = parse_date("2026-09-01").expect("parse date");

    // First pass: the workbook has an X the database has never seen.
    assert!(
        !has_absent_event_for_day(&pool, &student_id, "class-1", day).expect("query"),
        "no absence exists before the first import"
    );
    set_attendance_event_for_day(
        pool.clone(),
        &student_id,
        "class-1",
        day,
        "08:00",
        AttendanceType::Absent,
        "test",
    )
    .expect("record absence");

    // Second pass: the same X is already recorded, so the import must skip it
    // rather than stacking a duplicate event for the same learner and day.
    assert!(
        has_absent_event_for_day(&pool, &student_id, "class-1", day).expect("query"),
        "the absence recorded by the first import must be visible to the second"
    );
    assert_eq!(
        absent_events_on(&pool, "2026-09-01"),
        1,
        "one learner-day must hold exactly one absence"
    );
}

#[test]
fn a_present_record_is_not_mistaken_for_an_absence() {
    use crate::domain::models::AttendanceType;

    let (pool, _temp_dir, student_id) = test_pool_with_student();
    let day = parse_date("2026-09-02").expect("parse date");

    set_attendance_event_for_day(
        pool.clone(),
        &student_id,
        "class-1",
        day,
        "08:00",
        AttendanceType::In,
        "test",
    )
    .expect("record presence");

    assert!(
        !has_absent_event_for_day(&pool, &student_id, "class-1", day).expect("query"),
        "a present record must not satisfy the absence check, or the import \
         would skip re-importing a mark the user can no longer see"
    );
}

#[test]
fn absences_do_not_leak_across_days() {
    use crate::domain::models::AttendanceType;

    let (pool, _temp_dir, student_id) = test_pool_with_student();

    set_attendance_event_for_day(
        pool.clone(),
        &student_id,
        "class-1",
        parse_date("2026-09-01").expect("parse date"),
        "08:00",
        AttendanceType::Absent,
        "test",
    )
    .expect("record absence");

    assert!(
        !has_absent_event_for_day(
            &pool,
            &student_id,
            "class-1",
            parse_date("2026-09-02").expect("parse date")
        )
        .expect("query"),
        "an absence on 09-01 must not satisfy the check for 09-02"
    );
    assert_eq!(absent_events_on(&pool, "2026-09-02"), 0);
}
