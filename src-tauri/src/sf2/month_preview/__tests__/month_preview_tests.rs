//! Tests for the instant month switch (spec D9, D10, §7).
//!
//! Two kinds of test live here and the split matters.
//!
//! * **Structural.** A month switch is not allowed to touch Excel, mutate
//!   anything, or emit a progress event (acceptance #8, #9). Those are
//!   properties of the *call graph*, not of a return value, so they are asserted
//!   against the source of the switch path: reintroduce a COM call or an
//!   `emit_sf2_progress` into a month read and these fail.
//! * **Behavioural.** A real SQLite database, a real month row, a real grid -
//!   including the case the year wrap makes dangerous: AUGUST, where the legacy
//!   June rule and the school-year September rule name different years.
//!
//! There is no timing assertion in this file. A stopwatch around a read of a
//! mocked preview measures the mock, which is the one thing a performance test
//! must not do. The honest statement of the performance target lives in
//! `src/routes/reports/month-switch-perf.test.ts`, which asserts the same
//! structural properties from the frontend side and says plainly that its timing
//! number is a lower bound.

use super::*;
use crate::infrastructure::database::init_db;
use crate::sf2::attendance::attendance_service::set_preview_attendance_lightweight;
use crate::sf2::month::{is_school_day, Sf2MonthStudentMapping, Sf2MonthTemplateRepo};
use rusqlite::params;

// ── Source of the switch path, for the structural assertions ───────────────

/// The month-read module's own source.
const MONTH_PREVIEW_SOURCE: &str = include_str!("../mod.rs");

/// The command handlers' source. The switch commands live here.
const SF2_COMMANDS_SOURCE: &str = include_str!("../../../commands/sf2.rs");

/// The Excel month-switch service that used to be the whole switch.
const TEMPLATE_OPS_SOURCE: &str = include_str!("../../template/template_ops.rs");

/// Drop `//` line comments so a structural assertion is about *code*, not about
/// prose that happens to name a forbidden symbol.
///
/// Limitation, and it is a deliberate one: a `//` inside a string literal is
/// treated as a comment. Nothing in the sources checked here contains one, and a
/// false negative here is a weakened test rather than a wrong pass - the
/// behavioural tests below still cover what the structural ones cannot.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter_map(|line| {
            let code = match line.find("//") {
                Some(index) => &line[..index],
                None => line,
            };
            let trimmed = code.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(code)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Everything a month read must not reach: the COM serialisation, the COM
/// session, shelling out to Excel, the session the switch used to run, the
/// hide/rename loop it used to trigger, the mutating command it replaced, and
/// the progress event the deleted `ReportMonthSwitchOverlay` existed to cover.
const FORBIDDEN_IN_A_MONTH_READ: [&str; 10] = [
    "run_excel_task",
    "with_workbook",
    "ExcelSession",
    "open::that",
    "refresh_template_calendar_from_saved_month",
    "write_metadata",
    "configure_sf2_calendar",
    "set_report_month",
    "emit_sf2_progress",
    "sf2-progress",
];

/// The repositories the read is allowed to go through. Each one is a SELECT.
const THE_READ_REPOSITORIES: [&str; 7] = [
    "Sf2MonthTemplateRepo",
    "Sf2MonthDateRepo",
    "Sf2MonthStudentRepo",
    // The pre-split tables, read for whatever the month tables cannot answer.
    // `Sf2Repository` also holds the mutating methods, which is why the write-call
    // assertions below are the ones that carry the weight here, not the type.
    "Sf2Repository",
    "EventRepository",
    "StudentRepository",
    "ClassRepository",
];

#[test]
fn a_month_read_never_reaches_excel_and_never_writes() {
    let code = code_only(MONTH_PREVIEW_SOURCE);
    for forbidden in FORBIDDEN_IN_A_MONTH_READ {
        assert!(
            !code.contains(forbidden),
            "the month path must not mention `{forbidden}`: a month switch is one SQL read, \
             and this is a write or an Excel round-trip hiding in it"
        );
    }
}

#[test]
fn a_month_read_never_reads_the_legacy_june_year_rule() {
    // The two rules disagree only about AUGUST, and they disagree about which
    // *file* AUGUST is: `SF2-AUGUST-2026.xls` under the June rule,
    // `SF2-AUGUST-2027.xls` under the school-year rule. Recomputing the year
    // inside a month read is exactly how a month file gets filed under the wrong
    // one. The read takes `report_year` off the stored row instead.
    let code = code_only(MONTH_PREVIEW_SOURCE);
    assert!(
        !code.contains("sf2_report_year"),
        "the month path must not use the legacy June rule; it reads the stored report_year"
    );
    assert!(
        code.contains("report_year_for_school_month"),
        "the September rule is the only rule the month path may fall back on"
    );
}

#[test]
fn a_month_read_goes_through_read_only_repositories() {
    // Split the module at the create: the read half must contain no write call
    // at all, and the create half is the only place a write is allowed.
    let (read_half, create_half) = MONTH_PREVIEW_SOURCE
        .split_once("pub fn create_month_worksheet_in_dir")
        .expect("the create is the last function in the module");

    for repository in THE_READ_REPOSITORIES {
        assert!(
            read_half.contains(repository),
            "the month read is expected to go through {repository}"
        );
    }
    for write in [
        "replace_for_template",
        "upsert(",
        "delete_for_template",
        "record_workbook_x_count",
        "set_last_synced_at",
        "override_first_school_day",
    ] {
        assert!(
            !code_only(read_half).contains(write),
            "`{write}` appears in a month read; a switch must not write"
        );
    }
    assert!(
        code_only(create_half).contains("upsert("),
        "the create half is where writing belongs"
    );
}

#[test]
fn the_switch_command_emits_no_progress_and_runs_no_excel() {
    // Narrow the command source to the three switch commands, then assert the
    // same properties on them. A month switch has nothing to report progress
    // about, so a `sf2-progress` listener would be dead weight, and a COM call
    // would be the regression this whole phase exists to remove.
    let switch_block = SF2_COMMANDS_SOURCE
        .split_once("pub fn get_sf2_month_preview")
        .and_then(|(_, rest)| rest.split_once("pub fn get_sf2_export_readiness"))
        .map(|(block, _)| block)
        .expect(
            "the switch commands sit between get_sf2_month_preview and get_sf2_export_readiness",
        );
    let code = code_only(switch_block);

    for forbidden in [
        "emit_sf2_progress",
        "sf2-progress",
        "run_excel_task",
        "with_workbook",
        "spawn_blocking",
    ] {
        assert!(
            !code.contains(forbidden),
            "a month switch must not go through `{forbidden}`; it is one SQL read"
        );
    }
    // The read is a plain sync command. `spawn_blocking` would be defensible for
    // a slow query, but it is also the shape every Excel command in this file
    // has, and a month switch is measured in single-digit milliseconds.
    assert!(
        SF2_COMMANDS_SOURCE.contains("pub fn get_sf2_month_preview("),
        "get_sf2_month_preview must be a sync command"
    );
}

#[test]
fn the_old_excel_month_switch_path_is_gone() {
    for source in [MONTH_PREVIEW_SOURCE, SF2_COMMANDS_SOURCE] {
        assert!(
            !code_only(source).contains("set_sf2_report_month"),
            "the mutating month-switch command must be deleted, not bypassed"
        );
    }
    assert!(
        !code_only(TEMPLATE_OPS_SOURCE).contains("set_report_month"),
        "the Excel month-switch service must be deleted (acceptance #8-#10)"
    );
    // The hide/rename/clear loop went with it: `configure_sf2_calendar` is now a
    // date-header writer with no `sf2_sheets` argument to loop over
    // (acceptance #18).
    let calendar = code_only(include_str!("../../excel/excel_com/calendar.rs"));
    assert!(
        !calendar.contains("__SF2_HIDDEN"),
        "the hidden-tab rename loop must be gone (acceptance #18)"
    );
    assert!(
        !calendar.contains("EXCEL_SHEET_HIDDEN"),
        "nothing hides a sheet any more; a month file has one"
    );
}

// ── Fixtures ───────────────────────────────────────────────────────────────

const CLASS_ID: &str = "class-1";
const SCHOOL_YEAR: &str = "2026-2027";

/// Student ids are UUIDs - `student_from_row` parses them and unwraps, so a
/// readable placeholder would panic the read rather than fail an assertion.
const STUDENT_ONE: &str = "11111111-1111-4111-8111-111111111111";
const STUDENT_TWO: &str = "22222222-2222-4222-8222-222222222222";

/// A migrated, empty database in a throwaway directory.
fn test_pool() -> DbPool {
    let dir = tempfile::tempdir().expect("temp dir for the test database");
    let pool = init_db(dir.path().join("attendance.db")).expect("migrate test database");
    std::mem::forget(dir);
    pool
}

/// A workbook directory in a throwaway place, for the `file_exists` stat.
fn test_workbook_dir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir for the workbooks");
    let path = dir.path().join("sf2-workbooks");
    std::fs::create_dir_all(&path).expect("create the workbook directory");
    (dir, path)
}

fn seed_class(pool: &DbPool) {
    let conn = pool.get().expect("connection");
    conn.execute(
        "INSERT OR IGNORE INTO classes (id, name, day_start, day_end, late_after, created_at)
         VALUES (?1, 'Grade 1 - A', '07:00', '13:00', '07:30', 1)",
        [CLASS_ID],
    )
    .expect("insert class");
}

fn seed_student(pool: &DbPool, id: &str, name: &str) {
    let conn = pool.get().expect("connection");
    // Idempotent: `seed_roster` seeds the students a mapping references, and some
    // tests also name them up front. Seeding the same learner twice is not the
    // thing under test.
    conn.execute(
        "INSERT OR IGNORE INTO students (id, name, class_id, created_at) VALUES (?1, ?2, ?3, 4)",
        params![id, name, CLASS_ID],
    )
    .expect("insert student");
}

/// Record one `absent` event on a local calendar date.
///
/// The event id has to be a UUID: `attendance_event_from_row` parses it and
/// unwraps, so a readable `"event-student-1-2026-09-01"` would panic the read
/// rather than fail an assertion. The same is true of student ids, which is why
/// [`STUDENT_ONE`] is a UUID and not a name.
fn seed_absent(pool: &DbPool, student_id: &str, date: &str) {
    static NEXT_EVENT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
    let sequence = NEXT_EVENT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let day = parse_date(date).expect("a real date");
    let timestamp = day
        .and_hms_opt(8, 0, 0)
        .expect("08:00")
        .and_local_timezone(chrono::Local)
        .earliest()
        .expect("a local time")
        .with_timezone(&chrono::Utc)
        .timestamp();
    let conn = pool.get().expect("connection");
    conn.execute(
        "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
         VALUES (?1, ?2, ?3, 'absent', ?4, NULL, NULL, NULL, NULL)",
        params![
            format!("33333333-3333-4333-8333-{sequence:012}"),
            student_id,
            CLASS_ID,
            timestamp
        ],
    )
    .expect("insert absent event");
}

fn set_school_start_date(pool: &DbPool, value: &str) {
    let conn = pool.get().expect("connection");
    conn.execute(
        "UPDATE settings SET school_start_date = ?1 WHERE id = 'app'",
        [value],
    )
    .expect("set school_start_date");
}

fn set_last_report_month(pool: &DbPool, month: &str) {
    let conn = pool.get().expect("connection");
    conn.execute(
        "UPDATE settings SET last_report_month = ?1 WHERE id = 'app'",
        [month],
    )
    .expect("set last_report_month");
}

/// Pin the school year without seeding any month rows.
///
/// Needed to reach the "this month has no school days" case deliberately: with no
/// school year on record the resolution falls back to the clock, and every month
/// then lands in a year that has classes in it.
fn set_school_year(pool: &DbPool, year: &str) {
    let conn = pool.get().expect("connection");
    conn.execute(
        "UPDATE settings SET school_year = ?1 WHERE id = 'app'",
        [year],
    )
    .expect("set school_year");
}

/// Store one month row, the way the split does.
fn seed_month(pool: &DbPool, id: &str, month: &str, report_year: i32) -> Sf2MonthTemplate {
    let template = Sf2MonthTemplate {
        id: id.to_string(),
        active_class_id: CLASS_ID.to_string(),
        school_year: SCHOOL_YEAR.to_string(),
        report_month: month.to_string(),
        report_year,
        source_path: format!("C:/sf2-workbooks/SF2-{month}-{report_year}.xls"),
        source_hash: format!("hash-{id}"),
        school_id: Some("S-1".to_string()),
        school_name: Some("Espiritu Elementary".to_string()),
        grade_level: Some("1".to_string()),
        section: Some("A".to_string()),
        adviser_name: Some("Dela Cruz".to_string()),
        school_head_name: Some("Santos".to_string()),
        first_school_day: 1,
        first_school_day_override: None,
        imported_at: 1_000,
        last_synced_at: None,
        workbook_x_count: 0,
        workbook_scanned_at: None,
    };
    Sf2MonthTemplateRepo::new(pool.clone())
        .upsert(&template)
        .expect("insert the month row");
    template
}

fn seed_roster(pool: &DbPool, template_id: &str, students: &[(&str, &str, u32)]) {
    let mappings = students
        .iter()
        .map(
            |(student_id, workbook_name, row_index)| Sf2MonthStudentMapping {
                template_id: template_id.to_string(),
                student_id: (*student_id).to_string(),
                workbook_name: (*workbook_name).to_string(),
                normalized_name: workbook_name.to_lowercase(),
                row_index: *row_index,
                gender_block: Some("MALE".to_string()),
                sf2_learner_id: None,
            },
        )
        .collect::<Vec<_>>();
    // `sf2_month_student_mappings.student_id` references `students(id)`, so the
    // roster cannot be written before the students exist. Doing it here keeps the
    // referential order in one place instead of in every caller.
    for (student_id, workbook_name, _) in students {
        seed_student(pool, student_id, workbook_name);
    }
    Sf2MonthStudentRepo::new(pool.clone())
        .replace_for_template(template_id, &mappings)
        .expect("insert the month roster");
}

fn seed_grid(pool: &DbPool, template_id: &str, dates: &[(&str, u32)]) {
    let mappings = dates
        .iter()
        .map(|(date, column_index)| Sf2MonthDateMapping {
            template_id: template_id.to_string(),
            date: (*date).to_string(),
            column_letter: column_letter(*column_index),
            column_index: *column_index,
            // Every day of a month lives on that month's own worksheet, and the
            // stored name is what a write path reads back to address the cell.
            sheet_name: Some("SEPTEMBER 2026".to_string()),
        })
        .collect::<Vec<_>>();
    Sf2MonthDateRepo::new(pool.clone())
        .replace_for_template(template_id, &mappings)
        .expect("insert the month grid");
}

// ── The pre-split tables ───────────────────────────────────────────────────
//
// Everything below seeds `sf2_templates` / `sf2_student_mappings` /
// `sf2_date_mappings` directly rather than through a repository, because these
// tables have no write path left: the code that populated them is the pre-split
// code, and the per-month model does not write them. They are the tables the
// affected install's data actually lives in, which is exactly why they are read
// and never touched.

const LEGACY_TEMPLATE_ID: &str = "legacy-template-1";

/// One pre-split template row for the class, as the v9-v18 code left it.
fn seed_legacy_template(pool: &DbPool, report_month: &str) {
    let conn = pool.get().expect("connection");
    conn.execute(
        "INSERT OR IGNORE INTO sf2_templates (
            id, source_path, source_hash, school_id, school_name, school_year,
            report_month, grade_level, section, adviser_name, school_head_name,
            layout_fingerprint, active_class_id, imported_at
         ) VALUES (
            ?1, 'C:/sf2-workbooks/SF2-GRADE-3-MATAPAT.xls', 'legacy-hash', 'S-1',
            'Espiritu Elementary', ?2, ?3, '3', 'MATAPAT', 'Dela Cruz', 'Santos',
            'legacy-layout', ?4, 500
         )",
        params![LEGACY_TEMPLATE_ID, SCHOOL_YEAR, report_month, CLASS_ID],
    )
    .expect("insert the pre-split template row");
}

/// The pre-split roster: `sf2_student_mappings`, keyed `(template_id, student_id)`
/// over one shared row set rather than twelve.
fn seed_legacy_roster(pool: &DbPool, students: &[(&str, &str, u32)]) {
    for (student_id, workbook_name, _) in students {
        seed_student(pool, student_id, workbook_name);
    }
    let conn = pool.get().expect("connection");
    for (student_id, workbook_name, row_index) in students {
        conn.execute(
            "INSERT OR IGNORE INTO sf2_student_mappings (
                template_id, student_id, workbook_name, normalized_name, row_index, gender_block
             ) VALUES (?1, ?2, ?3, ?4, ?5, 'MALE')",
            params![
                LEGACY_TEMPLATE_ID,
                student_id,
                workbook_name,
                workbook_name.to_lowercase(),
                row_index
            ],
        )
        .expect("insert the pre-split student mapping");
    }
}

/// The pre-split day-number grid, as the analysis wrote it: a full `YYYY-MM-DD`
/// per day, with **no month scoping of its own** - which is the property the
/// month-scoped fallback has to impose.
fn seed_legacy_grid(pool: &DbPool, dates: &[(&str, u32)]) {
    let conn = pool.get().expect("connection");
    for (date, column_index) in dates {
        conn.execute(
            "INSERT OR IGNORE INTO sf2_date_mappings (
                template_id, sheet_name, date, column_letter, column_index
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                LEGACY_TEMPLATE_ID,
                &date[..7],
                date,
                column_letter(*column_index),
                *column_index
            ],
        )
        .expect("insert the pre-split date mapping");
    }
}

fn read_month(pool: &DbPool, workbooks: &Path, month: &str) -> Sf2MonthGridPreview {
    month_preview(pool, workbooks, Some(CLASS_ID), Some(SCHOOL_YEAR), month)
        .unwrap_or_else(|error| panic!("reading {month} failed: {error}"))
}

// ── The read ───────────────────────────────────────────────────────────────

#[test]
fn a_month_read_returns_every_weekday_with_its_stored_column() {
    let pool = test_pool();
    seed_class(&pool);
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    seed_roster(
        &pool,
        "month-september",
        &[(STUDENT_ONE, "DELA CRUZ, JUAN", 8)],
    );
    seed_grid(
        &pool,
        "month-september",
        &[("2026-09-01", 6), ("2026-09-02", 8)],
    );
    let (_dir, workbooks) = test_workbook_dir();

    let preview = read_month(&pool, &workbooks, "SEPTEMBER");

    assert_eq!(preview.month, "SEPTEMBER");
    assert_eq!(preview.report_year, 2026);
    assert_eq!(preview.sheet_name, "SEPTEMBER 2026");
    assert_eq!(preview.file_name, "SF2-SEPTEMBER-2026.xls");
    assert!(preview.has_template);
    assert!(!preview.file_exists, "the fixture wrote no file");
    assert!(!preview.grid_empty);
    assert_eq!(preview.mapped_dates, 2);

    // Every Monday-Friday of the month is listed, mapped or not. That is what
    // makes the grid paint without a second read.
    let dates = preview
        .dates
        .iter()
        .map(|date| date.date.as_str())
        .collect::<Vec<_>>();
    assert_eq!(dates.len(), 22, "September 2026 has 22 weekdays");
    assert_eq!(dates.first(), Some(&"2026-09-01"));
    assert_eq!(dates.last(), Some(&"2026-09-30"));
    assert!(dates.iter().all(|date| date.starts_with("2026-09-")));

    let mapped = preview
        .dates
        .iter()
        .find(|date| date.date == "2026-09-02")
        .expect("2 September is listed");
    assert_eq!(mapped.column_letter, "H");
    let unmapped = preview
        .dates
        .iter()
        .find(|date| date.date == "2026-09-04")
        .expect("4 September is listed");
    assert!(unmapped.column_letter.is_empty());
    assert_eq!(unmapped.column_index, 0);
}

#[test]
fn a_month_read_reports_the_x_marks_the_database_holds() {
    let pool = test_pool();
    seed_class(&pool);
    seed_student(&pool, STUDENT_ONE, "Juan Dela Cruz");
    seed_student(&pool, STUDENT_TWO, "Maria Santos");
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    seed_roster(
        &pool,
        "month-september",
        &[
            (STUDENT_ONE, "DELA CRUZ, JUAN", 8),
            (STUDENT_TWO, "SANTOS, MARIA", 9),
        ],
    );
    seed_grid(&pool, "month-september", &[("2026-09-01", 6)]);
    seed_absent(&pool, STUDENT_ONE, "2026-09-01");
    let (_dir, workbooks) = test_workbook_dir();

    let preview = read_month(&pool, &workbooks, "SEPTEMBER");

    assert_eq!(preview.absence_count, 1);
    assert_eq!(preview.absent_list.len(), 1);
    assert_eq!(preview.absent_list[0].student_name, "Juan Dela Cruz");
    assert_eq!(preview.absent_list[0].date, "2026-09-01");
    assert_eq!(preview.absent_list[0].row_index, 8);
    assert_eq!(preview.mapped_students, 2);
    assert_eq!(preview.unmapped_student_count, 0);
}

#[test]
fn a_month_read_for_a_month_with_no_row_still_answers_and_says_so() {
    let pool = test_pool();
    seed_class(&pool);
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    let (_dir, workbooks) = test_workbook_dir();

    let preview = read_month(&pool, &workbooks, "NOVEMBER");

    assert!(!preview.has_template);
    assert!(preview.grid_empty);
    assert_eq!(preview.first_school_day, FIRST_SCHOOL_DAY_UNDETERMINED);
    assert!(preview.sheet_name.is_empty());
    assert!(preview
        .issues
        .iter()
        .any(|issue| issue.contains("No SF2 workbook is stored for NOVEMBER")));
}

#[test]
fn a_month_read_reports_a_missing_file_rather_than_failing() {
    // Edge case E4: the row is on record, the file is gone. The read still
    // answers - the grid shows what the database holds - and says the file is
    // missing, so every write path knows to refuse.
    let pool = test_pool();
    seed_class(&pool);
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    seed_grid(&pool, "month-september", &[("2026-09-01", 6)]);
    let (_dir, workbooks) = test_workbook_dir();

    let preview = read_month(&pool, &workbooks, "SEPTEMBER");

    assert!(preview.has_template);
    assert!(!preview.file_exists);
    assert!(
        !preview.grid_empty,
        "the grid is in the database, not in the file"
    );
    assert!(preview
        .issues
        .iter()
        .any(|issue| issue.contains("missing from disk")));
}

#[test]
fn august_uses_the_stored_year_and_not_the_legacy_june_rule() {
    // The one month where the two rules disagree. School year 2026-2027 runs
    // SEPTEMBER 2026 -> AUGUST 2027, so August's calendar year is 2027. The
    // legacy June rule would say 2026, which is a different file.
    let pool = test_pool();
    seed_class(&pool);
    seed_month(&pool, "month-august", "AUGUST", 2027);
    seed_grid(&pool, "month-august", &[("2027-08-02", 6)]);
    let (_dir, workbooks) = test_workbook_dir();

    let preview = read_month(&pool, &workbooks, "AUGUST");

    assert_eq!(
        preview.report_year, 2027,
        "August of 2026-2027 is August 2027, not August 2026"
    );
    assert_eq!(preview.file_name, "SF2-AUGUST-2027.xls");
    assert_eq!(preview.sheet_name, "AUGUST 2027");
    assert!(
        preview
            .dates
            .iter()
            .all(|date| date.date.starts_with("2027-08-")),
        "and the grid agrees: these are 2027 dates"
    );
}

#[test]
fn august_with_no_stored_row_falls_back_to_the_school_year_rule() {
    // The stored row is authoritative when there is one. With no row there is no
    // stored answer to disagree with, so the September rule fills in - and it
    // still says 2027, not 2026.
    let pool = test_pool();
    seed_class(&pool);
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    let (_dir, workbooks) = test_workbook_dir();

    let preview = read_month(&pool, &workbooks, "AUGUST");

    assert_eq!(preview.report_year, 2027);
    assert_eq!(
        preview.file_name, "",
        "with no month row and no pre-split row there is no workbook to name, and a
         per-month file name would be a lie: there is no such file any more"
    );
}

#[test]
fn a_month_read_rejects_a_month_name_it_cannot_resolve() {
    let pool = test_pool();
    seed_class(&pool);
    let (_dir, workbooks) = test_workbook_dir();

    // "SMARCH" is not a month: `sf2_month_number` matches on substrings, and
    // "SMARCH" contains "MAR". A token with no month substring in it is the only
    // honest way to ask for a name that cannot resolve.
    let error = month_preview(
        &pool,
        &workbooks,
        Some(CLASS_ID),
        Some(SCHOOL_YEAR),
        "BLYTH",
    )
    .expect_err("an unresolvable month is an error, not an empty grid");

    assert_eq!(
        error.to_string(),
        format!("invalid input: {UNKNOWN_MONTH_MESSAGE}")
    );
}

#[test]
fn a_month_read_answers_for_the_only_class_when_none_is_named() {
    // The Reports page has no class selected on first load. Guessing wrong here
    // would show one class's grid under another's heading.
    let pool = test_pool();
    seed_class(&pool);
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    let (_dir, workbooks) = test_workbook_dir();

    let preview = month_preview(&pool, &workbooks, None, Some(SCHOOL_YEAR), "SEPTEMBER")
        .expect("the only class on record is used");

    assert_eq!(preview.class_id, CLASS_ID);
    assert_eq!(preview.class_name, "Grade 1 - A");
}

#[test]
fn a_month_read_resolves_the_school_year_from_the_months_on_record() {
    // A fresh launch names no school year. Resolving it to the year the class
    // actually has months for is what makes the read land on a real row instead
    // of a guess.
    let pool = test_pool();
    seed_class(&pool);
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    let (_dir, workbooks) = test_workbook_dir();

    let preview =
        month_preview(&pool, &workbooks, Some(CLASS_ID), None, "SEPTEMBER").expect("read");

    assert_eq!(preview.school_year, SCHOOL_YEAR);
    assert!(preview.has_template);
}

// ── D5: which month to open on launch ──────────────────────────────────────

#[test]
fn launch_opens_todays_month_when_it_has_a_file() {
    let pool = test_pool();
    seed_class(&pool);
    // The start date has to be set for the month to be datable at all (E3). A
    // month nobody can date is not a month a create may be offered for.
    set_school_start_date(&pool, "2026-08-03");
    seed_month(&pool, "month-october", "OCTOBER", 2026);
    let (_dir, workbooks) = test_workbook_dir();
    point_month_at_workbook(&pool, "month-october", &workbooks);
    touch(&single_workbook_path_in(&workbooks));

    let launch = launch_month(&pool, &workbooks, Some(CLASS_ID), date("2026-10-15"))
        .expect("resolve the launch month");

    assert_eq!(launch.month, "OCTOBER");
    assert_eq!(launch.report_year, 2026);
    assert!(
        !launch.fell_back,
        "today's month was available, there was nothing to fall back to"
    );
    assert!(launch.file_exists);
    assert!(launch.has_school_days);
    assert!(!launch.can_create);
    assert!(launch.issues.is_empty());
}

#[test]
fn launch_falls_back_to_the_last_used_month_and_says_so() {
    // Edge case E1: the app is opened in a month that has no file. The fallback
    // is never silent - `fell_back` is set and both months are named, because a
    // teacher who opens the app in June and lands on May is otherwise left to
    // guess why.
    let pool = test_pool();
    seed_class(&pool);
    set_school_start_date(&pool, "2025-08-04");
    // School year 2026-2027, so June and May are both 2027: months 1-8 belong to
    // the following calendar year.
    seed_month(&pool, "month-may", "MAY", 2027);
    let (_dir, workbooks) = test_workbook_dir();
    point_month_at_workbook(&pool, "month-may", &workbooks);
    touch(&single_workbook_path_in(&workbooks));
    set_last_report_month(&pool, "MAY");

    let launch = launch_month(&pool, &workbooks, Some(CLASS_ID), date("2026-06-15"))
        .expect("resolve the launch month");

    assert_eq!(launch.month, "MAY");
    assert_eq!(launch.report_year, 2027, "May of 2026-2027 is May 2027");
    assert!(launch.fell_back);
    assert_eq!(launch.today_month, "JUNE");
    assert_eq!(launch.today_report_year, 2027);
    assert!(
        launch.today_can_create,
        "June has school days, so the one-click create may be offered"
    );
    assert!(
        !launch.can_create,
        "May is already on record, so nothing should be created for it"
    );
}

#[test]
fn launch_never_offers_a_create_for_a_month_with_no_school_days() {
    // Edge case E2: a month with no school days. A file for it has no day to
    // record, so the app must not offer to make one. The school year is pinned
    // and the start date is set *after* the month, which is the only way to get
    // here on purpose.
    let pool = test_pool();
    seed_class(&pool);
    let (_dir, workbooks) = test_workbook_dir();
    // Pin the school year so APRIL lands in 2026, and set the start date *after*
    // the month so April is entirely before classes began. That is the only way
    // to reach "no school days" on purpose.
    set_school_year(&pool, "2025-2026");
    set_school_start_date(&pool, "2026-08-03");

    let launch = launch_month(&pool, &workbooks, Some(CLASS_ID), date("2026-04-15"))
        .expect("resolve the launch month");

    assert_eq!(launch.month, "APRIL");
    assert_eq!(launch.report_year, 2026, "April of 2025-2026 is April 2026");
    assert!(
        !launch.has_school_days,
        "April 2026 is before the August 2026 start, and must say so rather than guess"
    );
    assert!(
        !launch.today_can_create,
        "E2: never offer to create a file for a month with no school days"
    );
    assert!(!launch.can_create);
}

#[test]
fn launch_reports_a_missing_start_date_instead_of_guessing_one() {
    // Edge case E3: `school_start_date` unset. No month can be dated, and the
    // app prompts once rather than inventing a first attendance day.
    let pool = test_pool();
    seed_class(&pool);
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    let (_dir, workbooks) = test_workbook_dir();

    let launch = launch_month(&pool, &workbooks, Some(CLASS_ID), date("2026-09-15"))
        .expect("resolve the launch month");

    assert!(launch.needs_school_start_date);
    assert!(
        !launch.has_school_days,
        "an undated month reports no school days rather than a guess"
    );
    assert!(
        !launch.can_create,
        "an undated month is not a month to create into"
    );
}

#[test]
fn launch_never_offers_a_create_for_a_month_that_already_has_a_worksheet() {
    let pool = test_pool();
    seed_class(&pool);
    set_school_start_date(&pool, "2025-08-04");
    // Pin the school year so the resolved month is deterministic: months 1-8 of
    // a 2026-2027 school year are in 2027, so June is June 2027.
    set_school_year(&pool, SCHOOL_YEAR);
    let (_dir, workbooks) = test_workbook_dir();
    seed_month(&pool, "month-june", "JUNE", 2027);
    point_month_at_workbook(&pool, "month-june", &workbooks);
    touch(&single_workbook_path_in(&workbooks));

    let launch = launch_month(&pool, &workbooks, Some(CLASS_ID), date("2026-06-15"))
        .expect("resolve the launch month");

    assert_eq!(launch.month, "JUNE");
    assert_eq!(launch.report_year, 2027);
    assert!(launch.file_exists);
    assert!(!launch.can_create);
    assert!(!launch.today_can_create);
    assert!(!launch.fell_back);
}

#[test]
fn launch_says_so_when_neither_month_exists() {
    // Nothing on record at all. The app must not silently open a blank grid and
    // leave the teacher wondering whether the app lost their data.
    let pool = test_pool();
    seed_class(&pool);
    set_school_start_date(&pool, "2026-08-03");
    let (_dir, workbooks) = test_workbook_dir();

    let launch = launch_month(&pool, &workbooks, Some(CLASS_ID), date("2026-10-15"))
        .expect("resolve the launch month");

    assert_eq!(launch.month, "OCTOBER");
    assert!(launch.fell_back);
    assert!(
        launch.today_can_create,
        "October has school days and no file"
    );
    assert!(
        launch
            .issues
            .iter()
            .any(|issue| issue.contains("No SF2 workbook exists")),
        "the missing-everything case must be said out loud: {:?}",
        launch.issues
    );
}

// â”€â”€ Â§0 A1: the one workbook, twelve worksheets â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// The one workbook's name, as the merge job and every read agree on it.
const SINGLE_WORKBOOK_NAME: &str = "SF2-1-A-a4f5d22a.xls";

/// Where the one workbook is, inside a workbook directory.
fn single_workbook_path_in(workbooks: &Path) -> PathBuf {
    workbooks.join(SINGLE_WORKBOOK_NAME)
}

/// Put a real workbook where the class's pre-split row says it is, and point
/// that row at it.
///
/// Under §0 A1 the class has ONE file and every month row's `source_path` names
/// it. A test that needs a file the app can actually open therefore has to put
/// bytes at the row's `source_path`, not at a per-month name.
fn seed_workbook_on_disk(pool: &DbPool, workbooks: &Path) {
    let conn = pool.get().expect("connection");
    conn.execute(
        "INSERT OR IGNORE INTO sf2_templates (id, source_path, source_hash, school_year,
            grade_level, section, layout_fingerprint, active_class_id, imported_at, school_id,
            school_name, report_month, adviser_name, school_head_name)
         VALUES (?1, ?2, 'bundled-seed', ?3, '1', 'A', 'fingerprint', ?4, 1782000000,
            'S-1', 'Espiritu Elementary', 'SEPTEMBER', 'Dela Cruz, Juan', 'Santos, Maria')",
        params![
            LEGACY_TEMPLATE_ID,
            single_workbook_path_in(workbooks)
                .to_string_lossy()
                .to_string(),
            SCHOOL_YEAR,
            CLASS_ID
        ],
    )
    .expect("insert the pre-split row");
    // A real workbook, not a placeholder: the create opens it and copies a
    // worksheet out of it, so a file of text bytes would fail for a reason that
    // has nothing to do with what is being tested.
    std::fs::write(
        single_workbook_path_in(workbooks),
        crate::sf2::workbook_files::BUNDLED_TEMPLATE_BYTES,
    )
    .expect("write a real bundled-template workbook");
}
/// Point a month's stored row at the one workbook, so a read finds it there.
fn point_month_at_workbook(pool: &DbPool, template_id: &str, workbooks: &Path) {
    let conn = pool.get().expect("connection");
    conn.execute(
        "UPDATE sf2_month_templates SET source_path = ?1 WHERE id = ?2",
        params![
            single_workbook_path_in(workbooks)
                .to_string_lossy()
                .to_string(),
            template_id
        ],
    )
    .expect("point the month row at the one workbook");
}

// â”€â”€ E1: the one-click create â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn a_month_read_names_the_one_workbook_every_month_shares() {
    // Â§0 A1: there is one file, and `file_name` is the same string for all twelve
    // months. What differs per month is the worksheet, not the file.
    let pool = test_pool();
    seed_class(&pool);
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    seed_month(&pool, "month-october", "OCTOBER", 2026);
    let (_dir, workbooks) = test_workbook_dir();
    point_month_at_workbook(&pool, "month-september", &workbooks);
    point_month_at_workbook(&pool, "month-october", &workbooks);

    let september = read_month(&pool, &workbooks, "SEPTEMBER");
    let october = read_month(&pool, &workbooks, "OCTOBER");

    assert_eq!(september.file_name, SINGLE_WORKBOOK_NAME);
    assert_eq!(october.file_name, SINGLE_WORKBOOK_NAME);
    assert_ne!(
        september.sheet_name, october.sheet_name,
        "the months differ by worksheet, which is what a column letter is resolved against"
    );
    assert_eq!(september.sheet_name, "SEPTEMBER 2026");
    assert_eq!(october.sheet_name, "OCTOBER 2026");
}

#[test]
fn every_day_of_a_month_carries_the_worksheet_it_is_written_to() {
    // A column letter is not an address in a twelve-sheet file without the sheet,
    // and the stored `sheet_name` is what a write path reads back. So every date
    // the grid renders names its own month worksheet, not whatever month the read
    // happened to be for.
    let pool = test_pool();
    seed_class(&pool);
    seed_student(&pool, STUDENT_ONE, "Juan Dela Cruz");
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    seed_roster(
        &pool,
        "month-september",
        &[(STUDENT_ONE, "DELA CRUZ, JUAN", 8)],
    );
    seed_grid(&pool, "month-september", &[("2026-09-01", 6)]);
    let (_dir, workbooks) = test_workbook_dir();

    let preview = read_month(&pool, &workbooks, "SEPTEMBER");

    assert!(!preview.dates.is_empty());
    for date in &preview.dates {
        assert_eq!(
            date.sheet_name, "SEPTEMBER 2026",
            "{} is on the wrong worksheet",
            date.date
        );
    }
    let mapped = preview
        .dates
        .iter()
        .find(|date| date.date == "2026-09-01")
        .expect("the mapped day is rendered");
    assert_eq!(mapped.column_letter, "F");
    assert_eq!(mapped.column_index, 6);
}

#[test]
fn two_months_with_different_marks_render_different_grids() {
    // The failure this whole project exists to prevent is a grid that says nobody
    // was absent while the database disagrees. A month filter that is silently
    // ignored produces exactly that, so it is asserted directly: the same
    // database, two months, two different mark sets.
    let pool = test_pool();
    seed_class(&pool);
    seed_student(&pool, STUDENT_ONE, "Juan Dela Cruz");
    seed_student(&pool, STUDENT_TWO, "Maria Santos");
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    seed_month(&pool, "month-october", "OCTOBER", 2026);
    seed_roster(
        &pool,
        "month-september",
        &[(STUDENT_ONE, "DELA CRUZ, JUAN", 8)],
    );
    seed_roster(&pool, "month-october", &[(STUDENT_TWO, "SANTOS, MARIA", 8)]);
    seed_grid(&pool, "month-september", &[("2026-09-01", 6)]);
    seed_grid(&pool, "month-october", &[("2026-10-01", 6)]);
    seed_absent(&pool, STUDENT_ONE, "2026-09-01");
    seed_absent(&pool, STUDENT_TWO, "2026-10-01");
    let (_dir, workbooks) = test_workbook_dir();

    let september = read_month(&pool, &workbooks, "SEPTEMBER");
    let october = read_month(&pool, &workbooks, "OCTOBER");

    assert_eq!(september.absence_count, 1, "September's one absence");
    assert_eq!(october.absence_count, 1, "October's one absence");
    assert_ne!(
        september.students[0].student_id.as_str(),
        october.students[0].student_id.as_str(),
        "a month filter that is ignored shows the same learner for both months"
    );
    assert_eq!(september.students[0].absent_count, 1);
    assert_eq!(october.students[0].absent_count, 1);
    assert_eq!(september.students[0].present_count, 0);
    assert_eq!(october.students[0].present_count, 0);
    assert!(september.absent_list[0].date.starts_with("2026-09-"));
    assert!(october.absent_list[0].date.starts_with("2026-10-"));
}

#[test]
fn creating_a_month_worksheet_records_the_month_the_row_and_the_roster() {
    let pool = test_pool();
    seed_class(&pool);
    set_school_start_date(&pool, "2026-08-03");
    seed_student(&pool, STUDENT_ONE, "Juan Dela Cruz");
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    seed_roster(
        &pool,
        "month-september",
        &[(STUDENT_ONE, "DELA CRUZ, JUAN", 8)],
    );
    let (_dir, workbooks) = test_workbook_dir();
    seed_workbook_on_disk(&pool, &workbooks);

    let template = create_month_worksheet_in_dir(&pool, &workbooks, Some(CLASS_ID), "OCTOBER")
        .expect("create October");

    assert_eq!(template.report_month, "OCTOBER");
    assert_eq!(
        template.report_year, 2026,
        "October of 2026-2027 is October 2026"
    );
    assert_eq!(
        template.source_path,
        single_workbook_path_in(&workbooks).to_string_lossy(),
        "the month lives on the one workbook, not on a file of its own"
    );

    // The roster is carried over, so the new worksheet is immediately usable.
    let roster = Sf2MonthStudentRepo::new(pool.clone())
        .for_template(&template.id)
        .expect("roster");
    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].student_id, STUDENT_ONE);
    assert_eq!(roster[0].row_index, 8);

    // Measured, not "never scanned": the build counted the marks it wrote into
    // the worksheet, so this month is on record as holding zero. A month nobody
    // has counted is `Unmeasured` (spec §9.1, edge case E4), and that is a
    // different state - one nothing may clear on its word.
    assert_eq!(template.workbook_x_count, 0);
    assert!(
        template.is_workbook_measured(),
        "the build counted the worksheet it just wrote, so this month is measured"
    );

    // The day-number grid comes from the worksheet that was just written, so the
    // new month is dated the moment it exists instead of reading as
    // `grid_empty` - the state SEPTEMBER 2026 was in on the install this was
    // written for, with 17 absences and no day columns at all.
    let grid = Sf2MonthDateRepo::new(pool.clone())
        .for_template(&template.id)
        .expect("grid");
    assert!(
        !grid.is_empty(),
        "a created month with no date mappings shows no weekday columns, and every write path \
         refuses to act on it - the month the user just created could not be recorded into"
    );
    let school_days = grid
        .iter()
        .filter(|mapping| parse_date(&mapping.date).is_some_and(is_school_day))
        .count();
    assert!(
        school_days >= 20,
        "October 2026 has 21 school days from the 3rd; the grid holds {school_days}"
    );
    for mapping in &grid {
        assert_eq!(
            mapping.template_id, template.id,
            "a date mapping must belong to the month it was created for"
        );
        assert!(
            mapping.date.starts_with("2026-10-"),
            "October's grid must hold October dates, got {}",
            mapping.date
        );
        assert_eq!(
            mapping.resolved_sheet_name(),
            "OCTOBER 2026",
            "every day names the worksheet it is written to, or a write has nothing to address"
        );
    }
}

#[test]
fn creating_a_month_never_copies_another_months_marks() {
    // A new month starts with zero marks. The database holds no October absences
    // in this test, and the one file has an October worksheet already - so if any
    // of October's marks surfaced on the new month the database could not produce
    // them, which is the failure the guard exists to stop.
    let pool = test_pool();
    seed_class(&pool);
    set_school_start_date(&pool, "2026-08-03");
    seed_student(&pool, STUDENT_ONE, "Juan Dela Cruz");
    seed_month(&pool, "month-october", "OCTOBER", 2026);
    seed_roster(
        &pool,
        "month-october",
        &[(STUDENT_ONE, "DELA CRUZ, JUAN", 8)],
    );
    seed_absent(&pool, STUDENT_ONE, "2026-10-05");
    let (_dir, workbooks) = test_workbook_dir();
    seed_workbook_on_disk(&pool, &workbooks);

    create_month_worksheet_in_dir(&pool, &workbooks, Some(CLASS_ID), "NOVEMBER")
        .expect("create November");

    // October's absence does not surface as a November one.
    let preview = read_month(&pool, &workbooks, "NOVEMBER");
    assert_eq!(
        preview.absence_count, 0,
        "October's absence must not surface as a November one"
    );
    // And the one workbook is still the one workbook: adding a month did not
    // create a second file and did not remove the first.
    assert!(single_workbook_path_in(&workbooks).is_file());
    assert!(
        !workbooks.join("SF2-NOVEMBER-2026.xls").exists(),
        "a month has no file of its own under Â§0 A1"
    );
}

#[test]
fn creating_a_month_refuses_a_month_with_no_school_days() {
    // Edge case E2, on the write side: refuse rather than make a worksheet with no
    // day to record. With no start date set, no month can be dated at all.
    let pool = test_pool();
    seed_class(&pool);
    let (_dir, workbooks) = test_workbook_dir();

    let error = create_month_worksheet_in_dir(&pool, &workbooks, Some(CLASS_ID), "APRIL")
        .expect_err("an undatable month must not get a worksheet");

    assert_eq!(
        error.to_string(),
        format!("invalid input: {NO_SCHOOL_DAYS_MESSAGE}"),
    );
}

#[test]
fn creating_a_month_refuses_a_month_that_is_already_on_record() {
    // The refusal that used to be "the file already exists on disk" is now "the
    // month already has a row", because the file is shared: overwriting it to
    // create a month would rewrite the eleven months already on it.
    let pool = test_pool();
    seed_class(&pool);
    set_school_start_date(&pool, "2026-08-03");
    seed_month(&pool, "month-october", "OCTOBER", 2026);
    let (_dir, workbooks) = test_workbook_dir();
    seed_workbook_on_disk(&pool, &workbooks);

    let error = create_month_worksheet_in_dir(&pool, &workbooks, Some(CLASS_ID), "OCTOBER")
        .expect_err("a month already on record must not get a second row");

    assert!(
        error.to_string().contains("already on record"),
        "unexpected message: {error}"
    );
}

// ── The school-year walk the create and the fallback share ─────────────────

#[test]
fn the_previous_school_month_walks_the_school_year_not_the_calendar() {
    // A roster is carried over from the month classes actually met before, which
    // for OCTOBER is SEPTEMBER, and for SEPTEMBER there is nothing.
    assert_eq!(
        previous_school_month(SCHOOL_YEAR, 10),
        Some(("SEPTEMBER".to_string(), 2026))
    );
    assert_eq!(
        previous_school_month(SCHOOL_YEAR, 3),
        Some(("FEBRUARY".to_string(), 2027))
    );
    assert_eq!(previous_school_month(SCHOOL_YEAR, 9), None);
}

// ── The pre-split fallback: the reported bug ───────────────────────────────
//
// The five tests below are the ones this module was changed for. They all read
// the same way: seed a database whose data lives in the pre-split tables, read a
// month, and assert on what the grid would show and whether its cells can be
// clicked.

/// Is every cell of this student's row one the grid would let the user click?
///
/// The frontend's condition is `disabled={!cell.editable || !row.mapped}`, so
/// "clickable" is exactly `every(cell.editable)` over a mapped row. Asserting it
/// here rather than in the component is deliberate: the flag is computed by the
/// shared preview builder, and the component is a pure function of it.
fn all_cells_clickable(preview: &Sf2MonthGridPreview, student_id: &str) -> bool {
    let Some(row) = preview
        .students
        .iter()
        .find(|row| row.student_id == student_id)
    else {
        return false;
    };
    row.mapped && row.cells.iter().all(|cell| cell.editable)
}

fn cell_status(preview: &Sf2MonthGridPreview, student_id: &str, date: &str) -> String {
    let row = preview
        .students
        .iter()
        .find(|row| row.student_id == student_id)
        .unwrap_or_else(|| panic!("{student_id} has no row in the grid"));
    let cell = row
        .cells
        .iter()
        .find(|cell| cell.date == date)
        .unwrap_or_else(|| panic!("{date} is not a column in the grid"));
    format!("{:?}", cell.status).to_lowercase()
}

#[test]
fn a_month_with_its_marks_only_in_the_legacy_tables_renders_them_and_is_clickable() {
    // The reported bug, end to end. Nothing is in `sf2_month_templates`,
    // `sf2_month_date_mappings` or `sf2_month_student_mappings`; the roster and
    // the day's absence are in the pre-split tables, and `events` holds the
    // absence.
    //
    // Before the fallback, the empty roster meant every learner was pushed as an
    // *unmapped* row, and the preview builder hard-codes an unmapped row's cells
    // to `Present`. So the grid showed nobody absent while the database held an
    // absence, and `editable: false` disabled every cell. Both halves of the
    // report, from one cause.
    let pool = test_pool();
    seed_class(&pool);
    seed_legacy_template(&pool, "SEPTEMBER");
    seed_legacy_roster(
        &pool,
        &[
            (STUDENT_ONE, "DELA CRUZ, JUAN", 8),
            (STUDENT_TWO, "SANTOS, MARIA", 9),
        ],
    );
    seed_legacy_grid(&pool, &[("2026-09-01", 6), ("2026-09-02", 8)]);
    seed_absent(&pool, STUDENT_ONE, "2026-09-02");
    let (_dir, workbooks) = test_workbook_dir();

    let preview = read_month(&pool, &workbooks, "SEPTEMBER");

    assert!(
        preview.uses_legacy_mappings,
        "this grid is being served from the pre-split tables, and must say so"
    );
    assert_eq!(
        preview.mapped_students, 2,
        "the pre-split roster is the class roster"
    );
    assert_eq!(preview.unmapped_student_count, 0);
    assert_eq!(
        cell_status(&preview, STUDENT_ONE, "2026-09-02"),
        "absent",
        "the X the database holds has to appear in the grid"
    );
    assert_eq!(preview.absence_count, 1);
    assert_eq!(preview.absent_list.len(), 1);
    assert_eq!(preview.absent_list[0].date, "2026-09-02");

    for student in [STUDENT_ONE, STUDENT_TWO] {
        assert!(
            all_cells_clickable(&preview, student),
            "{student} must be a mapped row with every cell editable"
        );
    }

    // And the columns are the real ones, not blanks: the fallback reads the
    // day-number grid as well as the roster.
    let mapped = preview
        .dates
        .iter()
        .find(|date| date.date == "2026-09-02")
        .expect("2 September is listed");
    assert_eq!(mapped.column_letter, "H");
    assert!(!preview.grid_empty);

    // The identity panel is fed from the pre-split row, so Open SF2 and Sync
    // roster stay available for a month whose data is on screen - but the month
    // named is the month being read, never the pre-split row's own.
    let template = preview
        .template
        .as_ref()
        .expect("the pre-split row describes the workbook");
    assert_eq!(template.report_month, "SEPTEMBER");
    assert_eq!(template.school_name, "Espiritu Elementary");
}

#[test]
fn two_months_with_different_marks_render_differently() {
    // Spec §0 A1: one file, twelve month sheets, **each carrying its own X
    // marks**. So a month filter that is silently ignored anywhere in the chain
    // shows the same grid twice over - which is the reported bug wearing a
    // different hat. Both months here are served from the pre-split tables, so
    // this also pins the fallback's per-month behaviour.
    let pool = test_pool();
    seed_class(&pool);
    seed_legacy_template(&pool, "SEPTEMBER");
    seed_legacy_roster(&pool, &[(STUDENT_ONE, "DELA CRUZ, JUAN", 8)]);
    seed_legacy_grid(
        &pool,
        &[
            ("2026-09-01", 6),
            ("2026-09-02", 8),
            ("2026-10-01", 6),
            ("2026-10-02", 8),
        ],
    );
    seed_absent(&pool, STUDENT_ONE, "2026-09-02");
    seed_absent(&pool, STUDENT_ONE, "2026-10-01");
    let (_dir, workbooks) = test_workbook_dir();

    let september = read_month(&pool, &workbooks, "SEPTEMBER");
    let october = read_month(&pool, &workbooks, "OCTOBER");

    assert_ne!(
        september.month, october.month,
        "the read must report the month it was asked for"
    );

    // The marks are in different places, and the grid has to agree. Asserted on
    // the absent list rather than on a cell lookup, because the two months have
    // no day in common: asking September's grid about 1 October is asking it for
    // a column it does not have, which is the scoping being tested in the first
    // place.
    assert_eq!(cell_status(&september, STUDENT_ONE, "2026-09-02"), "absent");
    assert_eq!(
        september
            .absent_list
            .iter()
            .map(|a| a.date.as_str())
            .collect::<Vec<_>>(),
        vec!["2026-09-02"],
        "October's mark must not surface in September's grid"
    );
    assert_eq!(cell_status(&october, STUDENT_ONE, "2026-10-01"), "absent");
    assert_eq!(
        october
            .absent_list
            .iter()
            .map(|a| a.date.as_str())
            .collect::<Vec<_>>(),
        vec!["2026-10-01"],
        "September's mark must not surface in October's grid"
    );

    // And the columns are different days, not the same list twice.
    let september_columns = september
        .dates
        .iter()
        .map(|date| date.date.as_str())
        .collect::<Vec<_>>();
    let october_columns = october
        .dates
        .iter()
        .map(|date| date.date.as_str())
        .collect::<Vec<_>>();
    assert!(september_columns
        .iter()
        .all(|date| date.starts_with("2026-09-")));
    assert!(october_columns
        .iter()
        .all(|date| date.starts_with("2026-10-")));

    // Only the mapped day of each month carries a column; the other month's
    // mapped day is not even a column here.
    let october_first = october
        .dates
        .iter()
        .find(|date| date.date == "2026-10-01")
        .expect("1 October is listed");
    assert_eq!(october_first.column_letter, "F");
    let october_second = october
        .dates
        .iter()
        .find(|date| date.date == "2026-10-02")
        .expect("2 October is listed");
    assert_eq!(
        october_second.column_letter, "H",
        "October has its own grid, read from October's rows"
    );
}

#[test]
fn the_legacy_fallback_is_scoped_to_the_month_it_was_asked_for() {
    // The load-bearing test. `sf2_date_mappings` is keyed by a full `YYYY-MM-DD`
    // and has no month of its own, so a fallback that reads the whole table and
    // sorts it out afterwards is a fallback that will eventually hand September
    // October's columns. A grid wearing another month's columns is worse than an
    // empty grid: it looks authoritative and is wrong.
    let pool = test_pool();
    seed_class(&pool);
    seed_legacy_template(&pool, "OCTOBER");
    seed_legacy_roster(&pool, &[(STUDENT_ONE, "DELA CRUZ, JUAN", 8)]);
    // The whole school year is in the table, which is the point. June is 2027 -
    // a 2026-2027 school year runs SEPTEMBER 2026 to AUGUST 2027, and the
    // September rule is what says so.
    seed_legacy_grid(
        &pool,
        &[
            ("2026-09-01", 6),
            ("2026-09-02", 8),
            ("2026-10-01", 6),
            ("2026-10-02", 8),
            ("2026-12-01", 6),
            ("2027-06-01", 6),
        ],
    );
    let (_dir, workbooks) = test_workbook_dir();

    for (month, expected_prefix) in [
        ("SEPTEMBER", "2026-09-"),
        ("OCTOBER", "2026-10-"),
        ("JUNE", "2027-06-"),
        ("DECEMBER", "2026-12-"),
    ] {
        let preview = read_month(&pool, &workbooks, month);
        assert!(
            preview
                .dates
                .iter()
                .all(|date| date.date.starts_with(expected_prefix)),
            "{month}'s grid must hold only {expected_prefix} days, got {:?}",
            preview
                .dates
                .iter()
                .map(|date| date.date.as_str())
                .collect::<Vec<_>>()
        );
    }

    // Specifically: September's read is answered from September's rows and
    // October's are not visible in it, and the reverse.
    let september = read_month(&pool, &workbooks, "SEPTEMBER");
    let september_mapped = september
        .dates
        .iter()
        .filter(|date| !date.column_letter.is_empty())
        .map(|date| date.date.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        september_mapped,
        vec!["2026-09-01", "2026-09-02"],
        "September is mapped from September's rows only"
    );

    let october = read_month(&pool, &workbooks, "OCTOBER");
    let october_mapped = october
        .dates
        .iter()
        .filter(|date| !date.column_letter.is_empty())
        .map(|date| date.date.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        october_mapped,
        vec!["2026-10-01", "2026-10-02"],
        "October is mapped from October's rows only"
    );

    // A month the table has no rows for is honestly empty rather than borrowing
    // its neighbour's.
    let november = read_month(&pool, &workbooks, "NOVEMBER");
    assert!(
        november
            .dates
            .iter()
            .all(|date| date.column_letter.is_empty()),
        "November has no mappings anywhere, and must not wear another month's"
    );
    assert!(november.grid_empty);
}

#[test]
fn the_legacy_fallback_reaches_no_excel_and_emits_no_progress() {
    // Spec D9 and acceptance #8-#9, re-asserted for the code that was added on
    // top of it. The fallback is one more SQL read and nothing else; if a COM
    // call or a progress event ever appears here, a month switch stops being
    // instant and the p95 target goes with it.
    //
    // Narrowed to the fallback function and the repository methods it calls, so
    // this fails if the fallback itself regresses rather than duplicating the
    // whole-module check above.
    let source = MONTH_PREVIEW_SOURCE;
    let fallback = source
        .split_once("fn resolve_legacy_mappings(")
        .and_then(|(_, rest)| rest.split_once("fn month_range_bound("))
        .map(|(block, _)| block)
        .expect("the legacy fallback sits between resolve_legacy_mappings and month_range_bound");
    let code = code_only(fallback);

    for forbidden in [
        "run_excel_task",
        "with_workbook",
        "ExcelSession",
        "open::that",
        "configure_sf2_calendar",
        "write_metadata",
        "emit_sf2_progress",
        "sf2-progress",
        "set_report_month",
        // The read half of `month_preview` is already checked not to write, but
        // the fallback is the part that reaches for the pre-split repository, so
        // name its mutating methods here too: a fallback that upserts mappings
        // would be writing to the one set of tables this install's data lives in.
        "upsert_template_with_mappings",
        "update_template_with_mappings",
        "delete_date_mappings",
        "set_last_synced_at",
        "replace_for_template",
    ] {
        assert!(
            !code.contains(forbidden),
            "the legacy fallback must not mention `{forbidden}`: it is a second SQL read, and \
             anything else on this path is either Excel or a write to the tables the data lives in"
        );
    }

    // And it really is a read of the pre-split tables, not a copy into the
    // per-month ones. Asserting the *absence* of a write is the strongest form of
    // this; the read itself is pinned behaviourally by the tests above.
    assert!(
        code.contains("date_mappings_in_month"),
        "the fallback is expected to read the pre-split day-number grid, month-scoped"
    );
    assert!(
        code.contains("student_mappings_for_template"),
        "the fallback is expected to read the pre-split roster"
    );
}

#[test]
fn a_grid_correction_round_trips_through_the_database_and_back_into_the_grid() {
    // The click itself. Frontend: `onToggleAttendance` -> `toggle_sf2_preview_
    // attendance` -> `set_preview_attendance_lightweight` -> `events`, then the
    // month cache is dropped and `get_sf2_month_preview` is called again. So the
    // round trip is proven by: correct a mark, re-read the month, see the new
    // mark - against a month whose mappings came from the pre-split tables,
    // which is the case that was broken.
    let pool = test_pool();
    seed_class(&pool);
    seed_legacy_template(&pool, "SEPTEMBER");
    seed_legacy_roster(&pool, &[(STUDENT_ONE, "DELA CRUZ, JUAN", 8)]);
    seed_legacy_grid(&pool, &[("2026-09-01", 6), ("2026-09-02", 8)]);
    let (_dir, workbooks) = test_workbook_dir();

    // Before: 1 September is a mapped, clickable, unmarked cell.
    let before = read_month(&pool, &workbooks, "SEPTEMBER");
    assert!(all_cells_clickable(&before, STUDENT_ONE));
    assert_eq!(cell_status(&before, STUDENT_ONE, "2026-09-01"), "open");
    assert_eq!(before.absence_count, 0);

    // The click. No Excel, no workbook write: this is the command the grid calls.
    set_preview_attendance_lightweight(
        pool.clone(),
        CLASS_ID.to_string(),
        STUDENT_ONE.to_string(),
        "2026-09-01".to_string(),
        false,
    )
    .expect("correcting a mark must succeed on a month served from the pre-split tables");

    // After: the re-read shows it, and the grid can be corrected back.
    let after = read_month(&pool, &workbooks, "SEPTEMBER");
    assert_eq!(
        cell_status(&after, STUDENT_ONE, "2026-09-01"),
        "absent",
        "the correction has to be visible in the grid it was made from"
    );
    assert_eq!(after.absence_count, 1);
    assert_eq!(after.absent_list[0].date, "2026-09-01");
    assert!(all_cells_clickable(&after, STUDENT_ONE));

    // And back again, so the round trip is a round trip.
    set_preview_attendance_lightweight(
        pool.clone(),
        CLASS_ID.to_string(),
        STUDENT_ONE.to_string(),
        "2026-09-01".to_string(),
        true,
    )
    .expect("clearing a mark must succeed too");
    let cleared = read_month(&pool, &workbooks, "SEPTEMBER");
    assert_eq!(cleared.absence_count, 0);

    // The pre-split tables were not touched by any of it. They are read-only on
    // this path, and on the affected install they are the only copy of the
    // mappings.
    let conn = pool.get().expect("connection");
    let legacy_dates: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sf2_date_mappings WHERE template_id = ?1",
            params![LEGACY_TEMPLATE_ID],
            |row| row.get(0),
        )
        .expect("count the pre-split date mappings");
    let legacy_students: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sf2_student_mappings WHERE template_id = ?1",
            params![LEGACY_TEMPLATE_ID],
            |row| row.get(0),
        )
        .expect("count the pre-split student mappings");
    assert_eq!(
        legacy_dates, 2,
        "a grid correction must not touch the pre-split grid"
    );
    assert_eq!(
        legacy_students, 1,
        "a grid correction must not touch the pre-split roster"
    );
}

#[test]
fn a_grid_correction_works_on_an_install_with_no_pre_split_row() {
    // The same click, on an install that has been fully migrated and has no
    // `sf2_templates` row at all - the state the §0 A1 reversal leaves behind.
    //
    // It used to fail here with "No SF2 template imported for this class",
    // because the correction demanded a pre-split template row before it would
    // write anything. A cell that renders as clickable and then refuses the click
    // is a worse failure than a disabled cell, because the teacher is told the
    // correction was saved.
    let pool = test_pool();
    seed_class(&pool);
    seed_month(&pool, "month-september", "SEPTEMBER", 2026);
    seed_roster(
        &pool,
        "month-september",
        &[(STUDENT_ONE, "DELA CRUZ, JUAN", 8)],
    );
    seed_grid(&pool, "month-september", &[("2026-09-01", 6)]);
    let (_dir, workbooks) = test_workbook_dir();

    let conn = pool.get().expect("connection");
    let pre_split_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM sf2_templates", [], |row| row.get(0))
        .expect("count the pre-split templates");
    assert_eq!(pre_split_rows, 0, "this install has been fully migrated");

    set_preview_attendance_lightweight(
        pool.clone(),
        CLASS_ID.to_string(),
        STUDENT_ONE.to_string(),
        "2026-09-01".to_string(),
        false,
    )
    .expect("a correction must not require a pre-split template row");

    let after = read_month(&pool, &workbooks, "SEPTEMBER");
    assert_eq!(cell_status(&after, STUDENT_ONE, "2026-09-01"), "absent");
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn date(value: &str) -> NaiveDate {
    parse_date(value).expect("a real date")
}

fn touch(path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create the parent directory");
    }
    std::fs::write(path, b"workbook placeholder").expect("write the placeholder file");
}
