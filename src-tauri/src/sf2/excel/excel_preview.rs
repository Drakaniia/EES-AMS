use crate::domain::error::Result;
use crate::infrastructure::database::{ClassRepository, EventRepository, StudentRepository};
use crate::sf2::calendar::{last_day_of_month, sf2_month_number, sf2_report_year};
use crate::sf2::models::{
    Sf2DateMappingRecord, Sf2ExportPreview, Sf2ExportReadiness, Sf2PreviewDate, Sf2TemplateRecord,
};
use crate::sf2::preview;
use crate::sf2::repository::template_summary;
use crate::sf2::sf2_metadata::{
    sf2_date_mappings_for_report_month, sf2_metadata_warnings, template_metadata,
};

use chrono::{Datelike, NaiveDate};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub fn export_preview(
    pool: crate::infrastructure::database::DbPool,
    class_id: Option<String>,
) -> Result<Sf2ExportPreview> {
    let sf2_repo = crate::sf2::repository::Sf2Repository::new(pool.clone());

    // Query template and associated data ONCE — no duplicate round-trips.
    let template = match class_id {
        Some(ref class_id) if !class_id.is_empty() => {
            sf2_repo.latest_template_for_class(class_id)?
        }
        _ => sf2_repo
            .list_templates()?
            .into_iter()
            .next()
            .map(|summary| Sf2TemplateRecord {
                id: summary.id,
                source_path: summary.source_path,
                source_hash: String::new(),
                school_id: summary.school_id,
                school_name: summary.school_name,
                school_year: summary.school_year,
                report_month: summary.report_month,
                grade_level: summary.grade_level,
                section: summary.section,
                adviser_name: summary.adviser_name,
                school_head_name: summary.school_head_name,
                layout_fingerprint: String::new(),
                active_class_id: summary.class_id,
                imported_at: summary.imported_at,
                last_synced_at: None,
            }),
    };

    let Some(template) = template else {
        return Ok(Sf2ExportPreview {
            template: None,
            class_id: None,
            class_name: String::new(),
            source_path: None,
            dates: Vec::new(),
            students: Vec::new(),
            absent_list: Vec::new(),
            mapped_students: 0,
            mapped_dates: 0,
            present_count: 0,
            absence_count: 0,
            unmapped_student_count: 0,
            can_export: false,
            issues: vec![
                "Import an SF2 workbook or create one from the bundled template before exporting."
                    .to_string(),
            ],
            warnings: Vec::new(),
        });
    };

    // ── Query supporting data (no redundant queries) ──────────────────────
    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let all_date_mappings = sf2_repo.date_mappings_for_template(&template.id)?;
    let date_mappings = sf2_date_mappings_for_report_month(&template, &all_date_mappings);

    // Expand preview dates to ALL weekdays of the report month, not just
    // mapped SF2 columns.
    let preview_dates = expand_to_all_weekdays(&template, &date_mappings);

    // Build readiness-like checks from already-queried data (no re-querying)
    let mut issues: Vec<String> = Vec::new();
    if !Path::new(&template.source_path).exists() {
        issues.push(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again."
                .to_string(),
        );
    }

    // Unmapped students
    let class_students =
        StudentRepository::new(pool.clone()).list_by_class(Some(&template.active_class_id))?;
    let mapped_student_ids: HashSet<String> = student_mappings
        .iter()
        .map(|m| m.student_id.clone())
        .collect();
    let unmapped_names: Vec<String> = class_students
        .iter()
        .filter(|s| !mapped_student_ids.contains(&s.id.to_string()))
        .map(|s| s.name.clone())
        .collect();
    if !unmapped_names.is_empty() {
        let shown = unmapped_names
            .iter()
            .take(5)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        let more = unmapped_names.len().saturating_sub(5);
        let suffix = if more > 0 {
            format!(", and {more} more")
        } else {
            String::new()
        };
        issues.push(format!(
            "{}{} {} not mapped to an SF2 learner row. Sync the SF2 workbook roster before exporting.",
            shown,
            suffix,
            if unmapped_names.len() == 1 { "is" } else { "are" }
        ));
    }

    // Date check
    if date_mappings.is_empty() {
        issues.push("No attendance dates are mapped to this SF2 report month.".to_string());
    }

    let warnings = sf2_metadata_warnings(&template_metadata(&template));
    let mapped_students = student_mappings.len();
    let mapped_dates = date_mappings.len();

    let can_export = issues.is_empty();

    // Class name
    let class_repo = ClassRepository::new(pool.clone());
    let class = class_repo.get(&template.active_class_id)?;
    let class_name = class.map(|c| c.name).unwrap_or_else(|| {
        crate::sf2::naming::class_name(&template.grade_level, &template.section)
    });

    // ── Filtered events (only for this class + this month) ────────────────
    let events = if !preview_dates.is_empty() && !date_mappings.is_empty() {
        let first_date = &preview_dates[0].date;
        let last_date = &preview_dates[preview_dates.len() - 1].date;
        EventRepository::new(pool.clone()).list_for_class_and_date_range(
            &template.active_class_id,
            first_date,
            last_date,
        )?
    } else {
        Vec::new()
    };

    let readiness = Sf2ExportReadiness {
        template: Some(template_summary(template.clone())),
        mapped_students,
        mapped_dates,
        can_export,
        issues,
        warnings,
    };

    preview::export_preview(
        &template,
        &student_mappings,
        &preview_dates,
        &class_name,
        &class_students,
        &events,
        readiness,
    )
}

/// Expand SF2 date mappings to include ALL weekdays of the report month.
///
/// For each Monday–Friday day in the month:
/// - If a date mapping exists (SF2 column was detected in the workbook), use it.
/// - Otherwise, create a placeholder `Sf2PreviewDate` with empty sheet_name,
///   column_letter, and 0 column_index. These placeholders are used in the
///   preview grid so every weekday shows a clickable cell regardless of SF2
///   mapping. Unmapped dates are naturally filtered out during Excel export
///   by `write_template_marks_for_days`.
fn expand_to_all_weekdays(
    template: &Sf2TemplateRecord,
    date_mappings: &[Sf2DateMappingRecord],
) -> Vec<Sf2PreviewDate> {
    let Some(month) = sf2_month_number(&template.report_month) else {
        // Invalid report month — fall back to just the mapped dates
        return date_mappings
            .iter()
            .map(|m| Sf2PreviewDate {
                date: m.date.clone(),
                sheet_name: m.sheet_name.clone(),
                column_letter: m.column_letter.clone(),
                column_index: m.column_index,
            })
            .collect();
    };
    let year = sf2_report_year(&template.school_year, month);
    let last_day = last_day_of_month(year, month);

    // Build a lookup from date string to existing mapping
    let mapping_by_date: HashMap<&str, &Sf2DateMappingRecord> =
        date_mappings.iter().map(|m| (m.date.as_str(), m)).collect();

    let mut preview_dates = Vec::with_capacity(last_day as usize);
    for day in 1..=last_day {
        let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
            continue;
        };

        // Skip weekends (Mon=1 .. Fri=5)
        let weekday = date.weekday().number_from_monday();
        if weekday > 5 {
            continue;
        }

        let date_str = date.format("%Y-%m-%d").to_string();

        if let Some(mapping) = mapping_by_date.get(date_str.as_str()) {
            preview_dates.push(Sf2PreviewDate {
                date: date_str,
                sheet_name: mapping.sheet_name.clone(),
                column_letter: mapping.column_letter.clone(),
                column_index: mapping.column_index,
            });
        } else {
            // Placeholder for unmapped weekday — shown in preview but can't
            // be written to Excel (filtered out by export logic).
            preview_dates.push(Sf2PreviewDate {
                date: date_str,
                sheet_name: String::new(),
                column_letter: String::new(),
                column_index: 0,
            });
        }
    }

    preview_dates
}
