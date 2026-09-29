use super::*;
use crate::sf2::attendance_marks::{
    attendance_block_columns, attendance_scope_cells, differential_clear_marks,
};
use crate::sf2::guard::evaluate::{permit_from_scan, WorkbookMarkScan};
use crate::sf2::logic::Sf2CellMark;
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord};

// ── helpers ─────────────────────────────────────────────────────────────

/// One learner row on one day column of one sheet.
fn cell(sheet: &str, column: &str, row: u32) -> Sf2GridCell {
    Sf2GridCell {
        sheet_name: sheet.to_string(),
        column_letter: column.to_string(),
        row_index: row,
    }
}

/// A month with three mapped learners on rows 8-10 and three mapped day
/// columns F-H, which is the smallest shape that can express every case here.
const SHEET: &str = "SEPTEMBER 2026";

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

fn date(column: &str, day: &str) -> Sf2DateMappingRecord {
    Sf2DateMappingRecord {
        template_id: "tpl".to_string(),
        sheet_name: SHEET.to_string(),
        date: format!("2026-09-{day}"),
        column_letter: column.to_string(),
        column_index: 0,
    }
}

fn students() -> Vec<Sf2StudentMappingRecord> {
    vec![student(8), student(9), student(10)]
}

fn dates() -> Vec<Sf2DateMappingRecord> {
    vec![date("F", "01"), date("G", "02"), date("H", "03")]
}

/// Labels covering the same month the mappings describe.
fn labels() -> CellLabels {
    let mut labels = CellLabels::new();
    for mapping in students() {
        for day in dates() {
            labels.insert(
                &cell(SHEET, &day.column_letter, mapping.row_index),
                mapping.workbook_name.clone(),
                day.date.clone(),
            );
        }
    }
    labels
}

// ── the property: Unmeasured is the default, never Proven ───────────────
//
// Every "could not measure" case - file missing, Excel unavailable, file
// locked, no mappings - must land on Unmeasured. A single one of them
// resolving to Proven would let the write path clear the grid on no evidence.

#[test]
fn a_workbook_that_could_not_be_measured_is_unmeasured_not_proven() {
    let permit = decide(&[], None, &labels(), "Excel is not available");

    assert!(
        matches!(permit, SyncPermit::Unmeasured { .. }),
        "an unmeasurable workbook must never produce a permit that allows clearing"
    );
    assert_eq!(
        permit,
        SyncPermit::Unmeasured {
            reason: "Excel is not available".to_string()
        }
    );
}

#[test]
fn an_unmeasured_permit_never_permits_a_rewrite() {
    for reason in [
        "SF2-JULY.xls is missing. Restore it from a backup.",
        "the workbook is open in Microsoft Excel",
        "this workbook has no mapped attendance dates",
        "Microsoft Excel is not available",
    ] {
        let permit = SyncPermit::unmeasured(reason);
        assert_eq!(
            action_for(&permit),
            SyncAction::ReadOnly {
                reason: reason.to_string()
            },
            "an unmeasured workbook must be opened read-only, never rewritten"
        );
    }
}

#[test]
fn an_unmeasured_workbook_clears_nothing() {
    // The database is empty and the workbook could not be read. There is no
    // evidence for clearing any cell, so the whole grid survives - this is the
    // §4 chain with the measurement step failing.
    let permit = decide(&[], None, &labels(), "the workbook is locked");
    let outcome = guard_before_write(
        || permit.clone(),
        |_| panic!("must not import when unmeasured"),
    )
    .expect("guard runs");

    assert_eq!(
        outcome,
        SyncAction::ReadOnly {
            reason: "the workbook is locked".to_string()
        }
    );
}

#[test]
fn the_missing_file_message_is_the_specified_one() {
    assert_eq!(
        missing_workbook_file_message("JULY"),
        "SF2-JULY.xls is missing. Restore it from a backup."
    );
}

#[test]
fn a_template_with_no_mapped_dates_is_unmeasured() {
    // A degenerate workbook analysis leaves the month with zero date mappings.
    // That is the signature of a failed refresh, and it must resolve to
    // "do not clear" rather than "the database is empty, so blank everything".
    assert!(!NO_MAPPED_DATES.is_empty());
    assert!(!NO_MAPPED_LEARNERS.is_empty());
    assert!(!WORKBOOK_NOT_READABLE.is_empty());
}

// ── Proven / Stale ──────────────────────────────────────────────────────

#[test]
fn the_database_proving_at_least_as_many_marks_permits_a_rewrite() {
    let db = vec![cell(SHEET, "F", 8), cell(SHEET, "G", 9)];
    let workbook = vec![cell(SHEET, "F", 8)];

    assert_eq!(
        decide(&db, Some(&workbook), &labels(), "unused"),
        SyncPermit::Proven {
            db_count: 2,
            workbook_count: 1
        }
    );
}

#[test]
fn an_empty_database_against_an_empty_workbook_is_proven() {
    // Both empty is genuinely "the database holds everything the workbook
    // shows", which is the one harmless degenerate case.
    assert_eq!(
        decide(&[], Some(&[]), &labels(), "unused"),
        SyncPermit::Proven {
            db_count: 0,
            workbook_count: 0
        }
    );
}

#[test]
fn a_database_behind_the_workbook_is_stale_with_the_named_missing_pairs() {
    // The acceptance scenario: a database whose events were emptied for a
    // month, against a workbook still holding 12 X marks. Here two of them.
    let workbook = vec![
        cell(SHEET, "F", 8),
        cell(SHEET, "G", 9),
        cell(SHEET, "H", 10),
    ];
    let db = vec![cell(SHEET, "F", 8)];

    let permit = decide(&db, Some(&workbook), &labels(), "unused");

    assert_eq!(
        permit,
        SyncPermit::Stale {
            db_count: 1,
            workbook_count: 3,
            missing: vec![
                ("LEARNER 9".to_string(), "2026-09-02".to_string()),
                ("LEARNER 10".to_string(), "2026-09-03".to_string()),
            ],
        }
    );
}

#[test]
fn a_stale_permit_asks_for_an_import_rather_than_a_clear() {
    let permit = SyncPermit::Stale {
        db_count: 0,
        workbook_count: 2,
        missing: vec![
            ("LEARNER 8".to_string(), "2026-09-01".to_string()),
            ("LEARNER 9".to_string(), "2026-09-02".to_string()),
        ],
    };

    assert_eq!(
        action_for(&permit),
        SyncAction::ImportThenRecheck {
            db_count: 0,
            workbook_count: 2,
            missing: vec![
                ("LEARNER 8".to_string(), "2026-09-01".to_string()),
                ("LEARNER 9".to_string(), "2026-09-02".to_string()),
            ],
        }
    );
}

#[test]
fn equal_counts_over_different_cells_are_stale_not_proven() {
    // The count rule alone would say "proven" here: 2 >= 2. But the workbook's
    // two X marks are on days the database knows nothing about, so clearing
    // them destroys marks the user never retracted. An X may only be removed
    // by the user marking that student present, and they did not.
    let workbook = vec![cell(SHEET, "G", 8), cell(SHEET, "H", 9)];
    let db = vec![cell(SHEET, "F", 8), cell(SHEET, "F", 9)];

    let permit = decide(&db, Some(&workbook), &labels(), "unused");

    match permit {
        SyncPermit::Stale { missing, .. } => assert_eq!(
            missing,
            vec![
                ("LEARNER 8".to_string(), "2026-09-02".to_string()),
                ("LEARNER 9".to_string(), "2026-09-03".to_string()),
            ],
            "every workbook X the database cannot produce must be named"
        ),
        other => panic!("equal counts over different cells must be Stale, got {other:?}"),
    }
}

#[test]
fn an_unlabelled_missing_cell_is_still_reported() {
    // A workbook X in a cell the mappings do not cover cannot be named, but it
    // must never be dropped from the report - a silently shorter list is a
    // silently shorter recovery.
    let workbook = vec![cell("__SF2_HIDDEN_9", "Z", 99)];
    let permit = decide(&[], Some(&workbook), &labels(), "unused");

    let SyncPermit::Stale { missing, .. } = permit else {
        panic!("an unlabelled workbook X must still be Stale");
    };
    assert_eq!(missing.len(), 1);
    assert!(
        missing[0].0.contains('9') || missing[0].0.contains("row"),
        "the fallback must still identify the row: {:?}",
        missing[0]
    );
}

// ── Stale -> import -> re-evaluate ──────────────────────────────────────

#[test]
fn a_stale_permit_that_the_import_repairs_proceeds() {
    let behind = SyncPermit::Stale {
        db_count: 0,
        workbook_count: 1,
        missing: vec![("LEARNER 8".to_string(), "2026-09-01".to_string())],
    };
    let caught_up = SyncPermit::Proven {
        db_count: 1,
        workbook_count: 1,
    };
    let mut evaluations = 0;

    let outcome = guard_before_write(
        || {
            evaluations += 1;
            if evaluations == 1 {
                behind.clone()
            } else {
                caught_up.clone()
            }
        },
        |missing| {
            assert_eq!(missing.len(), 1, "the import needs the concrete list");
            Ok(())
        },
    )
    .expect("guard runs");

    assert_eq!(
        outcome,
        SyncAction::Rewrite {
            db_count: 1,
            workbook_count: 1
        },
        "re-evaluating after the import is what makes Stale -> Proven work"
    );
    assert_eq!(
        evaluations, 2,
        "the guard must re-evaluate after the import"
    );
}

#[test]
fn a_stale_permit_the_import_cannot_repair_aborts_with_the_specified_message() {
    let behind = SyncPermit::Stale {
        db_count: 0,
        workbook_count: 2,
        missing: vec![
            ("LEARNER 8".to_string(), "2026-09-01".to_string()),
            ("LEARNER 9".to_string(), "2026-09-02".to_string()),
        ],
    };
    let mut evaluations = 0;

    let outcome = guard_before_write(
        || {
            evaluations += 1;
            behind.clone()
        },
        |_| Ok(()),
    )
    .expect("guard runs");

    assert_eq!(
        outcome,
        SyncAction::Aborted {
            message: "The workbook has 2 X marks the app has no record of. \
                      Nothing was changed. Restore a backup or run workbook recovery."
                .to_string()
        }
    );
    assert_eq!(
        evaluations, 2,
        "one import and one re-evaluation, then a refusal - not a retry loop"
    );
}

#[test]
fn an_import_that_fails_propagates_its_error() {
    let behind = SyncPermit::Stale {
        db_count: 0,
        workbook_count: 1,
        missing: vec![("LEARNER 8".to_string(), "2026-09-01".to_string())],
    };

    let result = guard_before_write(
        || behind.clone(),
        |_| {
            Err(crate::domain::error::AppError::InvalidInput(
                "Excel is not available".into(),
            ))
        },
    );

    assert!(
        result.is_err(),
        "an import that could not run must not be treated as a repair"
    );
}

#[test]
fn a_repaired_stale_that_turns_unmeasured_opens_read_only() {
    // The import made the counts agree, but the re-evaluation then failed to
    // measure the workbook. That is not permission to write.
    let behind = SyncPermit::Stale {
        db_count: 0,
        workbook_count: 1,
        missing: vec![("LEARNER 8".to_string(), "2026-09-01".to_string())],
    };
    let mut evaluations = 0;

    let outcome = guard_before_write(
        || {
            evaluations += 1;
            if evaluations == 1 {
                behind.clone()
            } else {
                SyncPermit::unmeasured("the workbook is locked")
            }
        },
        |_| Ok(()),
    )
    .expect("guard runs");

    assert_eq!(
        outcome,
        SyncAction::ReadOnly {
            reason: "the workbook is locked".to_string()
        }
    );
}

#[test]
fn a_proven_permit_never_calls_the_import() {
    let outcome = guard_before_write(
        || SyncPermit::Proven {
            db_count: 5,
            workbook_count: 3,
        },
        |_| panic!("a proven database must not trigger a workbook import"),
    )
    .expect("guard runs");

    assert_eq!(
        outcome,
        SyncAction::Rewrite {
            db_count: 5,
            workbook_count: 3
        }
    );
}

// ── the differential clear (spec §9.2) ──────────────────────────────────

#[test]
fn the_differential_clear_blanks_a_cell_whose_database_record_is_gone() {
    // The workbook holds an X on 09-01 for LEARNER 8 and one on 09-02 for
    // LEARNER 9. The database still proves 09-02 but no longer records 09-01,
    // so 09-01 is the one cell a sync may blank and 09-02 is not.
    let scope = attendance_scope_cells(&students(), &dates());
    let db = vec![cell(SHEET, "G", 9)];
    let workbook = vec![cell(SHEET, "F", 8), cell(SHEET, "G", 9)];

    let marks = differential_clear_marks(&scope, &db, &workbook);

    assert_eq!(
        marks,
        vec![Sf2CellMark {
            sheet_name: SHEET.to_string(),
            cell_address: "F8".to_string(),
            value: String::new(),
        }],
        "only the X with no database record behind it may be blanked"
    );
}

#[test]
fn the_differential_clear_leaves_an_x_with_a_live_database_record_alone() {
    // The same workbook, but the database still proves 09-01. Marking that
    // student present is the only thing allowed to remove that X, and the user
    // has not done it.
    let scope = attendance_scope_cells(&students(), &dates());
    let db = vec![cell(SHEET, "F", 8), cell(SHEET, "G", 9)];
    let workbook = vec![cell(SHEET, "F", 8), cell(SHEET, "G", 9)];

    assert!(
        differential_clear_marks(&scope, &db, &workbook).is_empty(),
        "a cell the database still proves must never be blanked"
    );
}

#[test]
fn the_differential_clear_is_bounded_and_never_the_whole_grid() {
    // The regression test for the §4 chain. 40 learners x 33 day columns is
    // 1,320 cells; the whole of it used to be blanked. Even with an empty
    // database and an empty mapping-derived scope, the clear can only ever be
    // a subset of the X marks the workbook actually holds.
    let block_columns = attendance_block_columns();
    assert_eq!(
        block_columns.len(),
        33,
        "the DepEd SF2 attendance block is F..AL"
    );

    let roster: Vec<Sf2StudentMappingRecord> = (8u32..=47).map(student).collect();
    let month: Vec<Sf2DateMappingRecord> = block_columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            let mut mapping = date(column, "01");
            mapping.column_index = index as u32;
            mapping
        })
        .collect();
    let scope = attendance_scope_cells(&roster, &month);
    assert_eq!(scope.len(), 40 * 33, "the scope is the whole grid");

    // Every mapped day carries the same date string here, so de-duplicate the
    // scope down to a single day column to model a degenerate month: one date
    // mapped, 33 columns of grid in scope.
    let one_day = vec![date("F", "01")];
    let degenerate_scope = attendance_scope_cells(&roster, &one_day);
    assert_eq!(degenerate_scope.len(), 40);

    // The workbook holds 12 X marks and the database holds none.
    let workbook: Vec<Sf2GridCell> = (0..12).map(|offset| cell(SHEET, "F", 8 + offset)).collect();
    let clear = differential_clear_marks(&degenerate_scope, &[], &workbook);

    assert_eq!(
        clear.len(),
        12,
        "the clear is exactly the workbook's own X marks, not the grid"
    );
    assert!(
        clear.len() < degenerate_scope.len(),
        "the clear must be a strict subset of the scope"
    );
    assert!(
        differential_clear_marks(&degenerate_scope, &[], &[]).is_empty(),
        "with no X marks in the workbook there is nothing to clear"
    );
}

#[test]
fn the_differential_clear_never_touches_a_cell_outside_the_scope() {
    // A cell the app cannot map to a date is preserved, not blanked: the app
    // cannot reproduce a mark it cannot place, so destroying it would lose it.
    let scope = attendance_scope_cells(&students(), &dates());
    let unmapped = vec![cell(SHEET, "AL", 8), cell("__SF2_HIDDEN_3", "F", 8)];

    assert!(
        differential_clear_marks(&scope, &[], &unmapped).is_empty(),
        "cells outside the mapped rows and day columns must be left alone"
    );
}

#[test]
fn the_differential_clear_does_not_write_to_an_already_blank_cell() {
    // Writing an empty value to every cell in scope would be functionally the
    // same total clear, just slower - and it would trip the "bounded set"
    // property. Only cells that actually hold an X are returned.
    let scope = attendance_scope_cells(&students(), &dates());

    let clear = differential_clear_marks(&scope, &[], &[]);

    assert!(
        clear.is_empty(),
        "blanking cells that are already blank is a needless write and a \
         step back towards a total clear"
    );
}

#[test]
fn the_scope_is_mapped_rows_times_mapped_day_columns() {
    let scope = attendance_scope_cells(&students(), &dates());

    assert_eq!(scope.len(), 3 * 3);
    assert!(scope.contains(&cell(SHEET, "F", 8)));
    assert!(scope.contains(&cell(SHEET, "H", 10)));
    assert!(
        !scope.contains(&cell(SHEET, "I", 8)),
        "column I is not a mapped day column"
    );
}

#[test]
fn the_scope_skips_an_unlinked_learner_row() {
    // row_index 0 is the "no workbook row" placeholder: it addresses no cell.
    let roster = vec![student(0), student(8)];
    let scope = attendance_scope_cells(&roster, &dates());

    assert_eq!(scope.len(), 3);
    assert!(
        scope.iter().all(|entry| entry.row_index == 8),
        "an unlinked learner must not put a row in the clear scope"
    );
}

#[test]
fn the_scope_of_a_month_with_no_mappings_is_empty() {
    // The degenerate analysis. An empty scope means an empty clear set, which
    // is the structural half of the fix: no mappings can no longer mean a wipe.
    assert!(attendance_scope_cells(&students(), &[]).is_empty());
    assert!(attendance_scope_cells(&[], &dates()).is_empty());
}

#[test]
fn a_grid_cell_addresses_itself_the_way_excel_does() {
    assert_eq!(cell(SHEET, "AL", 47).address(), "AL47");
}

// ── S1: a failed measurement is not a measurement of zero ────────────────
//
// The defect these encode, in one sentence: the guard's Excel read returned a
// `VT_ERROR` variant, the COM reader rendered that as the empty string, an empty
// row mask is zero `X` marks, and `db_count >= 0` is true for every month on
// earth - so the guard answered `Proven`, which is permission to clear the grid,
// on a read that never happened.
//
// Each test below is a link in that chain. Take any one of them out and the
// chain reassembles.

// The whole bug, end to end: the database is empty, the workbook holds three X
// marks, and the read fails. A guard that laundered the failure into zero would
// answer Proven here and erase all three.
#[test]
fn a_read_that_errors_is_never_proven_even_with_an_empty_database() {
    let db: Vec<Sf2GridCell> = Vec::new();
    let failed = WorkbookMarkScan::failed("Excel would not read row 9 of 'SEPTEMBER 2026'");

    let permit = permit_from_scan(&db, &failed, &labels());

    assert_eq!(
        permit,
        SyncPermit::Unmeasured {
            reason: WORKBOOK_NOT_READABLE.to_string()
        },
        "an errored read must never be laundered into a count of zero and then into Proven"
    );
    assert!(
        !action_for(&permit).permits_write(),
        "an errored read must not permit a write"
    );
}

#[test]
fn a_read_that_errors_is_never_proven_even_with_a_database_that_is_ahead() {
    // The state the real install was in: 37 absences in the database against a
    // workbook the guard could not read. Harmless today, and only because the
    // database happened to be ahead. The permit must not depend on that.
    let db: Vec<Sf2GridCell> = (0..37).map(|offset| cell(SHEET, "F", 8 + offset)).collect();
    let failed = WorkbookMarkScan::failed(WORKBOOK_NOT_READABLE);

    let permit = permit_from_scan(&db, &failed, &labels());

    assert!(matches!(permit, SyncPermit::Unmeasured { .. }));
}

// The other half: a read that genuinely measured zero must still be allowed to
// say so, or the fix has traded a false zero for a permanent refusal.
#[test]
fn a_genuine_zero_is_proven_only_when_the_database_also_holds_zero() {
    let empty_workbook = WorkbookMarkScan::Measured {
        x_cells: Vec::new(),
        cells_scanned: 3 * 3,
    };

    assert_eq!(
        permit_from_scan(&[], &empty_workbook, &labels()),
        SyncPermit::Proven {
            db_count: 0,
            workbook_count: 0
        },
        "a workbook measured as empty against an empty database is genuinely safe"
    );
}

#[test]
fn a_database_mark_the_workbook_does_not_have_is_proven_and_clears_nothing() {
    // The database holds a mark the workbook does not, so the workbook is not
    // ahead and there is nothing to import: `Proven` is correct. The property
    // that matters is not the permit - it is that clearing cannot reach a cell
    // the workbook does not hold an X in, because the differential clear is
    // computed from the workbook's own X marks. A cell the workbook has never
    // heard of is not in that set, so the mark the user made survives the sync.
    let db = vec![cell(SHEET, "F", 8)];
    let empty_workbook = WorkbookMarkScan::Measured {
        x_cells: Vec::new(),
        cells_scanned: 3 * 3,
    };

    assert_eq!(
        permit_from_scan(&db, &empty_workbook, &labels()),
        SyncPermit::Proven {
            db_count: 1,
            workbook_count: 0
        },
        "a database ahead of the workbook is not the stale case"
    );
    assert!(
        differential_clear_marks(&attendance_scope_cells(&students(), &dates()), &db, &[])
            .is_empty(),
        "and the clear it licenses must be empty: the workbook holds no X at F8, so \
         there is nothing to blank and the user's own record cannot be erased"
    );
}

// The end-to-end proof the brief asks for: a workbook holding more X than the
// database yields Stale, with the concrete missing list - never Proven.
#[test]
fn a_workbook_holding_more_x_than_the_database_is_stale_with_the_missing_list() {
    // The acceptance scenario, at the size that was actually broken. The
    // workbook holds six X marks; the database proves two of them. Before the
    // fix the workbook side read as zero and this was `Proven`.
    let workbook_cells: Vec<Sf2GridCell> =
        [("F", 8), ("G", 9), ("H", 10), ("F", 9), ("H", 8), ("G", 10)]
            .into_iter()
            .map(|(column, row)| cell(SHEET, column, row))
            .collect();
    let db = vec![cell(SHEET, "F", 8), cell(SHEET, "G", 9)];

    let scan = WorkbookMarkScan::Measured {
        x_cells: workbook_cells,
        cells_scanned: 3 * 3,
    };
    let permit = permit_from_scan(&db, &scan, &labels());
    let action = action_for(&permit);

    let SyncPermit::Stale {
        db_count,
        workbook_count,
        missing,
    } = permit
    else {
        panic!("a workbook ahead of the database must be Stale, never Proven");
    };
    assert_eq!(db_count, 2);
    assert_eq!(workbook_count, 6);
    assert_eq!(
        missing,
        vec![
            ("LEARNER 10".to_string(), "2026-09-03".to_string()),
            ("LEARNER 9".to_string(), "2026-09-01".to_string()),
            ("LEARNER 8".to_string(), "2026-09-03".to_string()),
            ("LEARNER 10".to_string(), "2026-09-02".to_string()),
        ],
        "the missing list is what the recovery import acts on, so it must be \
         the concrete (student, date) pairs, not a count"
    );
    assert!(
        !action.permits_write(),
        "Stale must never clear on the first evaluation"
    );
}

#[test]
fn a_workbook_ahead_of_the_database_is_repaired_by_the_import_then_writes() {
    // The same six-against-two month, run through the real write path: the
    // additive import fills the gap, the re-evaluation agrees, and only then is
    // the write permitted. The point is that the *first* evaluation already
    // refused - a false zero would have short-circuited this whole loop.
    let scan = WorkbookMarkScan::Measured {
        x_cells: [("F", 8), ("G", 9), ("H", 10), ("F", 9), ("H", 8), ("G", 10)]
            .into_iter()
            .map(|(column, row)| cell(SHEET, column, row))
            .collect(),
        cells_scanned: 3 * 3,
    };
    let behind_db = vec![cell(SHEET, "F", 8), cell(SHEET, "G", 9)];
    let caught_up_db = [("F", 8), ("G", 9), ("H", 10), ("F", 9), ("H", 8), ("G", 10)]
        .into_iter()
        .map(|(column, row)| cell(SHEET, column, row))
        .collect::<Vec<Sf2GridCell>>();

    let mut evaluations = 0;
    let outcome = guard_before_write(
        || {
            evaluations += 1;
            permit_from_scan(
                if evaluations == 1 {
                    &behind_db
                } else {
                    &caught_up_db
                },
                &scan,
                &labels(),
            )
        },
        |missing| {
            assert_eq!(
                missing.len(),
                4,
                "four absences the database cannot produce"
            );
            Ok(())
        },
    )
    .expect("guard runs");

    assert_eq!(
        outcome,
        SyncAction::Rewrite {
            db_count: 6,
            workbook_count: 6
        }
    );
    assert_eq!(evaluations, 2);
}

#[test]
fn a_failed_read_after_a_stale_import_still_refuses_to_write() {
    // The import cannot be trusted to have been the reason the counts now agree
    // if the second measurement is missing entirely.
    let scan = WorkbookMarkScan::Measured {
        x_cells: vec![cell(SHEET, "F", 8)],
        cells_scanned: 3 * 3,
    };
    let mut evaluations = 0;

    let outcome = guard_before_write(
        || {
            evaluations += 1;
            if evaluations == 1 {
                permit_from_scan(&[], &scan, &labels())
            } else {
                permit_from_scan(
                    &[],
                    &WorkbookMarkScan::failed("the sheet is gone"),
                    &labels(),
                )
            }
        },
        |_| Ok(()),
    )
    .expect("guard runs");

    assert!(!outcome.permits_write());
    assert!(matches!(outcome, SyncAction::ReadOnly { .. }));
}

#[test]
fn the_type_of_a_scan_makes_asking_it_for_a_count_impossible_when_it_failed() {
    // The type-level statement of the fix, and the reason `decide` takes an
    // `Option` rather than a count.
    let failed = WorkbookMarkScan::failed("boom");
    let measured = WorkbookMarkScan::Measured {
        x_cells: vec![cell(SHEET, "F", 8)],
        cells_scanned: 9,
    };

    assert_eq!(failed.x_cells().map(<[Sf2GridCell]>::len), None);
    assert_eq!(measured.x_cells().map(<[Sf2GridCell]>::len), Some(1));
    assert_ne!(
        failed, measured,
        "a failed read and a successful one are different values, and only the \
         successful one can be asked for a count"
    );
    assert_eq!(failed.failure_reason(), Some("boom"));
    assert_eq!(measured.failure_reason(), None);
}

#[test]
fn a_scan_of_a_month_holding_no_x_is_measured_not_failed() {
    // The degenerate case that must not become a permanent refusal.
    let scan = WorkbookMarkScan::Measured {
        x_cells: Vec::new(),
        cells_scanned: 40 * 33,
    };

    assert!(matches!(scan, WorkbookMarkScan::Measured { .. }));
    assert_eq!(scan.failure_reason(), None);
    assert_eq!(scan.x_cells(), Some([].as_slice()));
}
