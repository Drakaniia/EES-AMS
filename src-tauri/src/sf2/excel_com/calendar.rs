use crate::domain::error::{AppError, Result};
use crate::sf2::excel_com::learners::best_sf2_monthly_sheet;
use crate::sf2::excel_com::com_session::ComObject;
use crate::sf2::excel_com::workbook_utils::{month_name, month_number, report_year};
use crate::sf2::excel_com::worksheet::{
    rename_sheet_unique, set_sf2_cell, worksheet_cell,
};
use crate::sf2::models::Sf2WorkbookMetadata;
use chrono::{Datelike, NaiveDate};

const EXCEL_SHEET_VISIBLE: i32 = -1;
const EXCEL_SHEET_HIDDEN: i32 = 0;
const EXCEL_ALIGN_LEFT: i32 = -4131;

pub fn configure_sf2_calendar(
    monthly_sheets: &[ComObject],
    sf2_sheets: &[ComObject],
    metadata: &Sf2WorkbookMetadata,
) -> Result<()> {
    let report_month = month_number(&metadata.report_month);
    if report_month == 0 {
        return Err(AppError::InvalidInput(
            "Report Month must be a valid month name".to_string(),
        ));
    }

    let report_year = report_year(&metadata.school_year, report_month);
    let target_sheet_name = format!("{} {}", month_name(report_month), report_year);
    let target_sheet = match monthly_sheets.iter().find(|sheet| {
        sheet
            .get_string("Name")
            .is_ok_and(|name| name == target_sheet_name)
    }) {
        Some(sheet) => sheet.clone(),
        None => {
            best_sf2_monthly_sheet(monthly_sheets)?.unwrap_or_else(|| monthly_sheets[0].clone())
        }
    };

    target_sheet.put_i4("Visible", EXCEL_SHEET_VISIBLE)?;
    rename_sheet_unique(&target_sheet, &target_sheet_name)?;
    set_sf2_month_dates(
        &target_sheet,
        report_year,
        report_month,
        metadata.first_school_day.unwrap_or(1),
    )?;
    let _ = target_sheet.method("Activate", Vec::new());

    let target_index = target_sheet.get_i32("Index")?;
    let mut hidden_index = 1;
    for sheet in sf2_sheets {
        if sheet.get_i32("Index")? == target_index {
            continue;
        }

        clear_sf2_month_dates(sheet)?;
        let sheet_name = sheet.get_string("Name")?;
        if month_number(&sheet_name) > 0 && super::workbook_utils::year_from_sheet_name(&sheet_name) > 0 {
            rename_sheet_unique(sheet, &format!("__SF2_HIDDEN_{hidden_index}"))?;
        }
        sheet.put_i4("Visible", EXCEL_SHEET_HIDDEN)?;
        hidden_index += 1;
    }

    Ok(())
}

fn set_sf2_month_dates(
    sheet: &ComObject,
    year: i32,
    month: u32,
    first_school_day: u32,
) -> Result<()> {
    let slots = sf2_weekday_slots(sheet)?;
    if slots.is_empty() {
        return Ok(());
    }

    let last_day = days_in_month(year, month);
    if first_school_day < 1 || first_school_day > last_day {
        return Err(AppError::InvalidInput(format!(
            "First attendance day must be between 1 and {last_day} for this report month"
        )));
    }

    let first_school_date =
        NaiveDate::from_ymd_opt(year, month, first_school_day).ok_or_else(|| {
            AppError::InvalidInput("First attendance day is not a valid date".to_string())
        })?;
    if date_weekday_index(first_school_date).is_none() {
        return Err(AppError::InvalidInput(
            "First attendance day must be a Monday-Friday school day".to_string(),
        ));
    }

    let monday_anchor =
        first_school_date - chrono::Duration::days(date_weekday_index(first_school_date).unwrap());

    for slot in slots {
        let mut value = String::new();
        for day in first_school_day..=last_day {
            let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
                continue;
            };
            let Some(weekday_index) = date_weekday_index(date) else {
                continue;
            };
            let week_index = (date - monday_anchor).num_days() / 7;
            if week_index == i64::from(slot.week_index) && weekday_index == slot.weekday_index {
                value = day.to_string();
                break;
            }
        }

        set_sf2_date_cell(sheet, slot.column, &value)?;
        set_sf2_cell(sheet, 7, slot.column, &slot.label, true)?;
    }

    Ok(())
}

fn clear_sf2_month_dates(sheet: &ComObject) -> Result<()> {
    for slot in sf2_weekday_slots(sheet)? {
        set_sf2_cell(sheet, 6, slot.column, "", true)?;
    }
    Ok(())
}

fn sf2_weekday_slots(_sheet: &ComObject) -> Result<Vec<Sf2WeekdaySlot>> {
    // Compute weekday slots directly from column indices (6-38) using the
    // standard DepEd SF2 layout: 7 weeks × 5 weekdays (Mon-Fri) = 35 columns.
    //
    // Previously this function read weekday labels from row 7 of the workbook
    // and matched them against specific values ("M", "T", "W", "TH", "F").
    // That approach failed when the imported workbook used different label
    // formats (e.g., "MON" instead of "M"), causing ALL slots to be skipped
    // → no dates written → empty date mappings for the month.
    //
    // Hardcoding from the column index is safe because the DepEd SF2 standard
    // mandates that columns F-AL (6-38) are weekday columns in M-F repetition.
    // After writing dates, the correct labels are written back to row 7
    // (inside set_sf2_month_dates), so subsequent label-based reads work too.
    let mut slots = Vec::with_capacity(33);
    for column in 6..=38 {
        let relative = (column - 6) as usize;
        let week_index = (relative / 5) as i32;
        let weekday_index = (relative % 5) as i64;
        slots.push(Sf2WeekdaySlot {
            column,
            week_index,
            weekday_index,
            label: weekday_label(weekday_index).to_string(),
        });
    }
    Ok(slots)
}

fn set_sf2_date_cell(sheet: &ComObject, column: i32, value: &str) -> Result<()> {
    set_sf2_cell(sheet, 6, column, value, true)?;
    let cell = worksheet_cell(sheet, 6, column)?;
    let target = crate::sf2::excel_com::worksheet::merged_target(&cell)?;

    if cell.get_bool("MergeCells")? {
        if let Ok(merge_area) = cell.get_object("MergeArea") {
            let _ = merge_area.put_i4("HorizontalAlignment", EXCEL_ALIGN_LEFT);
            let _ = merge_area.put_i4("IndentLevel", 0);
        }
    }
    let _ = target.put_i4("HorizontalAlignment", EXCEL_ALIGN_LEFT);
    let _ = target.put_i4("IndentLevel", 0);
    Ok(())
}

fn date_weekday_index(date: NaiveDate) -> Option<i64> {
    match date.weekday() {
        chrono::Weekday::Mon => Some(0),
        chrono::Weekday::Tue => Some(1),
        chrono::Weekday::Wed => Some(2),
        chrono::Weekday::Thu => Some(3),
        chrono::Weekday::Fri => Some(4),
        chrono::Weekday::Sat | chrono::Weekday::Sun => None,
    }
}

fn weekday_label(index: i64) -> &'static str {
    match index {
        0 => "M",
        1 => "T",
        2 => "W",
        3 => "TH",
        4 => "F",
        _ => "",
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_next_month = NaiveDate::from_ymd_opt(next_year, next_month, 1).unwrap();
    (first_next_month - chrono::Duration::days(1)).day()
}

#[derive(Debug)]
struct Sf2WeekdaySlot {
    column: i32,
    week_index: i32,
    weekday_index: i64,
    label: String,
}
