//! Tests for the startup self-heal (spec D6, D8, §8; acceptance #15).
//!
//! Three kinds, and none of them needs Excel.
//!
//! * **Structural.** The properties that are facts about the *call graph* rather
//!   than about a return value: the heal never writes to the workbook, it reaches
//!   Excel through the guard's single scanner and nothing else, the once-per-launch
//!   latch is claimed before any work, and `lib.rs` spawns the run without joining
//!   it. Reintroduce an Excel writer into `heal.rs` and these fail.
//! * **Decision.** The §8.2 count comparison table, the D5/D8 month resolution, and
//!   the reconciliation of the per-month records with the guard's legacy-shaped
//!   signature - all pure, all exercised directly.
//! * **Behavioural.** A real SQLite database, a real month row, real mappings, and
//!   the additive import driven with real `X` cells, so "an X becomes an absent
//!   event and an absent event is never removed" is asserted against the database
//!   rather than against a mock.

use super::*;
use crate::infrastructure::database::init_db;

// ── Source of the heal path, for the structural assertions ──────────────────

/// The self-heal module's own source.
const HEAL_SOURCE: &str = include_str!("../heal.rs");

/// The Tauri bootstrap, where the heal is started.
const LIB_SOURCE: &str = include_str!("../../lib.rs");

/// Drop `//` line comments so a structural assertion is about *code*, not about
/// prose that names a forbidden symbol. The module docs here deliberately do name
/// several of them, which is exactly why the stripping has to happen first.
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

/// Everything that could put a mark into, or take one out of, the `.xls`.
///
/// The heal is read-only on the workbook *and* additive-only on the database
/// (spec §8.2, §8.3). The first half of that is only checkable as a property of
/// the call graph, because a function that merely happens not to be called is
/// indistinguishable from one that is.
const FORBIDDEN_EXCEL_WRITES: [&str; 9] = [
    "write_template_marks_for_days",
    "differential_clear_marks",
    "set_sf2_mark",
    "write_marks",
    "sync_attendance_to_sf2_workbook",
    "export_workbook",
    "with_workbook",
    "run_excel_task",
    "ExcelSession",
];

#[test]
fn the_heal_never_writes_to_the_workbook() {
    let code = code_only(HEAL_SOURCE);
    for forbidden in FORBIDDEN_EXCEL_WRITES {
        assert!(
            !code.contains(forbidden),
            "the self-heal must not reach `{forbidden}`: it is read-only on the \
             workbook (spec §8.2), and recovering a mark must never be the thing \
             that costs one"
        );
    }
}

#[test]
fn the_heal_measures_through_the_guards_single_scanner() {
    // One scanner, not two. A second implementation of "which cells hold X" is
    // how "what we measured" and "what a write may clear" stop being the same
    // set, which is the failure §9.1 exists to prevent.
    let code = code_only(HEAL_SOURCE);
    assert!(
        code.contains("measure_workbook_marks"),
        "the self-heal must measure through the guard's `measure_workbook_marks`"
    );
    assert!(
        !code.contains("Application.Evaluate") && !code.contains("TEXTJOIN"),
        "the self-heal must not contain a second cell reader of its own"
    );
}

#[test]
fn the_heal_never_deletes_a_database_mark() {
    // `set_attendance_event_for_day` clears the learner-day before inserting, so
    // the whole additive property rests on two things: it is only reached for a
    // cell that holds an X *in the workbook*, and it is skipped when the database
    // already has that absence. Both are asserted here structurally and both are
    // asserted behaviourally below.
    let code = code_only(HEAL_SOURCE);
    assert!(
        !code.contains("DELETE FROM"),
        "the self-heal must contain no DELETE: a database mark the workbook does \
         not have is never touched (spec §8.3)"
    );
    for required in [
        "has_absent_event_for_day",
        "has_present_event_for_day",
        "AttendanceType::Absent",
        "SELF_HEAL_REASON",
    ] {
        assert!(
            code.contains(required),
            "the additive import is expected to go through `{required}`"
        );
    }
}

#[test]
fn the_heal_refuses_to_overrule_an_explicit_present() {
    // D1, and the reason §8.3's "additive only" needed a second guard. Without
    // `has_present_event_for_day` on the path, `set_attendance_event_for_day`
    // deletes the day's `present` on its way to inserting the absence - an
    // unattended process replacing a mark a person made. Asserted here as well as
    // behaviourally, because a guard that is merely *present in the file* is not
    // the same as one that is consulted before the write.
    let code = code_only(HEAL_SOURCE);
    let import = code
        .split_once("fn import_recovered_marks")
        .expect("the heal's only write path")
        .1;
    let guard = import
        .find("has_present_event_for_day")
        .expect("the heal must consult the present guard");
    let write = import
        .find("set_attendance_event_for_day(")
        .expect("the heal must still import absences");
    assert!(
        guard < write,
        "the present guard must be consulted before the delete-then-insert write, \
         or the write removes the present on its way to adding the absence"
    );
}

#[test]
fn the_heal_is_claimed_once_before_it_is_spawned() {
    // The latch is a process-wide `static`, so the only thing testable about the
    // production instance is *where* it is claimed: at the spawner, before the
    // run exists. Claiming it inside the run would let two starts both pass the
    // check and both open Excel.
    let spawner = HEAL_SOURCE
        .split_once("pub fn spawn_heal_at_startup")
        .and_then(|(_, rest)| {
            rest.split_once("\n/// ")
                .or_else(|| rest.split_once("\n// ──"))
                .map(|(block, _)| block)
        })
        .expect("the startup spawner is in this module");
    let claim = spawner
        .find("claim_heal_for_this_launch()")
        .expect("the spawner claims the launch's single run");
    let spawn = spawner
        .find("std::thread::spawn")
        .expect("the spawner spawns the run");
    assert!(claim < spawn, "the claim must precede the spawn");

    // The blocking run itself never claims: the latch is a *launch* property,
    // and the Tauri command below is an explicit, repeatable user-driven call.
    let run = HEAL_SOURCE
        .split_once("pub fn heal_month_on")
        .and_then(|(_, rest)| rest.split_once("\n/// ").map(|(block, _)| block))
        .expect("the injectable-clock entry point exists");
    assert!(
        !run.contains("claim_heal_for_this_launch"),
        "the run must not claim the launch latch; the spawner owns that"
    );
}

#[test]
fn startup_spawns_the_heal_and_does_not_join_it() {
    // The failure mode this exists to prevent: a COM pass over forty learners on
    // the UI thread, turning app launch into a multi-second hang - exactly the
    // slowness spec D9 removed.
    let setup = LIB_SOURCE
        .split_once(".setup(|app|")
        .and_then(|(_, rest)| rest.split_once("Ok(())").map(|(block, _)| block))
        .expect("the Tauri setup hook is in lib.rs");
    let code = code_only(setup);

    assert!(
        code.contains("sf2::heal::spawn_heal_at_startup"),
        "the setup hook must start the self-heal"
    );
    assert!(
        !code.contains(".join()"),
        "the setup hook must not join the heal: startup is not blocked on Excel"
    );
    assert!(
        !code.contains("heal_current_month_workbook("),
        "the setup hook must go through the spawner, not call the blocking run"
    );
}

// ── Fixtures ───────────────────────────────────────────────────────────────

const CLASS_ID: &str = "class-1";
const SCHOOL_YEAR: &str = "2026-2027";
const TEMPLATE_ID: &str = "11111111-1111-4111-8111-111111111111";
const SHEET: &str = "SEPTEMBER 2026";

const STUDENT_ONE: &str = "22222222-2222-4222-8222-222222222222";
const STUDENT_TWO: &str = "33333333-3333-4333-8333-333333333333";

fn student(row: u32, id: &str, name: &str) -> Sf2MonthStudentMapping {
    Sf2MonthStudentMapping {
        template_id: TEMPLATE_ID.to_string(),
        student_id: id.to_string(),
        workbook_name: name.to_string(),
        normalized_name: name.to_uppercase(),
        row_index: row,
        gender_block: Some("MALE".to_string()),
        sf2_learner_id: Some(format!("LRN-{row}")),
    }
}

fn date(column: &str, day: &str) -> Sf2MonthDateMapping {
    Sf2MonthDateMapping {
        template_id: TEMPLATE_ID.to_string(),
        date: format!("2026-09-{day}"),
        column_letter: column.to_string(),
        column_index: 0,
        // The worksheet the day is written to. One file holds twelve month
        // worksheets, so a column letter is not an address without it.
        sheet_name: Some("SEPTEMBER 2026".to_string()),
    }
}

fn cell(column: &str, row: u32) -> Sf2GridCell {
    Sf2GridCell {
        sheet_name: SHEET.to_string(),
        column_letter: column.to_string(),
        row_index: row,
    }
}

fn roster() -> Vec<Sf2MonthStudentMapping> {
    vec![
        student(8, STUDENT_ONE, "DELA CRUZ"),
        student(9, STUDENT_TWO, "SANTOS"),
    ]
}

fn dates() -> Vec<Sf2MonthDateMapping> {
    vec![date("F", "01"), date("G", "02"), date("H", "03")]
}

fn launch(has_template: bool, fell_back: bool) -> Sf2LaunchMonth {
    Sf2LaunchMonth {
        month: "SEPTEMBER".to_string(),
        report_year: 2026,
        school_year: SCHOOL_YEAR.to_string(),
        class_id: CLASS_ID.to_string(),
        file_name: "SF2-SEPTEMBER-2026.xls".to_string(),
        file_exists: true,
        has_template,
        has_school_days: true,
        today_month: "SEPTEMBER".to_string(),
        today_report_year: 2026,
        fell_back,
        can_create: false,
        today_can_create: false,
        needs_school_start_date: false,
        issues: Vec::new(),
    }
}

fn template() -> Sf2MonthTemplate {
    Sf2MonthTemplate {
        id: TEMPLATE_ID.to_string(),
        active_class_id: CLASS_ID.to_string(),
        school_year: SCHOOL_YEAR.to_string(),
        report_month: "SEPTEMBER".to_string(),
        report_year: 2026,
        source_path: "C:/nowhere/SF2-SEPTEMBER-2026.xls".to_string(),
        source_hash: String::new(),
        school_id: None,
        school_name: None,
        grade_level: None,
        section: None,
        adviser_name: None,
        school_head_name: None,
        first_school_day: 1,
        first_school_day_override: None,
        imported_at: 1,
        last_synced_at: Some(2),
        workbook_x_count: 0,
        workbook_scanned_at: None,
    }
}

// ── Decision: the §8.2 step-4 comparison table ──────────────────────────────

#[test]
fn a_database_ahead_of_the_workbook_does_nothing_at_all() {
    // "Nothing to do" means nothing to import. It must not mean "remove the
    // surplus": a database ahead of the file is the ordinary state after a
    // teacher marks someone present before the next export, and reconciling that
    // "away" is how real marks die.
    assert_eq!(decide_heal(13, 12), HealAction::NothingToDo);
    assert_eq!(decide_heal(1, 0), HealAction::NothingToDo);
}

#[test]
fn equal_counts_only_record_the_measurement() {
    assert_eq!(decide_heal(12, 12), HealAction::RecordOnly);
    assert_eq!(decide_heal(0, 0), HealAction::RecordOnly);
}

#[test]
fn a_workbook_ahead_of_the_database_imports_the_shortfall() {
    assert_eq!(decide_heal(0, 12), HealAction::Import { shortfall: 12 });
    assert_eq!(decide_heal(11, 12), HealAction::Import { shortfall: 1 });
}

#[test]
fn the_comparison_table_covers_every_pair_of_counts_in_one_sweep() {
    // Exhaustive over a small grid, so a change that inverts a branch cannot pass
    // by only the three named cases still holding.
    for db in 0..6usize {
        for workbook in 0..6usize {
            let expected = match db.cmp(&workbook) {
                std::cmp::Ordering::Greater => HealAction::NothingToDo,
                std::cmp::Ordering::Equal => HealAction::RecordOnly,
                std::cmp::Ordering::Less => HealAction::Import {
                    shortfall: workbook - db,
                },
            };
            assert_eq!(
                decide_heal(db, workbook),
                expected,
                "db={db} workbook={workbook}"
            );
        }
    }
}

// ── Decision: the month to heal (D5, D8) ────────────────────────────────────

#[test]
fn a_month_with_a_stored_row_is_the_heal_target() {
    let target = heal_target(&launch(true, false)).expect("the month has a row");

    assert_eq!(target.month, "SEPTEMBER");
    assert_eq!(target.report_year, 2026);
    assert_eq!(target.class_id, CLASS_ID);
    assert!(!target.fell_back);
}

#[test]
fn the_fallback_month_is_healed_and_says_so() {
    // Edge case E1: JUNE has no file, so the app shows MAY and the heal must
    // recover MAY - the month on screen, not the month on the calendar. Healing a
    // different month than the teacher is looking at is the one outcome that
    // turns a silent recovery into a support call.
    let mut june_missing = launch(true, true);
    june_missing.month = "MAY".to_string();
    june_missing.report_year = 2026;
    june_missing.fell_back = true;

    let target = heal_target(&june_missing).expect("the fallback month has a row");

    assert_eq!(target.month, "MAY");
    assert!(target.fell_back);
}

#[test]
fn a_month_with_no_stored_row_is_not_applicable_not_an_error() {
    // The state of every month before the split has run.
    assert_eq!(heal_target(&launch(false, false)), None);
}

// ── Decision: the once-per-launch latch (D8) ────────────────────────────────

#[test]
fn the_latch_hands_out_exactly_one_claim() {
    let latch = HealLatch::new();

    assert!(latch.try_claim(), "the first caller gets the run");
    assert!(!latch.try_claim(), "the second caller does not");
    assert!(!latch.try_claim());
}

#[test]
fn a_fresh_latch_starts_unclaimed() {
    assert!(HealLatch::new().try_claim());
    assert!(
        HealLatch::new().try_claim(),
        "a new latch is per-launch state"
    );
}

#[test]
fn only_one_of_many_racing_callers_claims_the_run() {
    use std::sync::Arc;

    let latch = Arc::new(HealLatch::new());
    let claims: Vec<bool> = (0..16)
        .map(|_| {
            let latch = Arc::clone(&latch);
            std::thread::spawn(move || latch.try_claim())
        })
        .map(|handle| handle.join().expect("the racing claim must not panic"))
        .collect();

    assert_eq!(
        claims.iter().filter(|claimed| **claimed).count(),
        1,
        "two startup runs would double-toast and double-import: {claims:?}"
    );
}

// ── Decision: reconciling the month records with the guard's signature ──────

#[test]
fn a_month_roster_becomes_guard_student_mappings_without_losing_the_identity() {
    let converted = month_students_as_guard_mappings(&roster());

    assert_eq!(converted.len(), 2);
    assert_eq!(converted[0].student_id, STUDENT_ONE);
    assert_eq!(converted[0].row_index, 8);
    assert_eq!(converted[0].workbook_name, "DELA CRUZ");
    assert_eq!(converted[0].normalized_name, "DELA CRUZ");
    assert_eq!(converted[0].template_id, TEMPLATE_ID);
    assert_eq!(converted[1].student_id, STUDENT_TWO);
    assert_eq!(converted[1].row_index, 9);
}

#[test]
fn a_month_grid_becomes_guard_date_mappings_with_the_derived_sheet_name() {
    // §6.2 dropped `sheet_name` from the table because a month file has one sheet.
    // The guard's signature predates that and still wants it spelled out, so it
    // is derived - and derived from the *stored* month, never the clock.
    let converted = month_dates_as_guard_mappings(&dates(), SHEET);

    assert_eq!(converted.len(), 3);
    for mapping in &converted {
        assert_eq!(mapping.sheet_name, SHEET);
    }
    assert_eq!(converted[0].date, "2026-09-01");
    assert_eq!(converted[0].column_letter, "F");
    assert_eq!(converted[2].date, "2026-09-03");
    assert_eq!(converted[2].column_letter, "H");
}

#[test]
fn the_sheet_name_comes_from_the_stored_month_row() {
    // The two year rules in this crate disagree about AUGUST, and the legacy June
    // rule would name a file the split did not write. Reading the row is what
    // keeps the scanner pointed at a worksheet that exists.
    assert_eq!(month_sheet_name(&template()), SHEET);

    let mut august = template();
    august.report_month = "AUGUST".to_string();
    august.report_year = 2027;
    assert_eq!(month_sheet_name(&august), "AUGUST 2027");
}

#[test]
fn converted_mappings_measure_the_same_scope_the_guard_measures() {
    // The reconciliation has to be lossless in the only direction that matters:
    // the cells the heal counts must be the cells a write is allowed to touch. So
    // the scope computed from the converted records has to equal the scope
    // computed from the originals.
    let students = month_students_as_guard_mappings(&roster());
    let guard_dates = month_dates_as_guard_mappings(&dates(), SHEET);

    let scope = crate::sf2::attendance_marks::attendance_scope_cells(&students, &guard_dates);

    assert_eq!(scope.len(), 6, "two learners x three days");
    assert!(scope.contains(&cell("F", 8)));
    assert!(scope.contains(&cell("H", 9)));
    assert!(
        !scope.contains(&cell("I", 8)),
        "I is not a mapped day column"
    );
    assert!(
        !scope.contains(&cell("F", 7)),
        "row 7 is not a mapped learner row"
    );
}

// ── Decision: measured X cells → (student, day) ─────────────────────────────

#[test]
fn a_measured_x_resolves_to_the_learner_and_the_day() {
    let resolved = resolve_x_cells(SHEET, &[cell("G", 9)], &roster(), &dates());

    assert_eq!(
        resolved,
        vec![HealedAbsence {
            student_id: STUDENT_TWO.to_string(),
            date: "2026-09-02".to_string(),
        }]
    );
}

#[test]
fn an_x_the_mappings_cannot_place_is_dropped_rather_than_guessed() {
    // An X on a row with no mapping has no student id to write an event for; an
    // X on a column with no mapping has no date. Attributing either to the wrong
    // learner would be a wrong absence nobody asked for.
    let unplaceable = vec![
        cell("F", 99), // no learner row
        cell("ZZ", 8), // no day column
        cell("G", 9),  // placeable
    ];

    let resolved = resolve_x_cells(SHEET, &unplaceable, &roster(), &dates());

    assert_eq!(
        resolved,
        vec![HealedAbsence {
            student_id: STUDENT_TWO.to_string(),
            date: "2026-09-02".to_string(),
        }]
    );
}

#[test]
fn an_x_on_another_sheet_is_not_this_month() {
    let mut elsewhere = cell("G", 9);
    elsewhere.sheet_name = "OCTOBER 2026".to_string();

    assert!(resolve_x_cells(SHEET, &[elsewhere], &roster(), &dates()).is_empty());
}

#[test]
fn an_unmapped_row_zero_never_claims_a_cell() {
    // Row 0 is the header band, and `mapped_attendance_rows` drops it upstream.
    // The lookup drops it too, so a header `X` cannot be attributed to a learner.
    let mut with_row_zero = roster();
    with_row_zero.push(student(0, STUDENT_ONE, "HEADER"));

    assert!(resolve_x_cells(SHEET, &[cell("F", 0)], &with_row_zero, &dates()).is_empty());
}

// ── The outcome ────────────────────────────────────────────────────────────

#[test]
fn only_a_recovery_is_worth_a_toast() {
    // Every other variant is either expected (no row, already ran) or a condition
    // §9.1 already handles on the next Open/Export. A toast for each would train
    // a teacher to dismiss messages from this app.
    let recovered = Sf2HealOutcome::Recovered {
        month: "SEPTEMBER".to_string(),
        file_name: "SF2-SEPTEMBER-2026.xls".to_string(),
        imported: 12,
        already_recorded: 0,
        skipped_present: 0,
        workbook_count: 12,
        db_count_before: 0,
        db_count_after: 12,
        scanned_at: 100,
        toast: recovered_toast("SF2-SEPTEMBER-2026.xls", 12),
    };
    assert_eq!(
        recovered.toast(),
        Some("Recovered 12 X marks from SF2-SEPTEMBER-2026.xls")
    );

    for quiet in [
        Sf2HealOutcome::AlreadyRan,
        Sf2HealOutcome::NotApplicable {
            reason: "no row".to_string(),
        },
        Sf2HealOutcome::ExcelUnavailable {
            reason: "no Excel".to_string(),
        },
        Sf2HealOutcome::WorkbookMissing {
            reason: "gone".to_string(),
        },
        Sf2HealOutcome::UpToDate {
            month: "SEPTEMBER".to_string(),
            db_count: 13,
            workbook_count: 12,
            scanned_at: 1,
        },
        Sf2HealOutcome::InSync {
            month: "SEPTEMBER".to_string(),
            db_count: 12,
            workbook_count: 12,
            scanned_at: 1,
        },
    ] {
        assert_eq!(quiet.toast(), None, "{quiet:?} must stay silent");
    }
}

#[test]
fn the_outcome_serialises_with_a_status_tag() {
    // The frontend reads `outcome.status` to decide whether to show anything, and
    // the payload has to be self-describing enough that a future variant cannot
    // be mistaken for a recovery.
    let json = serde_json::to_value(Sf2HealOutcome::AlreadyRan).expect("serialises");
    assert_eq!(json["status"], "alreadyRan");

    let json = serde_json::to_value(Sf2HealOutcome::InSync {
        month: "SEPTEMBER".to_string(),
        db_count: 12,
        workbook_count: 12,
        scanned_at: 99,
    })
    .expect("serialises");
    assert_eq!(json["status"], "inSync");
    assert_eq!(json["month"], "SEPTEMBER");
    assert_eq!(json["workbookCount"], 12);
    assert_eq!(json["scannedAt"], 99);
}

// ── Decision: which measured X cells the database cannot produce ────────────

#[test]
fn only_the_x_cells_the_database_cannot_produce_are_missing() {
    let workbook = vec![cell("F", 8), cell("G", 8), cell("H", 9)];
    let database = vec![cell("F", 8)];

    let missing = missing_x_cells(&workbook, &database);

    assert_eq!(missing, vec![&cell("G", 8), &cell("H", 9)]);
    assert_eq!(missing.len(), 2);
}

#[test]
fn a_database_holding_every_cell_leaves_nothing_to_recover() {
    let cells = vec![cell("F", 8), cell("G", 9)];

    assert!(missing_x_cells(&cells, &cells).is_empty());
    assert!(
        missing_x_cells(&cells, &[]).len() == 2,
        "an empty database means every X is missing"
    );
}

#[test]
fn the_same_count_on_different_days_still_leaves_the_right_days_missing() {
    // The case that makes the set difference worth having: three X in the file,
    // three in the database, and not one of them on the same day. The count table
    // says "in sync" and nothing is imported - which is §8.2 step 4's rule, and it
    // is safe only because §9.1's guard still refuses the write on set membership.
    // Reversed, the arithmetic has to come out the other way.
    let workbook = vec![cell("F", 8), cell("G", 8), cell("H", 8)];
    let database = vec![cell("F", 9), cell("G", 9), cell("H", 9)];
    assert_eq!(workbook.len(), database.len());

    let missing = missing_x_cells(&workbook, &database);
    assert_eq!(
        missing.len(),
        3,
        "no day overlaps, so all three are missing"
    );
    assert!(missing.iter().all(|held| held.row_index == 8));
}

// ── Behavioural: the additive import, against a real database ───────────────

fn test_pool() -> DbPool {
    let dir = tempfile::tempdir().expect("temp dir for the test database");
    let pool = init_db(dir.path().join("attendance.db")).expect("migrate test database");
    std::mem::forget(dir);
    pool
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
    conn.execute(
        "INSERT OR IGNORE INTO students (id, name, class_id, created_at) VALUES (?1, ?2, ?3, 4)",
        rusqlite::params![id, name, CLASS_ID],
    )
    .expect("insert student");
}

fn absent_event_count(pool: &DbPool, student_id: &str) -> usize {
    use rusqlite::params;
    let conn = pool.get().expect("connection");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE student_id = ?1 AND event_type = 'absent'",
            params![student_id],
            |row| row.get(0),
        )
        .expect("count absences");
    count as usize
}

fn present_event_count(pool: &DbPool, student_id: &str) -> usize {
    use rusqlite::params;
    let conn = pool.get().expect("connection");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE student_id = ?1 AND event_type = 'in'",
            params![student_id],
            |row| row.get(0),
        )
        .expect("count presences");
    count as usize
}

/// Record an explicit `present` the way an interactive toggle would: through
/// `set_attendance_event_for_day`, not by hand-inserting a row, so the guard is
/// tested against the shape the app really writes.
fn record_present(pool: &DbPool, student_id: &str, date: &str, day_start: &str) {
    set_attendance_event_for_day(
        pool.clone(),
        student_id,
        CLASS_ID,
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("a date"),
        day_start,
        AttendanceType::In,
        "test present",
    )
    .expect("record a present");
}

fn seeded_pool() -> (DbPool, String) {
    let pool = test_pool();
    seed_class(&pool);
    for mapping in roster() {
        seed_student(&pool, &mapping.student_id, &mapping.workbook_name);
    }
    let day_start = class_day_start(&pool, CLASS_ID).expect("the class has a day start");
    (pool, day_start)
}

#[test]
fn an_x_becomes_an_absent_event() {
    let (pool, day_start) = seeded_pool();
    let absences = vec![HealedAbsence {
        student_id: STUDENT_ONE.to_string(),
        date: "2026-09-01".to_string(),
    }];

    let recovered =
        import_recovered_marks(&pool, &template(), &day_start, &absences).expect("import runs");

    assert_eq!(recovered.imported, 1);
    assert_eq!(recovered.already_recorded, 0);
    assert_eq!(recovered.skipped_present, 0);
    assert_eq!(absent_event_count(&pool, STUDENT_ONE), 1);
}

#[test]
fn the_import_is_idempotent_rather_than_rewriting_the_same_absence() {
    // Re-running the heal must be a no-op, and must not re-audit-log the same
    // absence: `has_absent_event_for_day` is the only thing standing between a
    // second launch and a duplicate event per learner-day.
    let (pool, day_start) = seeded_pool();
    let absences = vec![HealedAbsence {
        student_id: STUDENT_ONE.to_string(),
        date: "2026-09-01".to_string(),
    }];

    let first = import_recovered_marks(&pool, &template(), &day_start, &absences)
        .expect("first import runs");
    let second = import_recovered_marks(&pool, &template(), &day_start, &absences)
        .expect("second import runs");

    assert_eq!(first.imported, 1);
    assert_eq!(first.already_recorded, 0);
    assert_eq!(second.imported, 0);
    assert_eq!(second.already_recorded, 1);
    assert_eq!(absent_event_count(&pool, STUDENT_ONE), 1);
}

#[test]
fn the_import_never_removes_a_database_mark_the_workbook_does_not_have() {
    // The load-bearing property of §8.3, asserted against the database: an
    // absence on 02 September, imported from a workbook, survives a heal that
    // only knows about an X on 01 September.
    let (pool, day_start) = seeded_pool();
    let kept = HealedAbsence {
        student_id: STUDENT_ONE.to_string(),
        date: "2026-09-02".to_string(),
    };
    import_recovered_marks(&pool, &template(), &day_start, std::slice::from_ref(&kept))
        .expect("seed the mark");

    let healed = HealedAbsence {
        student_id: STUDENT_ONE.to_string(),
        date: "2026-09-01".to_string(),
    };
    import_recovered_marks(&pool, &template(), &day_start, &[healed]).expect("the heal runs");

    let conn = pool.get().expect("connection");
    let dates: Vec<String> = {
        let mut statement = conn
            .prepare(
                "SELECT date(timestamp, 'unixepoch', 'localtime') AS d
                     FROM events WHERE student_id = ?1 AND event_type = 'absent' ORDER BY d",
            )
            .expect("prepare");
        let rows = statement
            .query_map(rusqlite::params![STUDENT_ONE], |row| row.get(0))
            .expect("query");
        rows.collect::<rusqlite::Result<Vec<String>>>()
            .expect("collect")
    };
    assert_eq!(dates, vec!["2026-09-01", "2026-09-02"]);
}

#[test]
fn the_import_leaves_other_learners_and_other_days_alone() {
    let (pool, day_start) = seeded_pool();
    let absences = vec![HealedAbsence {
        student_id: STUDENT_ONE.to_string(),
        date: "2026-09-01".to_string(),
    }];

    import_recovered_marks(&pool, &template(), &day_start, &absences).expect("import runs");

    assert_eq!(absent_event_count(&pool, STUDENT_ONE), 1);
    assert_eq!(
        absent_event_count(&pool, STUDENT_TWO),
        0,
        "the heal must not touch a learner the workbook said nothing about"
    );
}

#[test]
fn every_recovered_absence_is_audited_as_a_self_heal() {
    // The one write this module makes happens with nobody watching, so the trail
    // has to say which flow wrote it. Anything else is a record the teacher
    // cannot audit.
    let (pool, day_start) = seeded_pool();
    import_recovered_marks(
        &pool,
        &template(),
        &day_start,
        &[HealedAbsence {
            student_id: STUDENT_ONE.to_string(),
            date: "2026-09-01".to_string(),
        }],
    )
    .expect("import runs");

    let conn = pool.get().expect("connection");
    let reasons: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE override_reason = ?1",
            rusqlite::params![SELF_HEAL_REASON],
            |row| row.get(0),
        )
        .expect("count self-heal events");
    assert_eq!(reasons, 1);
}

#[test]
fn the_self_heal_reason_is_the_specs_own_wording() {
    // §8.2 step 4: the reason recorded on an unattended recovery is spelled out
    // there, and it differs from the manual import's - which is the point. An
    // audit trail that cannot tell the two apart cannot answer "who added this?".
    assert_eq!(SELF_HEAL_REASON, "SF2 workbook self-heal");
}

// ── Behavioural: the heal never overrules an explicit `present` (D1) ─────────

#[test]
fn an_x_over_a_day_the_database_calls_present_leaves_the_present_alone() {
    // The D1 defect. `set_attendance_event_for_day` deletes the learner-day before
    // inserting, so an `X` over a recorded `present` used to turn that mark into
    // an absence - an unattended process rewriting something a person explicitly
    // did. The `present` is the surviving record.
    let (pool, day_start) = seeded_pool();
    record_present(&pool, STUDENT_ONE, "2026-09-01", &day_start);
    assert_eq!(present_event_count(&pool, STUDENT_ONE), 1);

    let recovered = import_recovered_marks(
        &pool,
        &template(),
        &day_start,
        &[HealedAbsence {
            student_id: STUDENT_ONE.to_string(),
            date: "2026-09-01".to_string(),
        }],
    )
    .expect("the heal runs");

    assert_eq!(recovered.skipped_present, 1);
    assert_eq!(recovered.imported, 0);
    assert_eq!(recovered.already_recorded, 0);
    assert_eq!(
        present_event_count(&pool, STUDENT_ONE),
        1,
        "the explicit present must survive the heal"
    );
    assert_eq!(
        absent_event_count(&pool, STUDENT_ONE),
        0,
        "the heal must not invent an absence over a recorded present"
    );
}

#[test]
fn the_guard_does_not_cost_the_absences_the_heal_exists_to_recover() {
    // Both halves in one run, on one learner-day apart: the guard has to skip the
    // disputed day and still import the missing absence next to it. A guard that
    // simply turned the import off would pass the test above and fail this one.
    let (pool, day_start) = seeded_pool();
    record_present(&pool, STUDENT_ONE, "2026-09-01", &day_start);

    let recovered = import_recovered_marks(
        &pool,
        &template(),
        &day_start,
        &[
            HealedAbsence {
                student_id: STUDENT_ONE.to_string(),
                date: "2026-09-01".to_string(),
            },
            HealedAbsence {
                student_id: STUDENT_ONE.to_string(),
                date: "2026-09-02".to_string(),
            },
            HealedAbsence {
                student_id: STUDENT_TWO.to_string(),
                date: "2026-09-01".to_string(),
            },
        ],
    )
    .expect("the heal runs");

    assert_eq!(recovered.skipped_present, 1, "the disputed day is skipped");
    assert_eq!(recovered.imported, 2, "the other two absences are recorded");
    assert_eq!(absent_event_count(&pool, STUDENT_ONE), 1);
    assert_eq!(absent_event_count(&pool, STUDENT_TWO), 1);
    assert_eq!(present_event_count(&pool, STUDENT_ONE), 1);
}

#[test]
fn a_present_on_another_day_does_not_block_the_heal() {
    // The guard is scoped to the learner-*day*, not to the learner. A present on
    // 02 September says nothing about 01 September, and treating it as if it did
    // would quietly stop the heal from ever recovering a term.
    let (pool, day_start) = seeded_pool();
    record_present(&pool, STUDENT_ONE, "2026-09-02", &day_start);

    let recovered = import_recovered_marks(
        &pool,
        &template(),
        &day_start,
        &[HealedAbsence {
            student_id: STUDENT_ONE.to_string(),
            date: "2026-09-01".to_string(),
        }],
    )
    .expect("the heal runs");

    assert_eq!(recovered.imported, 1);
    assert_eq!(recovered.skipped_present, 0);
    assert_eq!(absent_event_count(&pool, STUDENT_ONE), 1);
    assert_eq!(present_event_count(&pool, STUDENT_ONE), 1);
}

#[test]
fn a_skipped_present_is_not_audited_as_a_recovery() {
    // Nothing was recovered, so nothing may appear in the trail as though it
    // had been. An audit entry for a mark the app deliberately did not write is
    // worse than no entry: it tells the teacher the app touched the record.
    let (pool, day_start) = seeded_pool();
    record_present(&pool, STUDENT_ONE, "2026-09-01", &day_start);

    import_recovered_marks(
        &pool,
        &template(),
        &day_start,
        &[HealedAbsence {
            student_id: STUDENT_ONE.to_string(),
            date: "2026-09-01".to_string(),
        }],
    )
    .expect("the heal runs");

    let conn = pool.get().expect("connection");
    let recovered: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE override_reason = ?1",
            rusqlite::params![SELF_HEAL_REASON],
            |row| row.get(0),
        )
        .expect("count self-heal events");
    assert_eq!(recovered, 0);
}

#[test]
fn a_disputed_cell_still_counts_as_a_missing_x_next_launch() {
    // The heal is conservative, not done. Skipping leaves the workbook reading
    // ahead of the database, so the next launch measures the same shortfall and
    // reports it again - which is the honest state, and the reason the skip is
    // counted rather than quietly absorbed.
    let (pool, day_start) = seeded_pool();
    record_present(&pool, STUDENT_ONE, "2026-09-01", &day_start);
    let absence = HealedAbsence {
        student_id: STUDENT_ONE.to_string(),
        date: "2026-09-01".to_string(),
    };

    let first = import_recovered_marks(
        &pool,
        &template(),
        &day_start,
        std::slice::from_ref(&absence),
    )
    .expect("first launch");
    let second =
        import_recovered_marks(&pool, &template(), &day_start, &[absence]).expect("second launch");

    assert_eq!(first.skipped_present, 1);
    assert_eq!(second.skipped_present, 1);
    assert_eq!(second.imported, 0);
}

#[test]
fn the_outcome_reports_the_skipped_presents_it_declined_to_write() {
    // The count is the only way anyone finds out the heal met a disagreement and
    // left it alone, so it has to survive into the payload the frontend reads.
    let json = serde_json::to_value(Sf2HealOutcome::Recovered {
        month: "SEPTEMBER".to_string(),
        file_name: "SF2-SEPTEMBER-2026.xls".to_string(),
        imported: 4,
        already_recorded: 1,
        skipped_present: 2,
        workbook_count: 7,
        db_count_before: 0,
        db_count_after: 4,
        scanned_at: 5,
        toast: recovered_toast("SF2-SEPTEMBER-2026.xls", 4),
    })
    .expect("serialises");

    assert_eq!(json["status"], "recovered");
    assert_eq!(json["imported"], 4);
    assert_eq!(json["skippedPresent"], 2);
}

// ── Behavioural: what a measured scan does to the month row ─────────────────

#[test]
fn recording_a_measurement_is_what_makes_a_month_counted_rather_than_empty() {
    // §12.2's status line and §9.1's `Unmeasured` default both turn on this: a
    // month whose file was never counted is *unmeasured*, not empty, and nothing
    // may be cleared on its word.
    let (pool, _) = seeded_pool();
    let repo = Sf2MonthTemplateRepo::new(pool.clone());
    repo.upsert(&template()).expect("store the month row");
    assert!(!template().is_workbook_measured());

    repo.record_workbook_x_count(TEMPLATE_ID, 12, 1_700_000_000)
        .expect("record the measurement");

    let stored = repo.find_by_id(TEMPLATE_ID).expect("read").expect("row");
    assert_eq!(stored.workbook_x_count, 12);
    assert_eq!(stored.workbook_scanned_at, Some(1_700_000_000));
    assert!(stored.is_workbook_measured());
}

#[test]
fn recording_a_measurement_never_touches_the_workbook_path() {
    // "Persist the measured workbook_x_count" is a row write. If it ever moved a
    // file, the heal would no longer be read-only on the workbook.
    let (pool, _) = seeded_pool();
    let repo = Sf2MonthTemplateRepo::new(pool.clone());
    repo.upsert(&template()).expect("store the month row");

    repo.record_workbook_x_count(TEMPLATE_ID, 0, 1)
        .expect("record the measurement");

    let stored = repo.find_by_id(TEMPLATE_ID).expect("read").expect("row");
    assert_eq!(
        stored.source_path,
        template().source_path,
        "the measurement must not rewrite where the month file is"
    );
    assert_eq!(
        stored.last_synced_at,
        template().last_synced_at,
        "recording a measurement is not a sync and must not mark one"
    );
}
