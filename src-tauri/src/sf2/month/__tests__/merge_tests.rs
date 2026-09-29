//! The pure decision rules of the merge job (spec §0 A1, A4, A5).
//!
//! Nothing here opens Excel or writes to the user's workbook. What is tested is
//! the arithmetic that decides which months count, what the sentence says, where
//! a month's day grid is anchored, and where each gender block starts - the four
//! things that are wrong in ways no compiler can see.

use super::*;
use crate::sf2::month::workbook_sheets::{month_sheet_name, MONTH_SHEET_NAME_MAX};

fn month(name: &str, year: i32, status: MergeMonthStatus) -> MergeMonthOutcome {
    MergeMonthOutcome {
        report_month: name.to_string(),
        report_year: year,
        sheet_name: month_sheet_name(sf2_month_number(name).unwrap_or_default(), year),
        file_name: "SF2-GRADE-3-MATAPAT-3b635890.xls".to_string(),
        status,
        x_marks: 0,
        learner_rows: 27,
        detail: None,
    }
}

fn all_verified() -> Vec<MergeMonthOutcome> {
    school_year_month_files("2026-2027", 1999)
        .into_iter()
        .map(|(name, year)| month(&name, year, MergeMonthStatus::Verified))
        .collect()
}

// ── Completeness ────────────────────────────────────────────────────────────

#[test]
fn a_merge_is_complete_only_when_all_twelve_months_verify() {
    let months = all_verified();
    assert_eq!(months.len(), SPLIT_MONTH_COUNT);
    assert!(is_merge_complete(&months));
}

#[test]
fn one_unproven_month_leaves_the_merge_incomplete() {
    let mut months = all_verified();
    months[3].status = MergeMonthStatus::NeedsAttention;
    assert!(!is_merge_complete(&months));
}

#[test]
fn a_month_an_earlier_run_already_merged_counts_as_accounted_for() {
    let mut months = all_verified();
    months[0].status = MergeMonthStatus::AlreadyMerged;
    assert!(is_merge_complete(&months));
}

#[test]
fn a_short_month_list_is_never_complete() {
    let months = all_verified().drain(..6).collect::<Vec<_>>();
    assert!(!is_merge_complete(&months));
}

#[test]
fn the_school_year_runs_september_to_august_with_the_year_wrapping() {
    let months = school_year_month_files("2026-2027", 1999);
    assert_eq!(months.first(), Some(&(String::from("SEPTEMBER"), 2026)));
    assert_eq!(months.last(), Some(&(String::from("AUGUST"), 2027)));
    assert!(months.iter().all(|(month, year)| match month.as_str() {
        "SEPTEMBER" | "OCTOBER" | "NOVEMBER" | "DECEMBER" => *year == 2026,
        _ => *year == 2027,
    }));
}

// ── The sentence ────────────────────────────────────────────────────────────

#[test]
fn the_summary_says_one_workbook_with_twelve_visible_worksheets() {
    let message = merge_summary_message(Some(1), &all_verified(), "SF2-1-A-1a2b3c4d.xls");
    assert!(
        message.contains("All 12 months are on one workbook"),
        "the sentence must say the file model changed: {message}"
    );
    assert!(
        message.contains("its own visible worksheet"),
        "the sentence must say no month is hidden: {message}"
    );
}

#[test]
fn the_summary_always_says_where_the_original_workbook_is_kept() {
    for completed in [Some(1), None] {
        let message = merge_summary_message(completed, &all_verified(), "SF2-1-A-1a2b3c4d.xls");
        assert!(
            message.contains("_legacy") && message.contains("SF2-1-A-1a2b3c4d.xls"),
            "the merge rewrites the file the original marks lived in, so the copy has to be \
             named: {message}"
        );
    }
}

#[test]
fn the_summary_names_the_months_that_need_attention() {
    let mut months = all_verified();
    months[2].status = MergeMonthStatus::NeedsAttention;
    months[7].status = MergeMonthStatus::NeedsAttention;
    let message = merge_summary_message(None, &months, "SF2-1-A-1a2b3c4d.xls");
    assert!(message.contains("2 needs attention"), "{message}");
    // The school year runs SEPTEMBER -> AUGUST, so the third and eighth months of
    // 2026-2027 are NOVEMBER 2026 and APRIL 2027 - not the calendar-order months,
    // which would name the wrong year for a teacher reading the list.
    assert!(message.contains("NOVEMBER 2026"), "{message}");
    assert!(message.contains("APRIL 2027"), "{message}");
    assert_eq!(
        needs_attention_labels(&months),
        vec!["NOVEMBER 2026".to_string(), "APRIL 2027".to_string()]
    );
}

#[test]
fn a_finished_merge_counts_every_month_as_verified() {
    let months = all_verified();
    assert_eq!(verified_count(&months), SPLIT_MONTH_COUNT);
}

// ── The day grid anchor ─────────────────────────────────────────────────────

#[test]
fn a_known_first_school_day_is_the_grid_anchor() {
    assert_eq!(grid_anchor_day(Some(8), 2026, 9), 8);
}

#[test]
fn an_undated_month_still_gets_a_grid_that_holds_every_school_day() {
    // SEPTEMBER 2026 on the install this was written for: 17 absences, no date
    // mappings at all, and no `school_start_date` entered. Anchoring on the 1st
    // is what gives every one of those absences a column; a narrower grid would
    // leave a recorded absence with nowhere to go, which is the reported bug.
    assert_eq!(grid_anchor_day(None, 2026, 9), 1);
}

#[test]
fn a_first_school_day_outside_the_month_is_not_believed() {
    // 0 is `FIRST_SCHOOL_DAY_UNDETERMINED`, and 31 is not a day of SEPTEMBER.
    assert_eq!(grid_anchor_day(Some(0), 2026, 9), 1);
    assert_eq!(grid_anchor_day(Some(31), 2026, 9), 1);
    assert_eq!(grid_anchor_day(Some(30), 2026, 9), 30);
}

// ── The roster layout ───────────────────────────────────────────────────────

#[test]
fn males_start_on_row_eight() {
    // The first female row is the one after the MALE TOTAL row, which a bundled
    // template puts at row 29 - so row 30, which is exactly where the real
    // install's female block starts.
    assert_eq!(female_block_start(17), 30);
    assert_eq!(female_block_start(0), 30);
}

#[test]
fn the_female_block_grows_only_past_twenty_one_males() {
    // A bundled template has room for 21 males before it has to grow, so a
    // roster of 21 or fewer puts the female block on the same row every time -
    // which is what lets one row index mean the same learner on all twelve sheets.
    assert_eq!(female_block_start(21), 30);
    assert_eq!(female_block_start(25), 34);
    assert_eq!(female_block_start(40), 49);
}

#[test]
fn the_female_block_agrees_with_where_the_build_puts_the_male_total_row() {
    for males in [0usize, 1, 17, 21, 25, 40] {
        let (male_total, _, _) =
            crate::sf2::roster::roster_parser::bundled_template_total_rows(males, 10);
        assert_eq!(
            female_block_start(males),
            male_total + 1,
            "with {males} males the roster and the TOTAL-row formulas have to agree, or a \
             learner's row index means one thing to the roster and another to the COUNTIF"
        );
    }
}

// ── The worksheet names ─────────────────────────────────────────────────────

#[test]
fn a_month_worksheet_is_named_for_its_month_and_year() {
    assert_eq!(month_sheet_name(9, 2026), "SEPTEMBER 2026");
    assert_eq!(month_sheet_name(8, 2027), "AUGUST 2027");
    assert_eq!(month_sheet_name(12, 2026), "DECEMBER 2026");
}

#[test]
fn no_worksheet_name_can_reach_excels_limit() {
    for month in 1..=12u32 {
        for year in [2026, 2027] {
            let name = month_sheet_name(month, year);
            assert!(
                name.chars().count() <= MONTH_SHEET_NAME_MAX,
                "`{name}` is too long for Excel"
            );
        }
    }
}
