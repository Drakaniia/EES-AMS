use crate::domain::error::{AppError, Result};
use crate::sf2::models::{
    Sf2DateMappingRecord, Sf2TemplateDraft, Sf2TemplateRecord, Sf2WorkbookAnalysis,
    Sf2WorkbookMetadata,
};
use chrono::{Datelike, Local, NaiveDate};

pub(super) fn date_mappings_are_current_for_report_month(
    template: &Sf2TemplateRecord,
    date_mappings: &[Sf2DateMappingRecord],
) -> bool {
    let Some(month) = sf2_month_number(&template.report_month) else {
        return false;
    };
    let year = sf2_report_year(&template.school_year, month);
    let year_text = year.to_string();

    !date_mappings.is_empty()
        && date_mappings.iter().all(|mapping| {
            let Ok(date) = parse_date(&mapping.date) else {
                return false;
            };
            date.year() == year
                && date.month() == month
                && date.weekday().number_from_monday() <= 5
                && sf2_month_number(&mapping.sheet_name) == Some(month)
                && mapping.sheet_name.contains(&year_text)
        })
}

pub(super) fn date_mappings_from_analysis(
    template_id: &str,
    analysis: &Sf2WorkbookAnalysis,
) -> Vec<Sf2DateMappingRecord> {
    analysis
        .dates
        .iter()
        .map(|date| Sf2DateMappingRecord {
            template_id: template_id.to_string(),
            sheet_name: date.sheet_name.clone(),
            date: date.date.clone(),
            column_letter: date.column_letter.clone(),
            column_index: date.column_index,
        })
        .collect()
}

pub(super) fn metadata_from_analysis(analysis: &Sf2WorkbookAnalysis) -> Sf2WorkbookMetadata {
    Sf2WorkbookMetadata {
        school_id: analysis.school_id.trim().to_string(),
        school_name: analysis.school_name.trim().to_string(),
        school_year: analysis.school_year.trim().to_string(),
        report_month: analysis.report_month.trim().to_string(),
        grade_level: analysis.grade_level.trim().to_string(),
        section: analysis.section.trim().to_string(),
        adviser_name: analysis.adviser_name.trim().to_string(),
        school_head_name: analysis.school_head_name.trim().to_string(),
        configure_calendar: false,
        first_school_day: None,
    }
}

pub(super) fn metadata_from_import_analysis(
    analysis: &Sf2WorkbookAnalysis,
) -> Result<Sf2WorkbookMetadata> {
    let mut metadata = metadata_from_analysis(analysis);
    if sf2_month_number(&metadata.report_month).is_some() {
        metadata.configure_calendar = true;
        metadata.first_school_day = Some(first_school_day_for_report_month(
            &metadata.report_month,
            &metadata.school_year,
            analysis.dates.iter().map(|date| date.date.as_str()),
        )?);
    }

    Ok(metadata)
}

pub(super) fn metadata_from_draft(draft: &Sf2TemplateDraft) -> Result<Sf2WorkbookMetadata> {
    let school_year = required_draft_text(&draft.school_year, "School Year")?;
    let report_month = required_draft_text(&draft.report_month, "Report Month")?;
    let first_school_day =
        required_first_school_day(draft.first_school_day, &report_month, &school_year)?;

    Ok(Sf2WorkbookMetadata {
        school_id: required_draft_text(&draft.school_id, "School ID")?,
        school_name: required_draft_text(&draft.school_name, "Name of School")?,
        school_year,
        report_month,
        grade_level: required_draft_text(&draft.grade_level, "Grade Level")?,
        section: required_draft_text(&draft.section, "Section")?,
        adviser_name: required_draft_text(&draft.adviser_name, "Adviser / LIS Name")?,
        school_head_name: required_draft_text(&draft.school_head_name, "School Head Name")?,
        configure_calendar: true,
        first_school_day: Some(first_school_day),
    })
}

fn required_draft_text(value: &str, label: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput(format!("{label} is required")));
    }
    Ok(trimmed.to_string())
}

fn required_first_school_day(
    day: Option<u32>,
    report_month: &str,
    school_year: &str,
) -> Result<u32> {
    let day = day.ok_or_else(|| {
        AppError::InvalidInput("First attendance day is required for SF2 templates".to_string())
    })?;
    validate_first_school_day(day, report_month, school_year)?;
    Ok(day)
}

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

pub(super) fn sf2_report_year(_school_year: &str, _month: u32) -> i32 {
    Local::now().year()
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
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

pub(super) fn template_metadata(template: &Sf2TemplateRecord) -> Sf2WorkbookMetadata {
    Sf2WorkbookMetadata {
        school_id: template.school_id.clone(),
        school_name: template.school_name.clone(),
        school_year: template.school_year.clone(),
        report_month: template.report_month.clone(),
        grade_level: template.grade_level.clone(),
        section: template.section.clone(),
        adviser_name: template.adviser_name.clone(),
        school_head_name: template.school_head_name.clone(),
        configure_calendar: false,
        first_school_day: None,
    }
}

pub(super) fn sf2_metadata_warnings(metadata: &Sf2WorkbookMetadata) -> Vec<String> {
    [
        ("School ID", &metadata.school_id),
        ("Name of School", &metadata.school_name),
        ("School Year", &metadata.school_year),
        ("Report for the Month of", &metadata.report_month),
        ("Grade Level", &metadata.grade_level),
        ("Section", &metadata.section),
        (
            "Signature of Adviser over Printed Name / Generated thru LIS adviser name",
            &metadata.adviser_name,
        ),
        (
            "Signature of School Head over Printed Name",
            &metadata.school_head_name,
        ),
    ]
    .into_iter()
    .filter_map(|(label, value)| {
        let missing = value.trim().is_empty();
        missing.then(|| format!("{label} is blank in this SF2 workbook."))
    })
    .collect()
}

pub(super) fn sf2_date_mappings_for_report_month(
    template: &Sf2TemplateRecord,
    date_mappings: &[Sf2DateMappingRecord],
) -> Vec<Sf2DateMappingRecord> {
    let Some(month) = sf2_month_number(&template.report_month) else {
        return Vec::new();
    };
    let year = sf2_report_year(&template.school_year, month);

    date_mappings
        .iter()
        .filter_map(|mapping| {
            let date = parse_date(&mapping.date).ok()?;
            if date.month() != month {
                return None;
            }

            let normalized_date = NaiveDate::from_ymd_opt(year, month, date.day())?;
            if normalized_date.weekday().number_from_monday() > 5 {
                return None;
            }

            Some(Sf2DateMappingRecord {
                template_id: mapping.template_id.clone(),
                sheet_name: mapping.sheet_name.clone(),
                date: normalized_date.format("%Y-%m-%d").to_string(),
                column_letter: mapping.column_letter.clone(),
                column_index: mapping.column_index,
            })
        })
        .collect()
}

pub(super) fn parse_date(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| AppError::InvalidInput(format!("invalid date: {date}")))
}

pub(super) fn first_school_day_from_mappings(date_mappings: &[Sf2DateMappingRecord]) -> u32 {
    date_mappings
        .iter()
        .filter_map(|mapping| parse_date(&mapping.date).ok())
        .map(|date| date.day())
        .min()
        .unwrap_or(1)
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
mod tests {
    use super::*;

    #[test]
    fn no_events_means_no_sync_needed() {
        assert!(
            !attendance_changed_since(Some(1000), None),
            "with no attendance events, the workbook is already current"
        );
    }

    #[test]
    fn never_synced_with_events_requires_sync() {
        assert!(
            attendance_changed_since(None, Some(500)),
            "if the workbook was never synced but has events, we must sync"
        );
    }

    #[test]
    fn event_after_last_sync_requires_sync() {
        assert!(
            attendance_changed_since(Some(1000), Some(1001)),
            "an event newer than the last sync means the workbook is stale"
        );
    }

    #[test]
    fn event_equal_to_last_sync_skips_sync() {
        assert!(
            !attendance_changed_since(Some(1000), Some(1000)),
            "an event exactly at the last sync time is already written"
        );
    }

    #[test]
    fn event_before_last_sync_skips_sync() {
        assert!(
            !attendance_changed_since(Some(1000), Some(999)),
            "events older than the last sync are already reflected"
        );
    }
}
