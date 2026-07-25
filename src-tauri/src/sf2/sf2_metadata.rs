use crate::domain::error::{AppError, Result};
use crate::sf2::calendar::{
    first_school_day_for_report_month, parse_date, sf2_month_number, sf2_report_year,
    validate_first_school_day,
};
use crate::sf2::models::{
    Sf2DateMappingRecord, Sf2TemplateDraft, Sf2TemplateRecord, Sf2WorkbookAnalysis,
    Sf2WorkbookMetadata,
};
use chrono::{Datelike, NaiveDate};

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
        && date_mappings.iter().any(|mapping| {
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

pub(super) fn first_school_day_from_mappings(date_mappings: &[Sf2DateMappingRecord]) -> u32 {
    date_mappings
        .iter()
        .filter_map(|mapping| parse_date(&mapping.date).ok())
        .map(|date| date.day())
        .min()
        .unwrap_or(1)
}

#[cfg(test)]
#[path = "__tests__/sf2_metadata_tests.rs"]
mod tests;
