use crate::domain::error::Result;
use crate::sf2::excel_com::com_session::{run_excel_task, with_workbook, ComObject, ComVariant};
use crate::sf2::excel_com::learners::{sf2_sheet_quality, workbook_learners, Sf2SheetQuality};
use crate::sf2::excel_com::workbook::WorkbookSession;
use crate::sf2::excel_com::workbook_utils::*;
use crate::sf2::excel_com::worksheet::cell_text;
use crate::sf2::models::{Sf2WorkbookAnalysis, Sf2WorkbookDate};
use chrono::NaiveDate;
use std::path::Path;

const EXCEL_SHEET_VISIBLE: i32 = -1;

/// Analyze a workbook at the given path, extracting metadata, dates, and learners.
pub fn analyze_workbook(path: &Path) -> Result<Sf2WorkbookAnalysis> {
    let path = path.to_path_buf();
    run_excel_task(move || {
        with_workbook(&path, true, false, |_, workbook| {
            let sheets = workbook.get_object("Worksheets")?;
            let sheet_count = sheets.get_i32("Count")?;
            let mut sheet_infos = Vec::new();
            let mut dates = Vec::new();
            let mut first_monthly_sheet = None;
            let mut best_roster_sheet: Option<(ComObject, Sf2SheetQuality)> = None;
            let mut school_year = String::new();
            let mut school_id = String::new();
            let mut school_name = String::new();
            let mut report_month = String::new();
            let mut grade_level = String::new();
            let mut section = String::new();
            let mut adviser_name = String::new();
            let mut school_head_name = String::new();

            for sheet_index in 1..=sheet_count {
                let sheet =
                    sheets.get_object_with_args("Item", vec![ComVariant::i4(sheet_index)])?;
                let sheet_name = sheet.get_string("Name")?;
                let visible = sheet.get_i32("Visible")?;
                let used_range = sheet.get_object("UsedRange")?;
                let used_range_address = used_range.get_with_args(
                    "Address",
                    vec![ComVariant::bool(false), ComVariant::bool(false)],
                )?;

                sheet_infos.push(crate::sf2::models::Sf2WorkbookSheet {
                    name: sheet_name.clone(),
                    visible,
                    used_range: used_range_address.to_string_value(),
                });

                if visible != EXCEL_SHEET_VISIBLE {
                    continue;
                }

                let month_number = month_number(&sheet_name);
                let year = year_from_sheet_name(&sheet_name);
                if month_number == 0 || year == 0 {
                    continue;
                }

                if first_monthly_sheet.is_none() {
                    school_id = cell_text(&sheet, 3, 6)?.trim().to_string();
                    school_name = cell_text(&sheet, 4, 6)?.trim().to_string();
                    school_year = cell_text(&sheet, 3, 13)?.trim().to_string();
                    report_month = cell_text(&sheet, 3, 27)?.trim().to_string();
                    grade_level = cell_text(&sheet, 4, 27)?.trim().to_string();
                    section = cell_text(&sheet, 4, 39)?.trim().to_string();
                    adviser_name = cell_text(&sheet, 76, 40)?.trim().to_string();
                    if adviser_name.is_empty() {
                        adviser_name = cell_text(&sheet, 82, 26)?.trim().to_string();
                    }
                    school_head_name = cell_text(&sheet, 82, 40)?.trim().to_string();
                    first_monthly_sheet = Some(sheet.clone());
                }

                let quality = sf2_sheet_quality(&sheet)?;
                if best_roster_sheet
                    .as_ref()
                    .is_none_or(|(_, best_quality)| quality > *best_quality)
                {
                    best_roster_sheet = Some((sheet.clone(), quality));
                }

                for column in 6..=38 {
                    let day_text = cell_text(&sheet, 6, column)?.trim().to_string();
                    let Ok(day) = day_text.parse::<u32>() else {
                        continue;
                    };
                    if !(1..=31).contains(&day) {
                        continue;
                    }
                    let Some(date) = NaiveDate::from_ymd_opt(year, month_number, day) else {
                        continue;
                    };
                    dates.push(Sf2WorkbookDate {
                        sheet_name: sheet_name.clone(),
                        date: date.format("%Y-%m-%d").to_string(),
                        column_letter: column_number_to_letter(column),
                        column_index: column as u32,
                    });
                }
            }

            // ── Fallback: no monthly sheets found ─────────────────────────
            if first_monthly_sheet.is_none() {
                for sheet_index in 1..=sheet_count {
                    let sheet =
                        sheets.get_object_with_args("Item", vec![ComVariant::i4(sheet_index)])?;
                    let sheet_name = sheet.get_string("Name")?;
                    let visible = sheet.get_i32("Visible")?;

                    if month_number(&sheet_name) > 0 && year_from_sheet_name(&sheet_name) > 0 {
                        continue;
                    }

                    let title = cell_text(&sheet, 1, 1)?.trim().to_string();
                    if !sheet_is_analysis_candidate(&sheet_name, &title, visible) {
                        continue;
                    }

                    school_id = cell_text(&sheet, 3, 6)?.trim().to_string();
                    school_name = cell_text(&sheet, 4, 6)?.trim().to_string();
                    school_year = cell_text(&sheet, 3, 13)?.trim().to_string();
                    report_month = cell_text(&sheet, 3, 27)?.trim().to_string();
                    grade_level = cell_text(&sheet, 4, 27)?.trim().to_string();
                    section = cell_text(&sheet, 4, 39)?.trim().to_string();
                    adviser_name = cell_text(&sheet, 76, 40)?.trim().to_string();
                    if adviser_name.is_empty() {
                        adviser_name = cell_text(&sheet, 82, 26)?.trim().to_string();
                    }
                    school_head_name = cell_text(&sheet, 82, 40)?.trim().to_string();

                    let fallback_month = month_number(&report_month);
                    let fallback_year = if fallback_month > 0 {
                        report_year(&school_year, fallback_month)
                    } else {
                        report_year("", 1)
                    };
                    let date_year = fallback_year;
                    let date_month = if fallback_month > 0 {
                        fallback_month
                    } else {
                        1
                    };

                    for column in 6..=38 {
                        let day_text = cell_text(&sheet, 6, column)?.trim().to_string();
                        let Ok(day) = day_text.parse::<u32>() else {
                            continue;
                        };
                        if !(1..=31).contains(&day) {
                            continue;
                        }
                        let Some(date) = NaiveDate::from_ymd_opt(date_year, date_month, day) else {
                            continue;
                        };
                        dates.push(Sf2WorkbookDate {
                            sheet_name: sheet_name.clone(),
                            date: date.format("%Y-%m-%d").to_string(),
                            column_letter: column_number_to_letter(column),
                            column_index: column as u32,
                        });
                    }

                    let quality = sf2_sheet_quality(&sheet)?;
                    best_roster_sheet = Some((sheet.clone(), quality));
                    first_monthly_sheet = Some(sheet);
                    break;
                }
            }

            let learner_sheet = best_roster_sheet
                .map(|(sheet, _)| sheet)
                .or(first_monthly_sheet);
            let learners = match learner_sheet {
                Some(sheet) => workbook_learners(&sheet)?,
                None => Vec::new(),
            };

            Ok(Sf2WorkbookAnalysis {
                file_format: workbook.get_i32("FileFormat")?,
                has_vb_project: workbook.get_bool("HasVBProject")?,
                school_id,
                school_name,
                school_year,
                report_month,
                grade_level,
                section,
                adviser_name,
                school_head_name,
                learners,
                dates,
                sheets: sheet_infos,
            })
        })
    })
}

impl WorkbookSession {
    /// Analyze the open workbook, extracting metadata, dates, and learners.
    pub fn analyze(&self) -> Result<Sf2WorkbookAnalysis> {
        let sheets = self.workbook.get_object("Worksheets")?;
        let sheet_count = sheets.get_i32("Count")?;
        let mut sheet_infos = Vec::new();
        let mut dates = Vec::new();
        let mut first_monthly_sheet = None;
        let mut best_roster_sheet: Option<(ComObject, Sf2SheetQuality)> = None;
        let mut school_year = String::new();
        let mut school_id = String::new();
        let mut school_name = String::new();
        let mut report_month = String::new();
        let mut grade_level = String::new();
        let mut section = String::new();
        let mut adviser_name = String::new();
        let mut school_head_name = String::new();

        for sheet_index in 1..=sheet_count {
            let sheet = sheets.get_object_with_args("Item", vec![ComVariant::i4(sheet_index)])?;
            let sheet_name = sheet.get_string("Name")?;
            let visible = sheet.get_i32("Visible")?;
            let used_range = sheet.get_object("UsedRange")?;
            let used_range_address = used_range.get_with_args(
                "Address",
                vec![ComVariant::bool(false), ComVariant::bool(false)],
            )?;

            sheet_infos.push(crate::sf2::models::Sf2WorkbookSheet {
                name: sheet_name.clone(),
                visible,
                used_range: used_range_address.to_string_value(),
            });

            if visible != EXCEL_SHEET_VISIBLE {
                continue;
            }

            let month_number = month_number(&sheet_name);
            let year = year_from_sheet_name(&sheet_name);
            if month_number == 0 || year == 0 {
                continue;
            }

            if first_monthly_sheet.is_none() {
                school_id = cell_text(&sheet, 3, 6)?.trim().to_string();
                school_name = cell_text(&sheet, 4, 6)?.trim().to_string();
                school_year = cell_text(&sheet, 3, 13)?.trim().to_string();
                report_month = cell_text(&sheet, 3, 27)?.trim().to_string();
                grade_level = cell_text(&sheet, 4, 27)?.trim().to_string();
                section = cell_text(&sheet, 4, 39)?.trim().to_string();
                adviser_name = cell_text(&sheet, 76, 40)?.trim().to_string();
                if adviser_name.is_empty() {
                    adviser_name = cell_text(&sheet, 82, 26)?.trim().to_string();
                }
                school_head_name = cell_text(&sheet, 82, 40)?.trim().to_string();
                first_monthly_sheet = Some(sheet.clone());
            }

            let quality = sf2_sheet_quality(&sheet)?;
            if best_roster_sheet
                .as_ref()
                .is_none_or(|(_, best_quality)| quality > *best_quality)
            {
                best_roster_sheet = Some((sheet.clone(), quality));
            }

            for column in 6..=38 {
                let day_text = cell_text(&sheet, 6, column)?.trim().to_string();
                let Ok(day) = day_text.parse::<u32>() else {
                    continue;
                };
                if !(1..=31).contains(&day) {
                    continue;
                }
                let Some(date) = NaiveDate::from_ymd_opt(year, month_number, day) else {
                    continue;
                };
                dates.push(Sf2WorkbookDate {
                    sheet_name: sheet_name.clone(),
                    date: date.format("%Y-%m-%d").to_string(),
                    column_letter: column_number_to_letter(column),
                    column_index: column as u32,
                });
            }
        }

        // Fallback: no monthly sheets found
        if first_monthly_sheet.is_none() {
            for sheet_index in 1..=sheet_count {
                let sheet =
                    sheets.get_object_with_args("Item", vec![ComVariant::i4(sheet_index)])?;
                let sheet_name = sheet.get_string("Name")?;
                let visible = sheet.get_i32("Visible")?;

                if month_number(&sheet_name) > 0 && year_from_sheet_name(&sheet_name) > 0 {
                    continue;
                }

                let title = cell_text(&sheet, 1, 1)?.trim().to_string();
                if !sheet_is_analysis_candidate(&sheet_name, &title, visible) {
                    continue;
                }

                school_id = cell_text(&sheet, 3, 6)?.trim().to_string();
                school_name = cell_text(&sheet, 4, 6)?.trim().to_string();
                school_year = cell_text(&sheet, 3, 13)?.trim().to_string();
                report_month = cell_text(&sheet, 3, 27)?.trim().to_string();
                grade_level = cell_text(&sheet, 4, 27)?.trim().to_string();
                section = cell_text(&sheet, 4, 39)?.trim().to_string();
                adviser_name = cell_text(&sheet, 76, 40)?.trim().to_string();
                if adviser_name.is_empty() {
                    adviser_name = cell_text(&sheet, 82, 26)?.trim().to_string();
                }
                school_head_name = cell_text(&sheet, 82, 40)?.trim().to_string();

                let fallback_month = month_number(&report_month);
                let fallback_year = if fallback_month > 0 {
                    report_year(&school_year, fallback_month)
                } else {
                    report_year("", 1)
                };
                let date_year = fallback_year;
                let date_month = if fallback_month > 0 {
                    fallback_month
                } else {
                    1
                };

                for column in 6..=38 {
                    let day_text = cell_text(&sheet, 6, column)?.trim().to_string();
                    let Ok(day) = day_text.parse::<u32>() else {
                        continue;
                    };
                    if !(1..=31).contains(&day) {
                        continue;
                    }
                    let Some(date) = NaiveDate::from_ymd_opt(date_year, date_month, day) else {
                        continue;
                    };
                    dates.push(Sf2WorkbookDate {
                        sheet_name: sheet_name.clone(),
                        date: date.format("%Y-%m-%d").to_string(),
                        column_letter: column_number_to_letter(column),
                        column_index: column as u32,
                    });
                }

                let quality = sf2_sheet_quality(&sheet)?;
                best_roster_sheet = Some((sheet.clone(), quality));
                first_monthly_sheet = Some(sheet);
                break;
            }
        }

        let learner_sheet = best_roster_sheet
            .map(|(sheet, _)| sheet)
            .or(first_monthly_sheet);
        let learners = match learner_sheet {
            Some(sheet) => workbook_learners(&sheet)?,
            None => Vec::new(),
        };

        Ok(Sf2WorkbookAnalysis {
            file_format: self.workbook.get_i32("FileFormat")?,
            has_vb_project: self.workbook.get_bool("HasVBProject")?,
            school_id,
            school_name,
            school_year,
            report_month,
            grade_level,
            section,
            adviser_name,
            school_head_name,
            learners,
            dates,
            sheets: sheet_infos,
        })
    }
}
