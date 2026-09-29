//! The worksheet-naming rules of the single-file, twelve-sheet workbook (spec
//! §0 A1, A2).
//!
//! The rules that matter are all about *not* mistaking one worksheet for another:
//! a leftover `__SF2_HIDDEN_{n}` must never read as a month, and a month must
//! never read as a year it is not. Nothing here opens Excel.

use super::*;

#[test]
fn a_retired_hidden_tab_is_recognised_so_it_can_be_removed() {
    assert!(is_hidden_sheet_name("__SF2_HIDDEN_1"));
    assert!(is_hidden_sheet_name("__SF2_HIDDEN_12"));
    // Case and padding do not matter: the name came from a loop, not from a user.
    assert!(is_hidden_sheet_name(" __sf2_hidden_3 "));
}

#[test]
fn a_month_worksheet_is_not_a_hidden_tab() {
    assert!(!is_hidden_sheet_name("SEPTEMBER 2026"));
    assert!(!is_hidden_sheet_name("COMPLETE DAYS"));
}

#[test]
fn a_month_worksheet_needs_a_month_and_a_four_digit_year() {
    assert!(is_month_sheet_name("SEPTEMBER 2026"));
    // The bundled template ships this spelling.
    assert!(is_month_sheet_name("SEPT. 2025"));
    assert!(is_month_sheet_name("JUNE2025"));
}

#[test]
fn a_name_without_a_year_is_not_a_month_worksheet() {
    // This is the property that makes "never import the template's sample data"
    // structural: `JUNE` with no year, `__SF2_HIDDEN_1`, and a helper sheet all
    // fail this test, so `read_legacy_months` can never return one as a month.
    assert!(!is_month_sheet_name("JUNE"));
    assert!(!is_month_sheet_name("__SF2_HIDDEN_1"));
    assert!(!is_month_sheet_name("COMPLETE DAYS"));
}

#[test]
fn a_name_with_a_year_that_is_not_this_century_is_not_a_month_worksheet() {
    assert!(!is_month_sheet_name("JUNE 1899"));
    assert!(!is_month_sheet_name("JUNE 20"));
    assert!(!is_month_sheet_name("JUNE 20255"));
}

#[test]
fn a_month_worksheet_is_matched_by_month_alone_for_a_year_the_caller_chooses() {
    assert!(is_month_sheet_of("SEPTEMBER 2026", 9));
    assert!(is_month_sheet_of("SEPT. 2025", 9));
    assert!(!is_month_sheet_of("OCTOBER 2026", 9));
    assert!(!is_month_sheet_of("__SF2_HIDDEN_1", 9));
}

#[test]
fn the_canonical_name_is_the_full_month_name_the_database_stores() {
    // `month_workbook_sheet_name` in `workbook_files` canonicalises whatever
    // spelling it is given; both must land on the same string or a write
    // addressed by a stored `sheet_name` would not find its worksheet.
    assert_eq!(month_sheet_name(9, 2026), "SEPTEMBER 2026");
    assert_eq!(
        crate::sf2::workbook_files::month_workbook_sheet_name("Sept.", 2026),
        month_sheet_name(9, 2026)
    );
    assert_eq!(
        crate::sf2::workbook_files::month_workbook_sheet_name("september", 2026),
        month_sheet_name(9, 2026)
    );
}

#[test]
fn a_weekday_label_becomes_a_monday_to_friday_index() {
    assert_eq!(parse_weekday_label("M"), Some(0));
    assert_eq!(parse_weekday_label("MON"), Some(0));
    assert_eq!(parse_weekday_label(" T "), Some(1));
    assert_eq!(parse_weekday_label("W"), Some(2));
    // "TH" is Thursday, "T" is Tuesday. Getting this wrong swaps two days of
    // every month.
    assert_eq!(parse_weekday_label("TH"), Some(3));
    assert_eq!(parse_weekday_label("THU"), Some(3));
    assert_eq!(parse_weekday_label("F"), Some(4));
    assert_eq!(parse_weekday_label("FRI"), Some(4));
}

#[test]
fn a_label_that_is_not_a_weekday_is_not_a_day_column() {
    // These are the columns to the right of the grid. Reading one as a weekday
    // would add a 26th day cell and shift every mark after it.
    assert_eq!(parse_weekday_label("ABSENT"), None);
    assert_eq!(parse_weekday_label("PRESENT"), None);
    assert_eq!(parse_weekday_label(""), None);
    assert_eq!(parse_weekday_label("  "), None);
}
