//! The end-to-end regression test for the false zero, against a **real** `.xls`.
//!
//! ## What this exists for
//!
//! Every other test in this file tests a pure function. This one drives the
//! whole measurement path - build a workbook with known `X` marks through Excel
//! itself, close it, then call
//! [`measure_workbook_marks`](crate::sf2::guard::evaluate::measure_workbook_marks)
//! on it - because the defect lived in the gap between those two halves: the
//! formula was refused by Excel, the answer was an error variant, the COM reader
//! rendered that as `""`, and `""` was read as "this month holds no `X` marks".
//!
//! No unit test of the parsing could have failed before the fix, because the
//! parsing was never wrong *on its own* - it was handed a lie and believed it.
//! This test hands the reader a workbook with three `X` marks in it and asserts
//! it finds three. On the unfixed code it finds **zero**, and the guard built on
//! that read answers `Proven` for any month.
//!
//! ## Why the sheet is hidden
//!
//! The `Application.Evaluate` form this used cannot evaluate a formula that
//! names a hidden sheet: it behaves like typing into the *active* cell, and on a
//! workbook whose active sheet is a different tab it returns an error variant.
//! Eleven of a file's twelve month sheets are hidden in the state the pre-split
//! calendar cycle leaves them in, so the read has to be worksheet-scoped. The
//! fixture therefore hides the month sheet and leaves a different sheet active -
//! which is the case that a worksheet-scoped read handles and an
//! application-scoped one does not.
//!
//! Requires Microsoft Excel at runtime; the suite's Excel tests are serialised
//! by the process-wide gate in `sf2::excel::excel_lock`.

use crate::sf2::attendance_marks::attendance_block_columns;
use crate::sf2::guard::evaluate::{measure_workbook_marks, WorkbookMarkScan};
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord};

/// The month sheet in the fixture. Named like a real one so
/// `worksheet_by_name`-style lookups and the sheet-name quoting are exercised.
const MONTH_SHEET: &str = "SEPTEMBER 2026";
/// The sheet that stays visible and active, standing in for the rest of the
/// workbook.
const ACTIVE_SHEET: &str = "COVER";

/// The `X` marks the fixture holds: `(learner row, column letter)`.
///
/// Three, in three different columns and three different rows, so a read that
/// collapsed the columns together or shifted them by one would be caught.
const KNOWN_MARKS: [(&str, u32); 3] = [("F", 8), ("K", 9), ("AL", 12)];

/// A mapped learner row.
fn student(row: u32) -> Sf2StudentMappingRecord {
    Sf2StudentMappingRecord {
        template_id: "tpl".to_string(),
        student_id: format!("student-{row}"),
        workbook_name: format!("LEARNER {row}"),
        normalized_name: format!("LEARNER {row}"),
        row_index: row,
        gender_block: Some("MALE".to_string()),
    }
}

/// A mapped day column.
fn date(column: &str) -> Sf2DateMappingRecord {
    Sf2DateMappingRecord {
        template_id: "tpl".to_string(),
        sheet_name: MONTH_SHEET.to_string(),
        date: "2026-09-01".to_string(),
        column_letter: column.to_string(),
        column_index: 0,
    }
}

/// Every day column of the block, mapped - so the whole grid is in scope and a
/// mark cannot be dropped for being out of it.
fn all_dates() -> Vec<Sf2DateMappingRecord> {
    attendance_block_columns()
        .iter()
        .map(|column| date(column))
        .collect()
}

fn all_students() -> Vec<Sf2StudentMappingRecord> {
    (8u32..=12).map(student).collect()
}

/// Build a workbook holding exactly [`KNOWN_MARKS`], and return its path.
///
/// Written with Excel rather than shipped as a binary fixture, so the file is
/// this build's own idea of the format and the test cannot rot against a
/// hand-made `.xls`. The month sheet is hidden and a different sheet is left
/// active before the workbook is saved, so the file on disk is in the state the
/// read has to survive.
///
/// `label` names the file, so two tests that both need a fixture do not race
/// over one path even though the Excel gate serialises them.
fn build_fixture(label: &str) -> std::path::PathBuf {
    use crate::sf2::excel::excel_com::com_session::{run_excel_task, ComVariant, ExcelSession};

    let path = std::env::temp_dir().join(format!(
        "sf2_guard_measure_{label}_{}.xls",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let path_for_setup = path.clone();

    let setup = run_excel_task(move || {
        let mut excel = ExcelSession::new()?;
        let workbooks = excel.app.get_object("Workbooks")?;
        let workbook = workbooks.method_object("Add", vec![])?;
        let sheets = workbook.get_object("Worksheets")?;

        // The sheet that stays visible, and which is left active at the end.
        let cover = sheets.get_object_with_args("Item", vec![ComVariant::i4(1)])?;
        cover.put_string("Name", ACTIVE_SHEET)?;

        // `Worksheets.Add` takes `(Before, Count, Type)`, so a bare call adds a
        // single new sheet ahead of the active one. Passing the future name
        // here instead would ask Excel to insert before a sheet that does not
        // exist yet, and it refuses.
        let month = sheets.method_object("Add", vec![])?;
        month.put_string("Name", MONTH_SHEET)?;

        for (column, row) in KNOWN_MARKS {
            let cell = month
                .get_object_with_args("Range", vec![ComVariant::bstr(&format!("{column}{row}"))])?;
            cell.put_string("Value2", "X")?;
        }

        // Hide the month sheet. This is the state a post-cycle file is in, and
        // the reason the read has to be worksheet-scoped: with the month sheet
        // hidden, an application-scoped read evaluates in whatever sheet happens
        // to be active and answers an error value.
        month.put_i4("Visible", 0)?;

        // Make the cover active explicitly, so the file on disk is opened with a
        // different sheet active from the one being measured. `Activate` is a
        // member of `Worksheet`; it is not a method on `Application`, which is
        // why this is called on the sheet and not on `excel.app`.
        cover.method("Activate", vec![])?;

        workbook.method(
            "SaveAs",
            vec![
                ComVariant::bstr(&path_for_setup.to_string_lossy()),
                ComVariant::i4(-4143), // xlExcel8 (.xls)
            ],
        )?;
        let _ = workbook.method("Close", vec![ComVariant::bool(false)]);
        let _ = excel.quit();
        Ok(())
    });
    setup.expect("fixture setup should succeed");
    path
}

struct Cleanup(std::path::PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn the_guard_finds_the_x_marks_a_real_workbook_actually_holds() {
    let path = build_fixture("marks");
    let _guard = Cleanup(path.clone());
    assert!(
        path.exists(),
        "the fixture was not written: {}",
        path.display()
    );

    let scan = measure_workbook_marks(&path, &all_students(), &all_dates());

    let WorkbookMarkScan::Measured {
        x_cells,
        cells_scanned,
    } = &scan
    else {
        panic!(
            "the month sheet is readable and its marks are knowable, so this must be a \
             measurement - a failure here means the read refuses workbooks it can read, \
             which is the other half of the same defect: {scan:?}"
        );
    };

    let mut found: Vec<(String, String, u32)> = x_cells
        .iter()
        .map(|cell| {
            (
                cell.sheet_name.clone(),
                cell.column_letter.clone(),
                cell.row_index,
            )
        })
        .collect();
    let mut expected: Vec<(String, String, u32)> = KNOWN_MARKS
        .iter()
        .map(|(column, row)| (MONTH_SHEET.to_string(), (*column).to_string(), *row))
        .collect();
    found.sort();
    expected.sort();

    assert_eq!(
        found, expected,
        "the scan must report every X mark the workbook holds, at the cell it is in. \
         Before the fix this was empty: the row-mask formula was refused by Excel, the \
         error answer was read as \"\", and \"\" is zero marks - which satisfies \
         db_count >= workbook_count for any month and so licensed the clear."
    );
    assert_eq!(
        *cells_scanned,
        5 * 33,
        "the whole grid must be inspected: 5 mapped learner rows x 33 mapped day columns"
    );
}

#[test]
fn a_workbook_holding_no_x_measures_as_zero_and_not_as_a_failure() {
    // The other half, and the one a lazy fix gets wrong: validation that
    // refuses everything would also "never clear on a measurement that did not
    // happen", and would be equally useless. A real zero has to survive.
    let path = build_fixture("empty");
    let _guard = Cleanup(path.clone());

    // Read the ACTIVE_SHEET instead: it exists, it is visible, and it holds no
    // X marks at all.
    let mut dates = all_dates();
    for mapping in &mut dates {
        mapping.sheet_name = ACTIVE_SHEET.to_string();
    }

    let scan = measure_workbook_marks(&path, &all_students(), &dates);

    let WorkbookMarkScan::Measured { x_cells, .. } = &scan else {
        panic!("a sheet with no X marks is measured, not failed: {scan:?}");
    };
    assert!(
        x_cells.is_empty(),
        "the cover sheet genuinely holds no absences, and that has to be reportable as zero"
    );
}

#[test]
fn a_sheet_the_workbook_does_not_have_is_a_failure_and_never_zero_marks() {
    // The post-cycle case. A month the pre-split cycle renamed to
    // `__SF2_HIDDEN_n` cannot be measured, and "cannot be measured" is the whole
    // subject of this change: it must be `Failed`, so the guard opens the month
    // read-only, and not `Measured { x_cells: [] }`, so the guard does not read
    // it as a month with no absences and clear it.
    let path = build_fixture("absent");
    let _guard = Cleanup(path.clone());

    let mut dates = all_dates();
    for mapping in &mut dates {
        mapping.sheet_name = "__SF2_HIDDEN_9".to_string();
    }

    let scan = measure_workbook_marks(&path, &all_students(), &dates);

    assert!(
        matches!(scan, WorkbookMarkScan::Failed { .. }),
        "a month whose sheet is not in the workbook cannot be measured, and reporting that \
         as zero marks is how a failed read becomes permission to erase the grid: {scan:?}"
    );
    assert_eq!(scan.x_cells(), None);
}
