use crate::domain::error::{AppError, Result};
use crate::sf2::models::{Sf2WorkbookAnalysis, Sf2WorkbookMetadata};
use chrono::{Datelike, Local, NaiveDate};

pub(super) fn first_school_day_for_report_month<'a, I>(
    report_month: &str,
    school_year: &str,
    dates: I,
) -> Result<u32>
where
    I: IntoIterator<Item = &'a str>,
{
    let month = sf2_month_number(report_month).ok_or_else(|| {
        AppError::InvalidInput("Report Month must be a valid month name".to_string())
    })?;
    let mut detected_days = dates
        .into_iter()
        .filter_map(|date| parse_date(date).ok())
        .filter(|date| date.month() == month)
        .map(|date| date.day())
        .collect::<Vec<_>>();

    detected_days.sort_unstable();
    detected_days.dedup();

    for day in detected_days {
        if validate_first_school_day(day, report_month, school_year).is_ok() {
            return Ok(day);
        }
    }

    default_sf2_first_school_day(report_month, school_year)
}

pub(super) fn default_sf2_first_school_day(report_month: &str, school_year: &str) -> Result<u32> {
    let month = sf2_month_number(report_month).ok_or_else(|| {
        AppError::InvalidInput("Report Month must be a valid month name".to_string())
    })?;
    let year = sf2_report_year(school_year, month);
    let last_day = last_day_of_month(year, month);

    (1..=last_day)
        .find(|day| validate_first_school_day(*day, report_month, school_year).is_ok())
        .ok_or_else(|| {
            AppError::Internal(
                "failed to find a Monday-Friday attendance day for this report month".to_string(),
            )
        })
}

pub(super) fn validate_first_school_day(
    day: u32,
    report_month: &str,
    school_year: &str,
) -> Result<()> {
    let month = sf2_month_number(report_month).ok_or_else(|| {
        AppError::InvalidInput("Report Month must be a valid month name".to_string())
    })?;
    let year = sf2_report_year(school_year, month);
    let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        let last_day = last_day_of_month(year, month);
        AppError::InvalidInput(format!(
            "First attendance day must be between 1 and {last_day} for this report month"
        ))
    })?;

    if date.weekday().number_from_monday() > 5 {
        return Err(AppError::InvalidInput(
            "First attendance day must be a Monday-Friday school day".to_string(),
        ));
    }

    Ok(())
}

pub(super) fn validate_configured_calendar(
    analysis: &Sf2WorkbookAnalysis,
    metadata: &Sf2WorkbookMetadata,
) -> Result<()> {
    if !metadata.configure_calendar {
        return Ok(());
    }

    let expected_day = metadata.first_school_day.ok_or_else(|| {
        AppError::InvalidInput("First attendance day is required for SF2 templates".to_string())
    })?;
    let month = sf2_month_number(&metadata.report_month).ok_or_else(|| {
        AppError::InvalidInput("Report Month must be a valid month name".to_string())
    })?;
    let year = sf2_report_year(&metadata.school_year, month);
    let detected_day = analysis
        .dates
        .iter()
        .filter_map(|mapping| parse_date(&mapping.date).ok())
        .filter(|date| date.year() == year && date.month() == month)
        .map(|date| date.day())
        .min();

    match detected_day {
        Some(actual_day) if actual_day == expected_day => Ok(()),
        Some(actual_day) => Err(AppError::Internal(format!(
            "SF2 calendar was not configured correctly: expected first attendance day {expected_day}, but the workbook starts at day {actual_day}"
        ))),
        None => Err(AppError::Internal(
            "SF2 calendar was not configured correctly: no attendance dates were detected"
                .to_string(),
        )),
    }
}

pub(super) fn sf2_month_number(name: &str) -> Option<u32> {
    let normalized = name.trim().to_ascii_uppercase();
    if normalized.contains("JAN") {
        Some(1)
    } else if normalized.contains("FEB") {
        Some(2)
    } else if normalized.contains("MAR") {
        Some(3)
    } else if normalized.contains("APR") {
        Some(4)
    } else if normalized.contains("MAY") {
        Some(5)
    } else if normalized.contains("JUN") {
        Some(6)
    } else if normalized.contains("JUL") {
        Some(7)
    } else if normalized.contains("AUG") {
        Some(8)
    } else if normalized.contains("SEP") {
        Some(9)
    } else if normalized.contains("OCT") {
        Some(10)
    } else if normalized.contains("NOV") {
        Some(11)
    } else if normalized.contains("DEC") {
        Some(12)
    } else {
        None
    }
}

pub(super) fn sf2_month_name(month: u32) -> &'static str {
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

pub(super) fn sf2_report_year(school_year: &str, month: u32) -> i32 {
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
        Local::now().year()
    }
}

pub(super) fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|date| date.pred_opt())
        .map(|date| date.day())
        .unwrap_or(31)
}

pub(super) fn parse_date(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| AppError::InvalidInput(format!("invalid date: {date}")))
}

/// Decide whether attendance marks must be rewritten to the Excel working copy.
///
/// Returns `true` when at least one attendance event was recorded (or updated)
/// *after* the last successful sync, meaning the workbook is stale and Excel
/// automation must run. Returns `false` when the workbook is already in sync,
/// so `sync_and_open_sf2_workbook` can skip the slow Excel write entirely.
///
/// `last_synced_at` is the timestamp (seconds) of the last successful mark
/// write; `None` means the workbook has never been synced, so we must sync.
/// `latest_event_at` is the most recent attendance event timestamp for the
/// class; `None` means there are no attendance events at all (nothing to write).
pub(super) fn attendance_changed_since(
    last_synced_at: Option<i64>,
    latest_event_at: Option<i64>,
) -> bool {
    let Some(latest_event_at) = latest_event_at else {
        return false;
    };
    match last_synced_at {
        None => true,
        Some(synced) => latest_event_at > synced,
    }
}

#[cfg(test)]
#[path = "__tests__/calendar_tests.rs"]
mod tests;
