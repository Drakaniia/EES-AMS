use crate::domain::error::{AppError, Result};
use crate::sf2::excel::excel_com::com_session::ComObject;
use crate::sf2::excel::excel_com::learners::best_sf2_monthly_sheet;
use crate::sf2::excel::excel_com::workbook_utils::{month_name, month_number, report_year};
use crate::sf2::excel::excel_com::worksheet::{
    cell_text, rename_sheet_unique, set_sf2_cell, worksheet_cell,
};
use crate::sf2::models::Sf2WorkbookMetadata;
use crate::sf2::month::workbook_builder::{
    day_numbers_for_slots, days_without_a_slot, MonthDaySlot,
};
use chrono::{Datelike, NaiveDate};

const EXCEL_SHEET_VISIBLE: i32 = -1;
const EXCEL_ALIGN_LEFT: i32 = -4131;

/// Write the month's day numbers into the one worksheet a month file holds.
///
/// A month file is created with a single `"{MONTH} {year}"` tab (spec D4), so
/// there is nothing to hide, rename or clear: the target sheet is the sheet.
/// The eleven-hidden-tab loop that used to live here - clearing row 6, renaming
/// to `__SF2_HIDDEN_{n}`, and setting `Visible = 0` on every other month tab -
/// was the entire cost of a month switch, and it is gone because the eleven
/// other tabs are gone. A month switch is now a SQL read
/// (`crate::sf2::month_preview`); this function only runs when a month file is
/// written.
///
/// What remains is the date-header writer: find this month's sheet, make sure
/// it is visible and correctly named, write the day numbers across the merged
/// weekday pairs, and activate it.
pub fn configure_sf2_calendar(
    monthly_sheets: &[ComObject],
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

    // The same pure layout the month-file builder uses, so a date-header write
    // here and a `sf2_month_date_mappings` row there cannot disagree about which
    // day sits in which column. Two copies of this arithmetic is how a day's
    // absence ends up in a column the AMOUNT formulas do not count.
    let month_slots: Vec<MonthDaySlot> = slots
        .iter()
        .map(|slot| MonthDaySlot {
            column: slot.column as u32,
            week_index: slot.week_index as u32,
            weekday_index: slot.weekday_index as u32,
        })
        .collect();

    for (column, day) in day_numbers_for_slots(year, month, first_school_day, &month_slots) {
        let value = day.map_or_else(String::new, |day| day.to_string());
        set_sf2_date_cell(sheet, column as i32, &value)?;
    }

    // The DepEd grid is 25 labelled day cells - five weeks of Monday..Friday -
    // and `AM` (ABSENT) is the very next column, so a sixth week has nowhere to
    // go. A month needs at most 23, so this is empty; it is logged rather than
    // assumed because a silently missing day column is a day the user cannot
    // record an absence on.
    let dropped = days_without_a_slot(year, month, first_school_day, &month_slots);
    if !dropped.is_empty() {
        log::warn!(
            "SF2 month {month} {year} has {} school day(s) the DepEd form has no column for: \
             {:?}. These days cannot hold an X and cannot be recorded.",
            dropped.len(),
            dropped
        );
    }

    Ok(())
}

fn sf2_weekday_slots(sheet: &ComObject) -> Result<Vec<Sf2WeekdaySlot>> {
    // Read weekday labels from row 7 of the sheet. The template has merged
    // column pairs (e.g., F7:G7 = Mon), so merged sub-cells have no label
    // and must be skipped. Non-weekday labels ("ABSENT", "PRESENT") are
    // also skipped.
    let mut slots = Vec::new();
    for column in 6..=38 {
        let label = cell_text(sheet, 7, column)?;
        if label.trim().is_empty() {
            continue;
        }
        let Some(weekday_index) = parse_weekday_label(&label) else {
            continue;
        };
        let week_index = (slots.len() / 5) as i32;
        slots.push(Sf2WeekdaySlot {
            column,
            week_index,
            weekday_index,
        });
    }
    Ok(slots)
}

/// Parse a weekday label from row 7 into a 0-based index (0=Mon..4=Fri).
/// Handles common DepEd SF2 formats: "M", "MON", "MONDAY", "T", "TUE", "TH", etc.
fn parse_weekday_label(label: &str) -> Option<i64> {
    let upper = label.trim().to_uppercase();
    let first = upper.chars().next()?;
    match first {
        'M' => Some(0),
        'T' => {
            // "TH" (Thursday) vs "T" (Tuesday)
            if upper.starts_with("TH") {
                Some(3)
            } else {
                Some(1)
            }
        }
        'W' => Some(2),
        'F' => Some(4),
        _ => None,
    }
}

fn set_sf2_date_cell(sheet: &ComObject, column: i32, value: &str) -> Result<()> {
    set_sf2_cell(sheet, 6, column, value, true)?;
    let cell = worksheet_cell(sheet, 6, column)?;
    let target = crate::sf2::excel::excel_com::worksheet::merged_target(&cell)?;

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
}
