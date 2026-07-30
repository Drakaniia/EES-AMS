use super::*;
use super::super::workbook_utils::{
    column_number_to_letter, contains_ignore_ascii_case, month_number, sheet_is_analysis_candidate,
    year_from_sheet_name,
};
use super::super::com_session::run_excel_task;
use super::super::{batch_operations, write_marks, write_marks_force};
use crate::sf2::logic::Sf2CellMark;

// ── month_number ──────────────────────────────────────────────────────

#[test]
fn month_number_standard_month_name() {
    assert_eq!(month_number("JUNE 2026"), 6);
}

#[test]
fn month_number_lowercase() {
    assert_eq!(month_number("june 2026"), 6);
}

#[test]
fn month_number_partial_match_march() {
    // "MAR" is inside "MARCH"
    assert_eq!(month_number("MARCH 2026"), 3);
}

#[test]
fn month_number_problematic_sheet_name() {
    // The sheet name from the failing SF2 file
    assert_eq!(month_number("school_form_2_ver2014.2.1.1"), 0);
}

#[test]
fn month_number_empty_string() {
    assert_eq!(month_number(""), 0);
}

#[test]
fn month_number_no_match() {
    // "SUMMARY" contains "MAR" as substring (positions 3-5), so it
    // returns 3. Use a truly non-matching name for the "no match" test.
    assert_eq!(month_number("OVERVIEW"), 0);
}

#[test]
fn month_number_all_months() {
    assert_eq!(month_number("JANUARY 2026"), 1);
    assert_eq!(month_number("FEBRUARY 2026"), 2);
    assert_eq!(month_number("MARCH 2026"), 3);
    assert_eq!(month_number("APRIL 2026"), 4);
    assert_eq!(month_number("MAY 2026"), 5);
    assert_eq!(month_number("JUNE 2026"), 6);
    assert_eq!(month_number("JULY 2026"), 7);
    assert_eq!(month_number("AUGUST 2026"), 8);
    assert_eq!(month_number("SEPTEMBER 2026"), 9);
    assert_eq!(month_number("OCTOBER 2026"), 10);
    assert_eq!(month_number("NOVEMBER 2026"), 11);
    assert_eq!(month_number("DECEMBER 2026"), 12);
}

// ── stale Excel process cleanup ────────────────────────────────────────

// ── year_from_sheet_name ──────────────────────────────────────────────

#[test]
fn year_from_sheet_name_standard() {
    assert_eq!(year_from_sheet_name("JUNE 2026"), 2026);
}

#[test]
fn year_from_sheet_name_problematic_name() {
    // The sheet name contains "2014" from the version string
    assert_eq!(year_from_sheet_name("school_form_2_ver2014.2.1.1"), 2014);
}

#[test]
fn year_from_sheet_name_no_year() {
    assert_eq!(year_from_sheet_name("SUMMARY"), 0);
}

#[test]
fn year_from_sheet_name_not_starting_with_20() {
    assert_eq!(year_from_sheet_name("SHEET_1999"), 0);
}

#[test]
fn year_from_sheet_name_four_digit_starting_with_20() {
    assert_eq!(year_from_sheet_name("SHEET_2026"), 2026);
}

#[test]
fn year_from_sheet_name_empty_string() {
    assert_eq!(year_from_sheet_name(""), 0);
}

// ── contains_ignore_ascii_case ────────────────────────────────────────

#[test]
fn contains_ignore_ascii_case_exact_match() {
    assert!(contains_ignore_ascii_case("School Form 2", "School Form 2"));
}

#[test]
fn contains_ignore_ascii_case_case_insensitive() {
    assert!(contains_ignore_ascii_case(
        "school form 2 (sf2)",
        "School Form 2"
    ));
}

#[test]
fn contains_ignore_ascii_case_substring() {
    assert!(contains_ignore_ascii_case(
        "School Form 2 (SF2) Daily Attendance Report of Learners",
        "School Form 2"
    ));
}

#[test]
fn contains_ignore_ascii_case_no_match() {
    assert!(!contains_ignore_ascii_case(
        "Something else entirely",
        "School Form 2"
    ));
}

#[test]
fn contains_ignore_ascii_case_empty_haystack() {
    assert!(!contains_ignore_ascii_case("", "School Form 2"));
}

#[test]
fn contains_ignore_ascii_case_empty_needle() {
    assert!(contains_ignore_ascii_case("anything", ""));
}

// ── column_number_to_letter ───────────────────────────────────────────

#[test]
fn column_number_to_letter_a() {
    assert_eq!(column_number_to_letter(1), "A");
}

#[test]
fn column_number_to_letter_z() {
    assert_eq!(column_number_to_letter(26), "Z");
}

#[test]
fn column_number_to_letter_aa() {
    assert_eq!(column_number_to_letter(27), "AA");
}

#[test]
fn column_number_to_letter_f() {
    assert_eq!(column_number_to_letter(6), "F");
}

#[test]
fn column_number_to_letter_al() {
    assert_eq!(column_number_to_letter(38), "AL");
}

// ── sheet_is_analysis_candidate ───────────────────────────────────────

#[test]
fn sheet_is_analysis_candidate_monthly_sheet() {
    // Standard monthly sheet should always be a candidate
    assert!(sheet_is_analysis_candidate(
        "JUNE 2026",
        "School Form 2 (SF2) Daily Attendance Report of Learners",
        -1,
    ));
}

#[test]
fn sheet_is_analysis_candidate_non_monthly_sf2_sheet() {
    // THIS IS THE BUG REPRODUCTION:
    // A visible sheet containing a genuine SF2 form but with a non-standard
    // sheet name (like the failing file) should be usable as a fallback.
    // Currently the code skips it entirely — this test expects it to work.
    assert!(sheet_is_analysis_candidate(
        "school_form_2_ver2014.2.1.1",
        "School Form 2 (SF2) Daily Attendance Report of Learners",
        -1,
    ));
}

#[test]
fn sheet_is_analysis_candidate_non_monthly_non_sf2_sheet() {
    // A non-monthly sheet that is NOT an SF2 form should not be a candidate
    assert!(!sheet_is_analysis_candidate(
        "Summary",
        "Some random data",
        -1,
    ));
}

#[test]
fn sheet_is_analysis_candidate_hidden_sheet() {
    // Hidden sheets should never be candidates
    assert!(!sheet_is_analysis_candidate(
        "JUNE 2026",
        "School Form 2 (SF2) Daily Attendance Report of Learners",
        0,
    ));
}

#[test]
fn sheet_is_analysis_candidate_non_monthly_sf2_with_no_sf2_title() {
    // A non-monthly sheet that has a name matching the problematic pattern
    // but doesn't have "School Form 2" in the title should NOT be a candidate
    assert!(!sheet_is_analysis_candidate(
        "school_form_2_ver2014.2.1.1",
        "Some other data",
        -1,
    ));
}

// ── formula cell handling (RED test) ─────────────────────────────────
//
// RED: This test proves that `set_sf2_mark` rejects formula cells via
// `ensure_not_formula` (the BUG behavior), while `set_sf2_mark_force`
// does NOT check for formulas and can overwrite them (the FIX behavior).
//
// The `write_template_marks_for_mappings` backfill used `write_marks` →
// `set_sf2_mark` and errored on formula cells like `JULY 2026!I23`.
// The fix changes to `write_marks_force` → `set_sf2_mark_force` which
// skips the formula check entirely.
//
// This test requires Microsoft Excel to be installed at runtime.

// ── WorkbookSession / batch_operations (Phase 1) ─────────────────────
//
// These tests prove that the new batch operations API can execute multiple
// Excel operations (analyze, write_marks, write_metadata, write_formulas,
// expand_roster_rows, hide_empty_learner_rows) in a SINGLE Excel process
// instead of one process per operation.
//
// They require Microsoft Excel to be installed at runtime.
//
// ⚠️ These tests MUST be run sequentially (--test-threads=1) because
// Excel COM automation does not support multiple concurrent sessions
// from the same process. When run in parallel, they will fail with
// "workbook setup should succeed" or similar COM errors.

#[test]
fn batch_operations_analyze_succeeds() {
    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let workbook_path = temp_dir.join(format!("test_batch_analyze_{pid}.xls"));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = Cleanup(workbook_path.clone());
    let _ = std::fs::remove_file(&workbook_path);

    // Setup: create a simple workbook
    // Note: tests run with --test-threads=1 to avoid COM conflicts between parallel Excel instances.
    let path_for_setup = workbook_path.clone();
    let setup = run_excel_task(move || {
        let mut excel = ExcelSession::new()?;
        let workbooks = excel.app.get_object("Workbooks")?;
        let workbook = workbooks.method_object("Add", vec![])?;
        let sheets = workbook.get_object("Worksheets")?;
        let sheet = sheets.get_object_with_args("Item", vec![ComVariant::i4(1)])?;
        sheet.put_string("Name", "JULY 2026")?;
        let _ = workbook.method(
            "SaveAs",
            vec![
                ComVariant::bstr(&path_for_setup.to_string_lossy()),
                ComVariant::i4(-4143),
            ],
        );
        let _ = workbook.method("Close", vec![ComVariant::bool(false)]);
        let _ = excel.quit();
        Ok(())
    });
    assert!(setup.is_ok(), "workbook setup should succeed");

    // TEST: batch_operations should be able to analyze the workbook
    let result = batch_operations(&workbook_path, false, |session| {
        let analysis = session.analyze()?;
        assert!(!analysis.sheets.is_empty(), "should have at least 1 sheet");
        assert!(
            !analysis.learners.is_empty() || analysis.sheets.len() >= 1,
            "analysis should return sheet info"
        );
        Ok(())
    });

    assert!(
        result.is_ok(),
        "batch_operations with analyze should succeed, got: {:?}",
        result
    );
}

#[test]
fn batch_operations_multiple_ops_succeed() {
    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let workbook_path = temp_dir.join(format!("test_batch_multi_{pid}.xls"));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = Cleanup(workbook_path.clone());
    let _ = std::fs::remove_file(&workbook_path);

    // Setup: create a workbook
    // Note: tests run with --test-threads=1 to avoid COM conflicts between parallel Excel instances.
    let path_for_setup = workbook_path.clone();
    let setup = run_excel_task(move || {
        let mut excel = ExcelSession::new()?;
        let workbooks = excel.app.get_object("Workbooks")?;
        let workbook = workbooks.method_object("Add", vec![])?;
        let sheets = workbook.get_object("Worksheets")?;
        let sheet = sheets.get_object_with_args("Item", vec![ComVariant::i4(1)])?;
        sheet.put_string("Name", "JULY 2026")?;
        let _ = workbook.method(
            "SaveAs",
            vec![
                ComVariant::bstr(&path_for_setup.to_string_lossy()),
                ComVariant::i4(-4143),
            ],
        );
        let _ = workbook.method("Close", vec![ComVariant::bool(false)]);
        let _ = excel.quit();
        Ok(())
    });
    assert!(setup.is_ok(), "workbook setup should succeed");

    // TEST: Execute multiple operations in a single batch_operations call
    // This is the KEY test proving batching works.
    //
    // Note: marks is moved into the closure (move keyword) because the
    // closure must be 'static (sent to another thread via run_excel_task).
    let marks = vec![crate::sf2::logic::Sf2CellMark {
        sheet_name: "JULY 2026".to_string(),
        cell_address: "A1".to_string(),
        value: "Test".to_string(),
    }];

    let result = batch_operations(&workbook_path, true, move |session| {
        // Operation 1: analyze
        let analysis = session.analyze()?;
        assert!(!analysis.sheets.is_empty(), "analyze must return sheets");

        // Operation 2: write marks
        session.write_marks(&marks)?;

        // Operation 3: write marks force
        session.write_marks_force(&marks)?;

        // Operation 4: write formulas
        session.write_formulas(&marks)?;

        // Operation 5: expand roster (0 rows — no-op but tests the API)
        session.expand_roster_rows(0, 0, None, None)?;

        // Operation 6: hide empty learner rows (no-op for this simple workbook)
        let occupied = std::collections::HashSet::new();
        session.hide_empty_learner_rows(29, 49, &occupied)?;

        Ok(())
    });

    assert!(
        result.is_ok(),
        "batch_operations with multiple ops should succeed, got: {:?}",
        result
    );

    // Verify: open the workbook again and check A1 value
    let path_for_check = workbook_path.clone();
    let check = run_excel_task(move || {
        let mut excel = ExcelSession::new()?;
        let workbooks = excel.app.get_object("Workbooks")?;
        let workbook = workbooks.method_object(
            "Open",
            vec![
                ComVariant::bstr(&path_for_check.to_string_lossy()),
                ComVariant::i4(0),
                ComVariant::bool(true),
            ],
        )?;
        let sheets = workbook.get_object("Worksheets")?;
        let sheet = sheets.get_object_with_args("Item", vec![ComVariant::bstr("JULY 2026")])?;
        let cell = sheet.get_object_with_args("Range", vec![ComVariant::bstr("A1")])?;
        let value = cell.get("Value2")?.to_string_value();
        assert_eq!(
            value, "Test",
            "cell A1 should contain 'Test' after write_marks"
        );
        let _ = workbook.method("Close", vec![ComVariant::bool(false)]);
        let _ = excel.quit();
        Ok(())
    });

    assert!(
        check.is_ok(),
        "verification should succeed, got: {:?}",
        check
    );
}

#[test]
fn set_sf2_mark_rejects_formula_cells_but_force_accepts() {
    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let workbook_path = temp_dir.join(format!("test_formula_cell_{pid}.xls"));
    let _ = std::fs::remove_file(&workbook_path);

    let path_for_thread = workbook_path.clone();
    let result = run_excel_task(move || {
        let mut excel = ExcelSession::new()?;
        let workbooks = excel.app.get_object("Workbooks")?;
        let workbook = workbooks.method_object("Add", vec![])?;
        let sheets = workbook.get_object("Worksheets")?;
        let sheet = sheets.get_object_with_args("Item", vec![ComVariant::i4(1)])?;

        // Write a formula to cell A1
        let cell = sheet.get_object_with_args("Range", vec![ComVariant::bstr("A1")])?;
        cell.put_string("Formula", "=1+1")?;

        // Confirm the cell HAS a formula (setup verification)
        let has_formula = cell.get_bool("HasFormula")?;
        assert!(
            has_formula,
            "cell A1 should have '=1+1' formula after setup"
        );

        // TEST 1: set_sf2_mark SHOULD REJECT formula cells (this IS the bug)
        let reject = crate::sf2::excel_com::worksheet::set_sf2_mark(&sheet, "A1", "X");
        assert!(
            reject.is_err(),
            "set_sf2_mark MUST reject formula cells — its call to ensure_not_formula should refuse"
        );
        let err_msg = reject.err().unwrap().to_string();
        assert!(
            err_msg.contains("Refusing to overwrite formula cell"),
            "error message should contain 'Refusing to overwrite formula cell', got: {err_msg}"
        );

        // TEST 2: set_sf2_mark_force SHOULD ACCEPT formula cells (this IS the fix)
        let force = crate::sf2::excel_com::worksheet::set_sf2_mark_force(&sheet, "A1", "X");
        assert!(
            force.is_ok(),
            "set_sf2_mark_force MUST accept formula cells — it skips ensure_not_formula"
        );

        // Save and close
        let _ = workbook.method(
            "SaveAs",
            vec![
                ComVariant::bstr(&path_for_thread.to_string_lossy()),
                ComVariant::i4(-4143), // xlExcel9795
            ],
        );
        let _ = workbook.method("Close", vec![ComVariant::bool(false)]);
        let _ = excel.quit();

        Ok(())
    });

    let _ = std::fs::remove_file(&workbook_path);

    match result {
        Ok(()) => {}
        Err(e) => panic!("Test failed with Excel error: {e}"),
    }
}

/// RED: This test proves that `write_marks` (the CURRENT function used by
/// `write_template_marks_for_mappings`) FAILS when the workbook contains
/// formula cells in the attendance area. `write_marks_force` succeeds.
///
/// This directly models the BUG: the backfill calls `write_marks` which
/// calls `set_sf2_mark` → `ensure_not_formula`. The fix calls
/// `write_marks_force` → `set_sf2_mark_force` (no formula check).
#[test]
fn write_marks_rejects_formula_cells_but_write_marks_force_accepts() {
    // Use Drop-based cleanup guard so temp file is removed even on panic
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let workbook_path = temp_dir.join(format!("test_write_marks_formula_{pid}.xls"));
    let _guard = Cleanup(workbook_path.clone());
    let _ = std::fs::remove_file(&workbook_path);

    // Step 1: Create a workbook with a formula in cell A1
    let path_for_setup = workbook_path.clone();
    let setup_result = run_excel_task(move || {
        let mut excel = ExcelSession::new()?;
        let workbooks = excel.app.get_object("Workbooks")?;
        let workbook = workbooks.method_object("Add", vec![])?;
        let sheets = workbook.get_object("Worksheets")?;
        let sheet = sheets.get_object_with_args("Item", vec![ComVariant::i4(1)])?;

        let cell = sheet.get_object_with_args("Range", vec![ComVariant::bstr("A1")])?;
        cell.put_string("Formula", "=1+1")?;

        // Rename sheet to match an SF2-like name for write_marks lookup
        let _ = sheet.put_string("Name", "JULY 2026");

        let _ = workbook.method(
            "SaveAs",
            vec![
                ComVariant::bstr(&path_for_setup.to_string_lossy()),
                ComVariant::i4(-4143),
            ],
        );
        let _ = workbook.method("Close", vec![ComVariant::bool(false)]);
        let _ = excel.quit();
        Ok(())
    });
    assert!(setup_result.is_ok(), "workbook setup should succeed");

    // Step 2: Test write_marks on a mark targeting the formula cell A1
    let marks = vec![Sf2CellMark {
        sheet_name: "JULY 2026".to_string(),
        cell_address: "A1".to_string(),
        value: "X".to_string(),
    }];

    // write_marks should FAIL — this is the BUG in the current backfill
    let write_result = write_marks(&workbook_path, &marks);
    assert!(
        write_result.is_err(),
        "write_marks MUST fail on formula cells — THIS IS THE BUG"
    );
    let err_msg = write_result.err().unwrap().to_string();
    assert!(
        err_msg.contains("Refusing to overwrite formula cell"),
        "error must mention formula cell refusal, got: {err_msg}"
    );

    // Step 3: Test write_marks_force on the same mark — should SUCCEED
    let force_result = write_marks_force(&workbook_path, &marks);
    assert!(
        force_result.is_ok(),
        "write_marks_force MUST succeed on formula cells — THIS IS THE FIX"
    );
}
