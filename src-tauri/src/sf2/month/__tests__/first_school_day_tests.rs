use super::*;

/// 2026-09-01 is a Tuesday, 2026-08-01 a Saturday, 2026-08-03 a Monday.
const SEPT_2026: (u32, i32) = (9, 2026);
const AUG_2026: (u32, i32) = (8, 2026);

fn date(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("test date")
}

#[test]
fn an_unset_start_date_yields_nothing_rather_than_a_guess() {
    // The controller's ruling: unset means unset. There is no "typical
    // Philippine school year" default anywhere in this module, because a
    // guessed start date silently mis-dates every month file.
    assert_eq!(
        derive_first_school_day(None, SEPT_2026.0, SEPT_2026.1),
        None
    );
    assert!(needs_school_start_date_prompt(None));
    assert!(!needs_school_start_date_prompt(Some(date("2026-08-03"))));
}

#[test]
fn a_month_before_the_start_date_is_dated_from_its_own_first_school_day() {
    // Classes start in August, so September is dated from its own first
    // Monday-Friday: 1 September 2026 is a Tuesday, so the answer is 1.
    assert_eq!(
        derive_first_school_day(Some(date("2026-08-03")), SEPT_2026.0, SEPT_2026.1),
        Some(1)
    );
}

#[test]
fn a_weekend_start_date_moves_to_the_next_monday() {
    // 1 August 2026 is a Saturday and 2 August a Sunday, so the first school
    // day on or after it is Monday the 3rd.
    assert_eq!(
        derive_first_school_day(Some(date("2026-08-01")), AUG_2026.0, AUG_2026.1),
        Some(3)
    );
    assert_eq!(
        derive_first_school_day(Some(date("2026-08-02")), AUG_2026.0, AUG_2026.1),
        Some(3)
    );
    assert_eq!(
        derive_first_school_day(Some(date("2026-08-03")), AUG_2026.0, AUG_2026.1),
        Some(3)
    );
}

#[test]
fn a_start_date_inside_the_month_dates_the_month_from_it() {
    // Classes start on 10 September 2026, a Thursday: the answer is 10.
    assert_eq!(
        derive_first_school_day(Some(date("2026-09-10")), SEPT_2026.0, SEPT_2026.1),
        Some(10)
    );
    // A Friday start date stays on the Friday.
    assert_eq!(
        derive_first_school_day(Some(date("2026-09-11")), SEPT_2026.0, SEPT_2026.1),
        Some(11)
    );
    // A Saturday start moves to the following Monday, 14 September 2026.
    assert_eq!(
        derive_first_school_day(Some(date("2026-09-12")), SEPT_2026.0, SEPT_2026.1),
        Some(14)
    );
    // The last day of the month, on a weekend, has nowhere to move to.
    // 30 September 2026 is a Wednesday, 26 is a Saturday.
    assert_eq!(
        derive_first_school_day(Some(date("2026-09-26")), SEPT_2026.0, SEPT_2026.1),
        Some(28)
    );
}

#[test]
fn a_month_that_ended_before_classes_started_has_no_school_day() {
    // Classes start on 3 August 2026, so every month of the *previous* school
    // year ended before the start date. No month file may be auto-created for
    // such a month, and asking for its first attendance day has to answer
    // "there isn't one" rather than invent a day (E2).
    let start = date("2026-08-03");
    for month in 1..=7 {
        assert_eq!(
            derive_first_school_day(Some(start), month, 2026),
            None,
            "month {month} of 2026 ended before classes started"
        );
    }
    // The start month itself is dated from the start date.
    assert_eq!(derive_first_school_day(Some(start), 8, 2026), Some(3));
    // A month of the NEXT school year is a normal month again: 1 September 2026
    // is a Tuesday, so the first school day is the 1st.
    assert_eq!(derive_first_school_day(Some(start), 9, 2027), Some(1));
    // And a school year whose start date is in January dates its own January
    // from the start date, not from the 1st.
    assert_eq!(
        derive_first_school_day(Some(date("2026-01-05")), 1, 2026),
        Some(5)
    );
}

#[test]
fn a_month_that_starts_on_a_weekend_uses_the_monday() {
    // 1 August 2026 is a Saturday, so August 2026 starts on Monday the 3rd.
    assert_eq!(
        derive_first_school_day(Some(date("2026-01-01")), AUG_2026.0, AUG_2026.1),
        Some(3)
    );
    // 1 November 2026 is a Sunday -> Monday the 2nd.
    assert_eq!(
        derive_first_school_day(Some(date("2026-01-01")), 11, 2026),
        Some(2)
    );
}

#[test]
fn leap_and_non_leap_february_both_work() {
    // 1 February 2027 is a Monday; 1 February 2028 is a Tuesday.
    assert_eq!(
        derive_first_school_day(Some(date("2026-01-04")), 2, 2027),
        Some(1)
    );
    assert_eq!(
        derive_first_school_day(Some(date("2028-02-01")), 2, 2028),
        Some(1)
    );
    // A leap day that exists only in 2028 is a valid start date.
    assert_eq!(
        derive_first_school_day(Some(date("2028-02-29")), 2, 2028),
        Some(29)
    );
    // 29 February 2027 does not exist, so it is not a date we can be given.
    assert!(NaiveDate::from_ymd_opt(2027, 2, 29).is_none());
    // February 2027 has 28 days, and a start date on the 28th is a Sunday, so
    // the month has no school day left to point at.
    assert_eq!(
        derive_first_school_day(Some(date("2027-02-28")), 2, 2027),
        None
    );
}

#[test]
fn an_impossible_month_yields_nothing() {
    assert_eq!(
        derive_first_school_day(Some(date("2026-01-05")), 0, 2026),
        None
    );
    assert_eq!(
        derive_first_school_day(Some(date("2026-01-05")), 13, 2026),
        None
    );
}

#[test]
fn is_school_day_rejects_only_the_weekend() {
    assert!(!is_school_day(date("2026-08-01"))); // Saturday
    assert!(!is_school_day(date("2026-08-02"))); // Sunday
    assert!(is_school_day(date("2026-08-03"))); // Monday
    assert!(is_school_day(date("2026-08-07"))); // Friday
    assert!(!is_school_day(date("2026-08-08"))); // Saturday again
    assert!(!is_school_day(date("2026-08-09"))); // Sunday again
}

#[test]
fn the_school_year_wraps_at_august() {
    // SEPTEMBER 2026 through AUGUST 2027: months from September on are the
    // start year, the rest is the following calendar year.
    let year_of = |month: u32| report_year_for_school_month("2026-2027", month, 1999);

    assert_eq!(year_of(9), 2026);
    assert_eq!(year_of(10), 2026);
    assert_eq!(year_of(11), 2026);
    assert_eq!(year_of(12), 2026);
    assert_eq!(year_of(1), 2027);
    assert_eq!(year_of(7), 2027);
    // The boundary itself: August belongs to the END of the school year, which
    // is what makes it the twelfth file and not the first.
    assert_eq!(year_of(8), 2027);
    assert_eq!(year_of(SCHOOL_YEAR_START_MONTH), 2026);
}

#[test]
fn the_school_year_start_year_is_read_from_the_label() {
    assert_eq!(school_year_start_year("2026-2027"), Some(2026));
    assert_eq!(school_year_start_year("SY 2026-2027"), Some(2026));
    assert_eq!(school_year_start_year("2026/2027"), Some(2026));
    assert_eq!(school_year_start_year("26-27"), None);
    assert_eq!(school_year_start_year(""), None);
    assert_eq!(school_year_start_year("SY 2026"), Some(2026));
}

#[test]
fn an_unreadable_school_year_uses_the_fallback_the_caller_supplied() {
    // The fallback is a parameter, not a hidden clock read, so this is
    // deterministic.
    assert_eq!(report_year_for_school_month("", 9, 1999), 1999);
    assert_eq!(report_year_for_school_month("n/a", 3, 1999), 1999);
    // A real year is never displaced by the fallback.
    assert_eq!(report_year_for_school_month("2026-2027", 3, 1999), 2027);
}

#[test]
fn an_override_always_wins_over_a_derived_day() {
    // Re-derivation runs on every startup; it must not be able to take the
    // value the user typed.
    assert_eq!(effective_first_school_day(Some(12), Some(1)), Some(12));
    assert_eq!(effective_first_school_day(Some(12), None), Some(12));
    assert_eq!(effective_first_school_day(None, Some(1)), Some(1));
    assert_eq!(effective_first_school_day(None, None), None);
}

#[test]
fn the_prompt_text_is_the_one_the_spec_asks_for() {
    assert_eq!(
        SCHOOL_START_DATE_PROMPT,
        "Enter the date classes started so each month's SF2 can be dated automatically."
    );
}

#[test]
fn every_month_of_a_school_year_can_be_dated() {
    // Walk the whole school year the way the app will: the first month is dated
    // from the start date, every later month from its own first school day,
    // and every answer lands inside its own month on a Monday-Friday.
    let start = date("2026-08-03");
    for (month, report_year) in [
        (9, 2026),
        (10, 2026),
        (11, 2026),
        (12, 2026),
        (1, 2027),
        (2, 2027),
        (3, 2027),
        (4, 2027),
        (5, 2027),
        (6, 2027),
        (7, 2027),
        (8, 2027),
    ] {
        let day = derive_first_school_day(Some(start), month, report_year)
            .unwrap_or_else(|| panic!("month {month} of {report_year} has no first school day"));
        let first = NaiveDate::from_ymd_opt(report_year, month, day).expect("valid date");
        let first_of_month = NaiveDate::from_ymd_opt(report_year, month, 1).expect("valid date");
        assert_eq!(first.month(), month, "the day must stay inside its month");
        assert!(
            is_school_day(first),
            "day {day} of month {month} is a weekend day"
        );
        // Every month is dated from the later of the start date and its own first
        // day, so a month can never be dated before classes started - and the
        // first month of the school year is dated from the start date rather
        // than from the 1st.
        assert!(
            first >= start.max(first_of_month),
            "month {month} of {report_year} was dated from {first}, which is before the \
             start date {start} or before the month itself"
        );
    }
}

// -- one canonical school-year label -------------------------------------
//
// The defect: the month tables are keyed on an exact school-year string, and
// the "which school year does this class have months for" query GLOBs for
// `NNNN-NNNN`. A user who types the DepEd form's own rendering, `2026 - 2027`,
// stores a label that matches no row and passes no GLOB - so the whole
// per-month table silently becomes unreachable and every read falls back to the
// legacy tables. Nothing errors. A whole school year of data is simply gone.

#[test]
fn every_spelling_of_a_school_year_normalises_to_one_form() {
    // The canonical form is `YYYY-YYYY`, which is what both the equality
    // lookups and the GLOB were written against.
    for written in [
        "2026-2027",     // already canonical
        "2026 - 2027",   // the real install's value: spaces around the dash
        "2026  -  2027", // more spaces, as typed by hand
        " 2026-2027 ",   // stray whitespace
        "2026 – 2027",   // en dash
        "2026—2027",     // em dash
        "SY 2026-2027",  // a prefixed label
        "S.Y. 2026-2027",
        "2026 / 2027",
    ] {
        assert_eq!(
            normalize_school_year(written),
            "2026-2027",
            "{written:?} must normalise to the one form the lookups are written against"
        );
    }
}

#[test]
fn normalisation_is_idempotent() {
    // Applied on read *and* on write, so applying it twice has to be a no-op or
    // a row would change under a reader that only reads.
    for once in ["2026-2027", "2026 - 2027", "SY 2026-2027"] {
        let twice = normalize_school_year(&normalize_school_year(once));
        assert_eq!(
            twice,
            normalize_school_year(once),
            "normalising {once:?} twice changed it"
        );
    }
}

#[test]
fn a_label_with_too_few_years_survives_normalisation() {
    // Normalisation is not validation. A label the user has not finished typing
    // must round-trip rather than be emptied - emptying it would lose the only
    // copy of what they entered.
    for unfinished in ["", "   ", "2026", "S.Y.", "20", "not a year"] {
        assert_eq!(
            normalize_school_year(unfinished),
            unfinished.trim(),
            "{unfinished:?} is not a school year and must not be rewritten into one"
        );
    }
}

#[test]
fn normalisation_keeps_the_start_year_the_report_year_arithmetic_reads() {
    // The two halves of this module have to agree: `normalize_school_year`
    // produces the label and `school_year_start_year` reads it, and a month
    // switch derives its calendar year from the result. If they disagreed, a
    // normalised label would silently mis-date a month.
    for written in ["2026-2027", "2026 - 2027", "SY 2026-2027", "2026 – 2027"] {
        let canonical = normalize_school_year(written);
        assert_eq!(
            school_year_start_year(&canonical),
            Some(2026),
            "{written:?}"
        );
        // SEPTEMBER onwards is the start year; AUGUST belongs to the next one.
        assert_eq!(
            report_year_for_school_month(&canonical, 9, 0),
            2026,
            "{written:?}"
        );
        assert_eq!(
            report_year_for_school_month(&canonical, 8, 0),
            2027,
            "{written:?}"
        );
    }
}

#[test]
fn a_normalised_label_passes_the_glob_the_latest_year_query_uses() {
    // The query is a GLOB, not a LIKE, and the space is what defeats it. This
    // asserts the property the SQL depends on, in the form the SQL reads it, so
    // a change to the canonical form that broke the GLOB fails here first.
    let glob = |label: &str| -> bool {
        let bytes: Vec<char> = label.chars().collect();
        bytes.len() == 9
            && bytes[4] == '-'
            && bytes[..4]
                .iter()
                .chain(bytes[5..].iter())
                .all(char::is_ascii_digit)
    };

    assert!(
        glob(&normalize_school_year("2026 - 2027")),
        "the canonical form must be exactly what the GLOB matches, or \
         month_latest_school_year.sql finds nothing"
    );
    assert!(
        !glob("2026 - 2027"),
        "the stored value this install has is exactly what the GLOB must not match - \
         that is the silent miss"
    );
}

#[test]
fn normalisation_never_invents_a_second_year() {
    // Unchanged by normalisation, and re-asserted because the new helper shares
    // the parsing. A DepEd version string carries one four-digit number, and a
    // single year is not a school year - so it is left as typed rather than
    // padded into `2014-2014`, which would be a year that never existed.
    assert_eq!(
        normalize_school_year("school_form_2_ver2014.2.1.1"),
        "school_form_2_ver2014.2.1.1",
        "one four-digit year is not a school year and must survive untouched"
    );
    assert_eq!(school_year_start_year("2026 - 2027"), Some(2026));
    assert_eq!(school_year_start_year(""), None);
}
