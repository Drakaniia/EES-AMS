// ── Utility functions ─────────────────────────────────────────────────────────
//
// Pure utility functions extracted from workbook.rs. No COM dependencies.
// These are used by both workbook.rs (COM operations) and directly by other
// modules that need Excel workbook sheet-name utilities.

use chrono::Datelike;

/// Parse a sheet name to determine which month (1-12) it represents.
/// Returns 0 if the name doesn't match any known month abbreviation.
pub fn month_number(name: &str) -> u32 {
    let normalized = name.to_uppercase();
    if normalized.contains("JAN") {
        1
    } else if normalized.contains("FEB") {
        2
    } else if normalized.contains("MAR") {
        3
    } else if normalized.contains("APR") {
        4
    } else if normalized.contains("MAY") {
        5
    } else if normalized.contains("JUN") {
        6
    } else if normalized.contains("JUL") {
        7
    } else if normalized.contains("AUG") {
        8
    } else if normalized.contains("SEP") {
        9
    } else if normalized.contains("OCT") {
        10
    } else if normalized.contains("NOV") {
        11
    } else if normalized.contains("DEC") {
        12
    } else {
        0
    }
}

/// Convert a month number (1-12) to its uppercase English name.
pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "JANUARY",
        2 => "FEBRUARY",
        3 => "MARCH",
        4 => "APRIL",
        5 => "MAY",
        6 => "JUNE",
        7 => "JULY",
        8 => "AUGUST",
        9 => "SEPTEMBER",
        10 => "OCTOBER",
        11 => "NOVEMBER",
        12 => "DECEMBER",
        _ => "",
    }
}

/// Extract a numeric year from an SF2 school-year string.
///
/// Given a school year in "START-END" format (e.g., "2024-2025"), returns:
/// - `start_year` for months June-December (month >= 6)
/// - `start_year + 1` for months January-May (month < 6)
///
/// Falls back to the current calendar year if parsing fails.
pub fn report_year(school_year: &str, month: u32) -> i32 {
    // Parse "START-END" school year format (e.g., "2024-2025").
    // Months June-December (>=6) use the start year; January-May (<6) use end year.
    if let Some(start_year) = school_year
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| part.len() == 4 && part.starts_with("20"))
        .filter_map(|part| part.parse::<i32>().ok())
        .next()
    {
        if month >= 6 {
            start_year
        } else {
            start_year + 1
        }
    } else {
        chrono::Local::now().year_ce().1 as i32
    }
}

/// Parse a 4-digit year from a sheet name (e.g., "JUNE2024" -> 2024).
/// Returns 0 if no 4-digit year starting with "20" is found.
pub fn year_from_sheet_name(name: &str) -> i32 {
    name.split(|ch: char| !ch.is_ascii_digit())
        .find_map(|part| {
            (part.len() == 4 && part.starts_with("20"))
                .then(|| part.parse::<i32>().ok())
                .flatten()
        })
        .unwrap_or(0)
}

/// Convert a 1-based column number to its Excel column letter(s).
/// (e.g., 1 -> "A", 26 -> "Z", 27 -> "AA")
pub fn column_number_to_letter(mut column: i32) -> String {
    let mut letter = String::new();
    while column > 0 {
        let modulo = (column - 1) % 26;
        letter.insert(0, (b'A' + modulo as u8) as char);
        column = (column - modulo) / 26;
    }
    letter
}

/// Case-insensitive substring check (ASCII only).
pub fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    haystack.to_uppercase().contains(&needle.to_uppercase())
}

/// Determine whether a worksheet should be used as an analysis source based on
/// its sheet name, title, and visibility.
///
/// - Monthly sheets (name contains a month abbreviation + 4-digit year starting
///   with "20") are always candidates — they have proper date headers.
/// - Non-monthly sheets are candidates ONLY if their cell A1 contains "School Form 2"
///   (indicating they are genuine SF2 forms despite the non-standard sheet name).
/// - Hidden sheets are never included.
pub fn sheet_is_analysis_candidate(name: &str, title: &str, visible: i32) -> bool {
    if visible != -1 {
        return false;
    }

    let mn = month_number(name);
    let yr = year_from_sheet_name(name);
    if mn > 0 && yr > 0 {
        // Monthly sheet — always a candidate
        return true;
    }

    // Non-monthly sheet — only a candidate if it is a genuine SF2 form
    contains_ignore_ascii_case(title, "School Form 2")
}
