use super::*;
use crate::infrastructure::database::DbPool;

// ── unmapped_roster_issue ─────────────────────────────────────────────

#[test]
fn unmapped_roster_issue_single_student() {
    let names = vec!["Juan".to_string()];
    let msg = unmapped_roster_issue(&names);
    assert!(msg.contains("Juan"));
    assert!(msg.contains("is"));
    assert!(msg.contains("not mapped"));
}

#[test]
fn unmapped_roster_issue_two_students() {
    let names = vec!["Juan".to_string(), "Maria".to_string()];
    let msg = unmapped_roster_issue(&names);
    assert!(msg.contains("Juan"));
    assert!(msg.contains("Maria"));
    assert!(msg.contains("are"));
    assert!(msg.contains("not mapped"));
}

#[test]
fn unmapped_roster_issue_shows_first_five() {
    let names = (1..=7).map(|i| format!("Student{i}")).collect::<Vec<_>>();
    let msg = unmapped_roster_issue(&names);
    assert!(msg.contains("Student1"));
    assert!(msg.contains("Student5"));
    assert!(msg.contains(", and 2 more"));
    assert!(msg.contains("are"));
}

#[test]
fn unmapped_roster_issue_exactly_five() {
    let names = (1..=5).map(|i| format!("Student{i}")).collect::<Vec<_>>();
    let msg = unmapped_roster_issue(&names);
    assert!(msg.contains("Student5"));
    assert!(!msg.contains("more"), "should not have 'more' suffix");
    assert!(msg.contains("are"));
}

#[test]
fn unmapped_roster_issue_zero_students() {
    let names: Vec<String> = vec![];
    let msg = unmapped_roster_issue(&names);
    assert!(msg.starts_with(" are"));
    assert!(msg.contains("not mapped"));
}

#[test]
fn unmapped_roster_issue_exactly_one_more_after_five() {
    let names = (1..=6).map(|i| format!("Student{i}")).collect::<Vec<_>>();
    let msg = unmapped_roster_issue(&names);
    assert!(msg.contains(", and 1 more"));
}

// ── acceptance #2: the export evaluates the destructive-sync guard ───

/// This file, as source. The behavioural tests below prove what the guard
/// *decides*; this proves the decision is reached in `export_workbook` at all,
/// above every write and above the copy. `export_workbook` needs a live
/// `AppHandle` for the save dialog, so its wiring cannot be exercised directly -
/// and wiring is exactly the thing that was missing.
const EXCEL_SERVICE_SOURCE: &str = include_str!("../excel/excel_service.rs");

fn test_pool() -> DbPool {
    let dir = tempfile::tempdir().expect("temp dir for the test database");
    let pool = crate::infrastructure::database::init_db(dir.path().join("attendance.db"))
        .expect("migrate test database");
    std::mem::forget(dir);
    pool
}

fn template(source_path: &str) -> Sf2TemplateRecord {
    Sf2TemplateRecord {
        id: "template-1".to_string(),
        source_path: source_path.to_string(),
        source_hash: String::new(),
        school_id: String::new(),
        school_name: String::new(),
        school_year: "2026-2027".to_string(),
        report_month: "SEPTEMBER".to_string(),
        grade_level: String::new(),
        section: String::new(),
        adviser_name: String::new(),
        school_head_name: String::new(),
        layout_fingerprint: String::new(),
        active_class_id: "class-1".to_string(),
        imported_at: 1,
        last_synced_at: None,
    }
}

fn date_mapping(date: &str, column: &str) -> Sf2DateMappingRecord {
    Sf2DateMappingRecord {
        template_id: "template-1".to_string(),
        sheet_name: "SEPTEMBER 2026".to_string(),
        date: date.to_string(),
        column_letter: column.to_string(),
        column_index: 6,
    }
}

/// One mapped learner. The guard refuses a month it cannot measure, and "no
/// mapped learners" is checked before the file is even looked at, so a test that
/// wants to reach the missing-file branch has to supply a roster.
fn student_mapping() -> Sf2StudentMappingRecord {
    Sf2StudentMappingRecord {
        template_id: "template-1".to_string(),
        student_id: "11111111-1111-4111-8111-111111111111".to_string(),
        workbook_name: "Dela Cruz, Juan".to_string(),
        normalized_name: "DELA CRUZ JUAN".to_string(),
        row_index: 8,
        gender_block: Some("MALE".to_string()),
    }
}

#[test]
fn the_export_refuses_a_month_it_cannot_measure() {
    // Degenerate analysis: the month's file is gone (E4) or was never dated.
    // There is no evidence, so the export must not write - and it must not
    // report "nothing to do" either, which is the reading that produced an empty
    // workbook in the first place.
    let pool = test_pool();
    let missing = template("Z:/no-such-dir/SF2-SEPTEMBER-2026.xls");
    let dates = vec![date_mapping("2026-09-01", "F")];

    let outcome = export_guard_outcome(&pool, &missing, &[student_mapping()], &dates);

    match outcome {
        ExportGuardOutcome::ReadOnly(reason) => assert!(
            reason.contains("SF2-SEPTEMBER.xls is missing"),
            "the reason has to name the missing file so the user knows what to restore: {reason}"
        ),
        ExportGuardOutcome::Write => panic!(
            "the export wrote a workbook it could not measure; that is spec acceptance #2 \
             failing"
        ),
        ExportGuardOutcome::Refuse(message) => {
            panic!("a missing file is `Unmeasured`, not `Stale`; got: {message}")
        }
    }
}

#[test]
fn the_export_refuses_a_month_with_no_mapped_dates() {
    let pool = test_pool();
    let outcome = export_guard_outcome(
        &pool,
        &template("Z:/no-such-dir/SF2-SEPTEMBER-2026.xls"),
        &[],
        &[],
    );

    assert!(
        matches!(outcome, ExportGuardOutcome::ReadOnly(_)),
        "an undated month must never be exported over, got {outcome:?}"
    );
}

#[test]
fn the_export_never_reports_write_without_a_proven_permit() {
    // The whole point in one assertion: `Write` is only reachable from
    // `SyncPermit::Proven`, which means Excel read the workbook and the database
    // held every `X` in it. Every unmeasured shape lands somewhere else.
    let pool = test_pool();
    let undated = template("Z:/no-such-dir/SF2-SEPTEMBER-2026.xls");

    for (label, dates) in [
        ("no dates", vec![]),
        ("one date", vec![date_mapping("2026-09-01", "F")]),
    ] {
        let outcome = export_guard_outcome(&pool, &undated, &[student_mapping()], &dates);
        assert_ne!(
            outcome,
            ExportGuardOutcome::Write,
            "{label}: the export granted itself permission to write"
        );
    }
}

#[test]
fn the_export_runs_the_guard_before_it_writes_or_produces_a_file() {
    let body = EXCEL_SERVICE_SOURCE
        .split_once("pub fn export_workbook")
        .expect("export_workbook is in this file")
        .1
        .split_once("pub(crate) fn refresh_template_calendar_from_saved_month")
        .expect("export_workbook sits above the calendar refresh helper")
        .0;

    let guard = body
        .find("export_guard_outcome")
        .expect("export_workbook must evaluate the destructive-sync guard");
    for write in [
        "save_workbook_path",
        "write_template_marks_for_days",
        "std::fs::copy",
    ] {
        let at = body
            .find(write)
            .unwrap_or_else(|| panic!("{write} is expected in export_workbook"));
        assert!(
            guard < at,
            "`{write}` runs at offset {at}, before the guard at offset {guard}. Acceptance #2 is \
             \"the export is refused ... No output file is written\"; a guard below the mark write \
             or below the copy satisfies the letter of it while still having written the grid"
        );
    }
}
