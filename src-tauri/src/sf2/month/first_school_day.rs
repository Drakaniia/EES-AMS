//! Deriving a month file's first attendance day (spec D16, §11.1).
//!
//! ```text
//! first_school_day(MONTH, YEAR) =
//!     the first Mon-Fri day on or after max(school_start_date, first day of MONTH)
//!     clamped into MONTH
//! ```
//!
//! Everything here is pure: no IO, no database, no COM, and no clock. The
//! caller supplies the calendar year, so the whole derivation is a function of
//! its arguments and can be tested exhaustively - which matters, because a
//! wrong day here silently mis-dates a month file and that is the exact class
//! of bug the per-month model exists to remove.
//!
//! ## `school_start_date` is not defaulted
//!
//! There is no "typical Philippine school year" fallback here. An unset
//! `school_start_date` stays unset and [`derive_first_school_day`] returns
//! `None`; the caller falls back to the legacy per-month value and prompts once
//! ([`needs_school_start_date_prompt`], spec edge case E3). A guessed default
//! would be wrong for every school that does not start on that Monday, and
//! nothing downstream would notice.

use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};

/// The first month of the school year, September.
///
/// A Philippine school year runs SEPTEMBER -> AUGUST, so the school year
/// `2026-2027` covers SEPTEMBER 2026 through AUGUST 2027. Months from
/// September on belong to the start year; the rest belong to the following
/// calendar year. The v22 backfill applies the same rule in SQL
/// (`migrate_to_v22.sql`).
pub const SCHOOL_YEAR_START_MONTH: u32 = 9;

/// Shown once, when the real start date has not been entered yet (spec E3).
pub const SCHOOL_START_DATE_PROMPT: &str =
    "Enter the date classes started so each month's SF2 can be dated automatically.";

/// Which calendar year a month of the school year falls in.
///
/// `fallback_year` is used when `school_year` holds no four-digit year. The
/// fallback is a parameter rather than a hidden `Local::now()` read so this
/// function stays pure and the caller decides what "we do not know" means.
#[must_use]
pub fn report_year_for_school_month(
    school_year: &str,
    report_month: u32,
    fallback_year: i32,
) -> i32 {
    match school_year_start_year(school_year) {
        Some(start_year) if report_month >= SCHOOL_YEAR_START_MONTH => start_year,
        Some(start_year) => start_year + 1,
        None => fallback_year,
    }
}

/// The first four-digit year in a school-year label, e.g. `2026` for
/// `2026-2027` or `SY 2026-2027`.
#[must_use]
pub fn school_year_start_year(school_year: &str) -> Option<i32> {
    school_year_years(school_year).first().copied()
}

/// The **canonical** form of a school-year label: `YYYY-YYYY`, no spaces.
///
/// ## Why one form has to exist
///
/// A month row is looked up by exact equality on `(class, school_year, month)`,
/// and the "latest school year" query GLOBs for
/// `[0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9]`. Both were written against
/// `2026-2027`. A user who types `2026 - 2027` - with spaces, the way the DepEd
/// form prints it - therefore produces a label that matches **no** month row and
/// passes **no** GLOB: the whole per-month table silently becomes unreachable
/// and every read falls back to the legacy tables. That is not a display
/// inconsistency, it is data that cannot be found.
///
/// So the label is normalised on **both** sides of every boundary - on write, so
/// nothing new is stored un-normalised, and on read, so a row that predates the
/// normalisation is still found - and schema migration v23
/// (`sf2/sql/migrate_to_v23.sql`) backfills the rows already on disk.
///
/// ## What it does not do
///
/// A label with fewer than two four-digit years is returned trimmed but
/// otherwise untouched. This function is not a validator: a label the user has
/// not finished typing must survive a round trip rather than be emptied, and
/// [`school_year_start_year`] is what decides whether a label is usable.
#[must_use]
pub fn normalize_school_year(school_year: &str) -> String {
    let years = school_year_years(school_year);
    match years.as_slice() {
        [start, end] => format!("{start:04}-{end:04}"),
        _ => school_year.trim().to_string(),
    }
}

/// Every four-digit year in a label, in the order they appear, keeping only the
/// plausible ones (1900-2999) and the first of each repeated value.
///
/// The bound is what makes `school_form_2_ver2014.2.1.1` yield no year at all
/// rather than `2014` twice, and it is the same bound
/// [`school_year_start_year`] has always applied.
fn school_year_years(school_year: &str) -> Vec<i32> {
    let mut years: Vec<i32> = Vec::new();
    for part in school_year.split(|ch: char| !ch.is_ascii_digit()) {
        let Ok(year) = part.parse::<i32>() else {
            continue;
        };
        if part.len() == 4 && (1900..=2999).contains(&year) && !years.contains(&year) {
            years.push(year);
        }
    }
    years
}

/// Monday to Friday. A Philippine school week is five days; Saturday and Sunday
/// are never attendance days.
#[must_use]
pub fn is_school_day(date: NaiveDate) -> bool {
    !matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

/// The first attendance day of a month, or `None` when it cannot be known.
///
/// `None` means one of exactly two things, and the caller has to tell them
/// apart with [`needs_school_start_date_prompt`]:
///
/// * `school_start_date` is unset - the user has not been asked yet.
/// * Classes start after the month ends, so the month has no school days at
///   all (spec edge case E2: never auto-create a file for such a month).
#[must_use]
pub fn derive_first_school_day(
    school_start_date: Option<NaiveDate>,
    report_month: u32,
    report_year: i32,
) -> Option<u32> {
    // Unset means unset. See the module docs.
    let school_start_date = school_start_date?;
    let first_of_month = NaiveDate::from_ymd_opt(report_year, report_month, 1)?;
    let last_of_month = last_day_of_month(report_year, report_month)?;

    // A month before classes started is dated from its own first school day; a
    // month classes started in the middle of is dated from the start date.
    let mut candidate = first_of_month.max(school_start_date);
    if candidate > last_of_month {
        return None;
    }

    // First Monday-Friday on or after the candidate, still inside the month.
    while !is_school_day(candidate) {
        candidate += Duration::days(1);
        if candidate > last_of_month {
            return None;
        }
    }

    // "Clamped into MONTH" holds by construction; keep the check so a future
    // change to the arithmetic cannot leak a day from the next month.
    if candidate.month() != report_month || candidate.year() != report_year {
        return None;
    }
    Some(candidate.day())
}

/// Should the app ask for the real start date?
///
/// True exactly while `school_start_date` is unset, which is the "prompt once"
/// condition of spec E3.
#[must_use]
pub fn needs_school_start_date_prompt(school_start_date: Option<NaiveDate>) -> bool {
    school_start_date.is_none()
}

/// Resolve a month file's first attendance day from what is stored on its row.
///
/// An override always wins. Re-derivation is free to run as often as it likes -
/// a change to `school_start_date`, a re-run of the split - and must never
/// discard a day the user typed (spec §11.1).
#[must_use]
pub fn effective_first_school_day(overridden: Option<u32>, derived: Option<u32>) -> Option<u32> {
    overridden.or(derived)
}

/// The last calendar day of a month.
fn last_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)?.pred_opt()
}

/// The calendar year to fall back on when a school year holds no year.
///
/// The one impure thing in the month model, kept behind a named function so
/// [`report_year_for_school_month`] itself stays a pure function of its
/// arguments and the "we do not know" case is visible at every call site.
#[must_use]
pub fn current_year() -> i32 {
    Local::now().year()
}

#[cfg(test)]
#[path = "__tests__/first_school_day_tests.rs"]
mod tests;
