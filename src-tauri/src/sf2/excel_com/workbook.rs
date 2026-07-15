use crate::domain::error::{AppError, Result};
use crate::sf2::excel_com::worksheet::{
    cell_text, set_sf2_cell, set_sf2_formula, set_sf2_mark, set_sf2_mark_force,
};
use crate::sf2::excel_com::calendar::{
    configure_sf2_calendar,
};
use crate::sf2::excel_com::learners::workbook_learners;
use crate::sf2::excel_com::learners::Sf2SheetQuality;
use crate::sf2::logic::Sf2CellMark;
use crate::sf2::models::{Sf2WorkbookAnalysis, Sf2WorkbookDate, Sf2WorkbookMetadata};
use chrono::{Datelike, NaiveDate};
use std::cell::Cell;
use std::collections::HashSet;
use std::path::Path;
use windows::core::{BSTR, GUID, PCWSTR};
use windows::Win32::Foundation::VARIANT_BOOL;
use windows::Win32::System::Com::{
    CLSIDFromProgID, CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch,
    CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, DISPATCH_FLAGS, DISPATCH_METHOD,
    DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPPARAMS,
};
use windows::Win32::System::Ole::DISPID_PROPERTYPUT;
use windows::Win32::System::Variant::{
    VariantClear, VARENUM, VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_BOOL, VT_BSTR,
    VT_DISPATCH, VT_EMPTY, VT_I2, VT_I4, VT_I8, VT_INT, VT_NULL, VT_R4, VT_R8, VT_UI2, VT_UI4,
    VT_UI8, VT_UINT,
};

const EXCEL_SHEET_VISIBLE: i32 = -1;
const LOCALE_USER_DEFAULT: u32 = 0x0400;

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
            // When a workbook has no sheets with standard month+year names
            // (e.g. single-sheet workbooks named "school_form_2_ver2014.2.1.1"),
            // fall back to the first visible sheet that contains "School Form 2"
            // in cell A1 and extract metadata / dates / learners from it.
            if first_monthly_sheet.is_none() {
                for sheet_index in 1..=sheet_count {
                    let sheet = sheets.get_object_with_args(
                        "Item",
                        vec![ComVariant::i4(sheet_index)],
                    )?;
                    let sheet_name = sheet.get_string("Name")?;
                    let visible = sheet.get_i32("Visible")?;

                    // Skip sheets already handled as monthly (above)
                    if month_number(&sheet_name) > 0 && year_from_sheet_name(&sheet_name) > 0 {
                        continue;
                    }

                    // Use the shared candidate check: visible, non-monthly, SF2 title
                    let title = cell_text(&sheet, 1, 1)?.trim().to_string();
                    if !sheet_is_analysis_candidate(&sheet_name, &title, visible) {
                        continue;
                    }

                    // Extract metadata from the fallback sheet
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

                    // Derive the year and month for constructing dates from
                    // the report month metadata found in the workbook itself.
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
                        let Some(date) = NaiveDate::from_ymd_opt(
                            date_year,
                            date_month,
                            day,
                        ) else {
                            continue;
                        };
                        dates.push(Sf2WorkbookDate {
                            sheet_name: sheet_name.clone(),
                            date: date.format("%Y-%m-%d").to_string(),
                            column_letter: column_number_to_letter(column),
                            column_index: column as u32,
                        });
                    }

                    // Also use this sheet for learner detection
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

/// Expand the roster area in an SF2 bundled workbook by inserting extra rows
/// before the MALE TOTAL and FEMALE TOTAL rows as needed.
///
/// `male_total_row` — the current row of the MALE TOTAL cell. Defaults to 29 for a fresh template.
/// `female_total_row` — the current row of the FEMALE TOTAL cell. Defaults to 49 for a fresh template.
///
/// For incremental expansions (sync/update of an already-expanded workbook),
/// pass the ACTUAL current positions of the TOTAL rows (e.g., from existing student mappings).
pub fn expand_roster_rows(
    workbook_path: &Path,
    extra_male_rows: u32,
    extra_female_rows: u32,
    male_total_row: Option<u32>,
    female_total_row: Option<u32>,
) -> Result<()> {
    if extra_male_rows == 0 && extra_female_rows == 0 {
        return Ok(());
    }

    let male_base = male_total_row.unwrap_or(29);
    let workbook_path = workbook_path.to_path_buf();
    run_excel_task(move || {
        with_workbook(&workbook_path, false, true, |_excel, workbook| {
            let sheets = workbook.get_object("Worksheets")?;
            let sheet_count = sheets.get_i32("Count")?;

            for sheet_index in 1..=sheet_count {
                let sheet = sheets.get_object_with_args("Item", vec![ComVariant::i4(sheet_index)])?;
                let visible = sheet.get_i32("Visible")?;
                if visible != EXCEL_SHEET_VISIBLE {
                    continue;
                }

                // Only modify SF2 monthly sheets
                let sheet_name = sheet.get_string("Name")?;
                if month_number(&sheet_name) == 0 || year_from_sheet_name(&sheet_name) == 0 {
                    continue;
                }

                // Insert extra male rows before the MALE TOTAL row.
                if extra_male_rows > 0 {
                    let insert_start = male_base as i32;
                    let range = sheet.get_object_with_args(
                        "Range",
                        vec![ComVariant::bstr(&format!("{insert_start}:{insert_start}"))],
                    )?;
                    let entire_row = range.get_object("EntireRow")?;
                    for _ in 0..extra_male_rows {
                        // xlShiftDown = -4121, xlFormatFromLeftOrAbove = 0
                        entire_row.method(
                            "Insert",
                            vec![ComVariant::i4(-4121), ComVariant::i4(0)],
                        )?;
                    }
                }

                // Insert extra female rows before FEMALE TOTAL row.
                // After male expansion, the FEMALE TOTAL has shifted down by extra_male_rows.
                if extra_female_rows > 0 {
                    let female_base = female_total_row.unwrap_or(49) + extra_male_rows;
                    let range = sheet.get_object_with_args(
                        "Range",
                        vec![ComVariant::bstr(&format!("{female_base}:{female_base}"))],
                    )?;
                    let entire_row = range.get_object("EntireRow")?;
                    for _ in 0..extra_female_rows {
                        entire_row.method(
                            "Insert",
                            vec![ComVariant::i4(-4121), ComVariant::i4(0)],
                        )?;
                    }
                }
            }

            workbook.method("Save", Vec::new())?;
            Ok(())
        })
    })
}

/// Hide empty learner rows on all monthly sheets in an SF2 workbook.
///
/// Rows within the standard learner ranges that are NOT in `occupied_rows` are
/// hidden (`.Hidden = true` in Excel), so only rows with actual students are visible.
///
/// `male_total_row` — the Excel row of the MALE TOTAL Per Day cell.
/// `female_total_row` — the Excel row of the FEMALE TOTAL Per Day cell.
pub fn hide_empty_learner_rows(
    workbook_path: &Path,
    male_total_row: u32,
    female_total_row: u32,
    occupied_rows: &HashSet<u32>,
) -> Result<()> {
    let workbook_path = workbook_path.to_path_buf();
    let occupied_rows = occupied_rows.clone();
    run_excel_task(move || {
        with_workbook(&workbook_path, false, true, |_excel, workbook| {
            let sheets = workbook.get_object("Worksheets")?;
            let sheet_count = sheets.get_i32("Count")?;

            for sheet_index in 1..=sheet_count {
                let sheet = sheets.get_object_with_args("Item", vec![ComVariant::i4(sheet_index)])?;
                let visible = sheet.get_i32("Visible")?;
                if visible != EXCEL_SHEET_VISIBLE {
                    continue;
                }

                let sheet_name = sheet.get_string("Name")?;
                if month_number(&sheet_name) == 0 || year_from_sheet_name(&sheet_name) == 0 {
                    continue;
                }

                // Male learner rows: row 8 to row (male_total_row - 1)
                if male_total_row > 8 {
                    for row in 8..male_total_row {
                        let range = sheet.get_object_with_args(
                            "Range",
                            vec![ComVariant::bstr(&format!("{row}:{row}"))],
                        )?;
                        let entire_row = range.get_object("EntireRow")?;
                        let hidden = !occupied_rows.contains(&row);
                        entire_row.put_bool("Hidden", hidden)?;
                    }
                }

                // Female learner rows: (male_total_row + 1) to row (female_total_row - 1)
                let female_start = male_total_row + 1;
                if female_total_row > female_start {
                    for row in female_start..female_total_row {
                        let range = sheet.get_object_with_args(
                            "Range",
                            vec![ComVariant::bstr(&format!("{row}:{row}"))],
                        )?;
                        let entire_row = range.get_object("EntireRow")?;
                        let hidden = !occupied_rows.contains(&row);
                        entire_row.put_bool("Hidden", hidden)?;
                    }
                }
            }

            workbook.method("Save", Vec::new())?;
            Ok(())
        })
    })
}

pub fn write_formulas(workbook_path: &Path, formula_marks: &[Sf2CellMark]) -> Result<()> {
    let workbook_path = workbook_path.to_path_buf();
    let formula_marks = formula_marks.to_vec();
    run_excel_task(move || {
        with_workbook(&workbook_path, false, true, |excel, workbook| {
            let sheets = workbook.get_object("Worksheets")?;
            for mark in &formula_marks {
                let sheet = sheets
                    .get_object_with_args("Item", vec![ComVariant::bstr(&mark.sheet_name)])?;
                set_sf2_formula(&sheet, &mark.cell_address, &mark.value)?;
            }

            excel.calculate_full_rebuild()?;
            workbook.method("Save", Vec::new())?;
            Ok(())
        })
    })
}

pub fn write_marks(workbook_path: &Path, marks: &[Sf2CellMark]) -> Result<()> {
    let workbook_path = workbook_path.to_path_buf();
    let marks = marks.to_vec();
    run_excel_task(move || {
        with_workbook(&workbook_path, false, true, |excel, workbook| {
            let sheets = workbook.get_object("Worksheets")?;
            for mark in &marks {
                let sheet = sheets
                    .get_object_with_args("Item", vec![ComVariant::bstr(&mark.sheet_name)])?;
                set_sf2_mark(&sheet, &mark.cell_address, &mark.value)?;
            }

            excel.calculate_full_rebuild()?;
            workbook.method("Save", Vec::new())?;
            Ok(())
        })
    })
}

/// Write marks to a workbook, overwriting formula cells.
/// Used specifically for TOTAL Per Day cells that may contain SUM formulas
/// that need to be replaced with computed numeric values.
pub fn write_marks_force(workbook_path: &Path, marks: &[Sf2CellMark]) -> Result<()> {
    let workbook_path = workbook_path.to_path_buf();
    let marks = marks.to_vec();
    run_excel_task(move || {
        with_workbook(&workbook_path, false, true, |excel, workbook| {
            let sheets = workbook.get_object("Worksheets")?;
            for mark in &marks {
                let sheet = sheets
                    .get_object_with_args("Item", vec![ComVariant::bstr(&mark.sheet_name)])?;
                set_sf2_mark_force(&sheet, &mark.cell_address, &mark.value)?;
            }

            excel.calculate_full_rebuild()?;
            workbook.method("Save", Vec::new())?;
            Ok(())
        })
    })
}

pub fn write_metadata(workbook_path: &Path, metadata: &Sf2WorkbookMetadata) -> Result<()> {
    let workbook_path = workbook_path.to_path_buf();
    let metadata = metadata.clone();
    run_excel_task(move || {
        with_workbook(&workbook_path, false, true, |excel, workbook| {
            let sheets = workbook.get_object("Worksheets")?;
            let sheet_count = sheets.get_i32("Count")?;
            let mut sf2_sheets = Vec::new();
            let mut monthly_sheets = Vec::new();
            let mut sheets_updated = 0usize;

            for sheet_index in 1..=sheet_count {
                let sheet =
                    sheets.get_object_with_args("Item", vec![ComVariant::i4(sheet_index)])?;
                let title = cell_text(&sheet, 1, 1)?.trim().to_string();
                if !contains_ignore_ascii_case(&title, "School Form 2") {
                    continue;
                }

                let sheet_name = sheet.get_string("Name")?;
                if month_number(&sheet_name) > 0 && year_from_sheet_name(&sheet_name) > 0 {
                    monthly_sheets.push(sheet.clone());
                }
                sf2_sheets.push(sheet.clone());

                set_sf2_cell(&sheet, 3, 6, &metadata.school_id, true)?;
                set_sf2_cell(&sheet, 3, 13, &metadata.school_year, true)?;
                set_sf2_cell(&sheet, 3, 27, &metadata.report_month, true)?;
                set_sf2_cell(&sheet, 4, 6, &metadata.school_name, true)?;
                set_sf2_cell(&sheet, 4, 27, &metadata.grade_level, true)?;
                set_sf2_cell(&sheet, 4, 39, &metadata.section, true)?;
                set_sf2_cell(&sheet, 76, 40, &metadata.adviser_name, true)?;
                set_sf2_cell(&sheet, 82, 26, &metadata.adviser_name, true)?;
                set_sf2_cell(&sheet, 82, 40, &metadata.school_head_name, true)?;
                sheets_updated += 1;
            }

            if metadata.configure_calendar && !monthly_sheets.is_empty() {
                configure_sf2_calendar(&monthly_sheets, &sf2_sheets, &metadata)?;
            }

            excel.calculate_full_rebuild()?;
            workbook.method("Save", Vec::new())?;
            log::debug!("updated SF2 metadata on {sheets_updated} sheets");
            Ok(())
        })
    })
}

// ── COM Infrastructure ────────────────────────────────────────────────────────

fn run_excel_task<T, F>(task: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    std::thread::spawn(move || {
        let _apartment = ComApartment::init()?;
        task()
    })
    .join()
    .map_err(|_| AppError::Internal("Excel automation thread panicked".to_string()))?
}

struct ComApartment;

impl ComApartment {
    fn init() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .ok()
                .map_err(|error| {
                    AppError::Internal(format!("failed to initialize Excel automation: {error}"))
                })?;
        }
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

fn with_workbook<T, F>(path: &Path, read_only: bool, save_on_close: bool, action: F) -> Result<T>
where
    F: FnOnce(&ExcelSession, &ComObject) -> Result<T>,
{
    let mut excel = ExcelSession::new()?;
    let workbook = excel.open_workbook(path, read_only)?;
    let action_result = action(&excel, &workbook);
    let close_result = workbook.method("Close", vec![ComVariant::bool(save_on_close)]);
    let quit_result = excel.quit();

    match (action_result, close_result, quit_result) {
        (Ok(value), Ok(_), Ok(_)) => Ok(value),
        (Err(error), _, _) => Err(error),
        (_, Err(error), _) => Err(error),
        (_, _, Err(error)) => Err(error),
    }
}

struct ExcelSession {
    app: ComObject,
    quit_called: Cell<bool>,
}

impl ExcelSession {
    fn new() -> Result<Self> {
        let app = ComObject::excel_application()?;
        app.put_bool("Visible", false)?;
        app.put_bool("DisplayAlerts", false)?;
        app.put_bool("EnableEvents", false)?;
        let _ = app.put_i4("AutomationSecurity", 3);
        Ok(Self {
            app,
            quit_called: Cell::new(false),
        })
    }

    fn open_workbook(&self, path: &Path, read_only: bool) -> Result<ComObject> {
        let workbooks = self.app.get_object("Workbooks")?;
        workbooks.method_object(
            "Open",
            vec![
                ComVariant::bstr(&path.to_string_lossy()),
                ComVariant::i4(0),
                ComVariant::bool(read_only),
            ],
        )
    }

    fn calculate_full_rebuild(&self) -> Result<()> {
        self.app.method("CalculateFullRebuild", Vec::new())?;
        Ok(())
    }

    fn quit(&mut self) -> Result<()> {
        if self.quit_called.replace(true) {
            return Ok(());
        }
        self.app.method("Quit", Vec::new())?;
        Ok(())
    }
}

impl Drop for ExcelSession {
    fn drop(&mut self) {
        let _ = self.quit();
    }
}

// ── ComObject ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ComObject {
    dispatch: IDispatch,
}

impl ComObject {
    fn excel_application() -> Result<Self> {
        let prog_id = wide_null("Excel.Application");
        let clsid = unsafe { CLSIDFromProgID(PCWSTR(prog_id.as_ptr())) }.map_err(|error| {
            AppError::Internal(format!("Microsoft Excel is not available: {error}"))
        })?;
        let dispatch = unsafe { CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER) }
            .map_err(|error| AppError::Internal(format!("failed to start Excel: {error}")))?;
        Ok(Self { dispatch })
    }

    pub fn get_object(&self, name: &str) -> Result<Self> {
        self.get(name)?.to_dispatch()
    }

    pub fn get_object_with_args(&self, name: &str, args: Vec<ComVariant>) -> Result<Self> {
        self.invoke(name, DISPATCH_PROPERTYGET, args)?.to_dispatch()
    }

    pub fn method_object(&self, name: &str, args: Vec<ComVariant>) -> Result<Self> {
        self.method(name, args)?.to_dispatch()
    }

    pub fn get_string(&self, name: &str) -> Result<String> {
        Ok(self.get(name)?.to_string_value())
    }

    pub fn get_i32(&self, name: &str) -> Result<i32> {
        self.get(name)?.to_i32()
    }

    pub fn get_bool(&self, name: &str) -> Result<bool> {
        self.get(name)?.to_bool()
    }

    pub fn put_bool(&self, name: &str, value: bool) -> Result<()> {
        self.put_variant(name, ComVariant::bool(value))
    }

    pub fn put_i4(&self, name: &str, value: i32) -> Result<()> {
        self.put_variant(name, ComVariant::i4(value))
    }

    pub fn put_string(&self, name: &str, value: &str) -> Result<()> {
        self.put_variant(name, ComVariant::bstr(value))
    }

    pub fn put_variant(&self, name: &str, value: ComVariant) -> Result<()> {
        self.invoke(name, DISPATCH_PROPERTYPUT, vec![value])?;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<ComVariant> {
        self.invoke(name, DISPATCH_PROPERTYGET, Vec::new())
    }

    pub fn get_with_args(&self, name: &str, args: Vec<ComVariant>) -> Result<ComVariant> {
        self.invoke(name, DISPATCH_PROPERTYGET, args)
    }

    pub fn method(&self, name: &str, args: Vec<ComVariant>) -> Result<ComVariant> {
        self.invoke(name, DISPATCH_METHOD, args)
    }

    fn invoke(
        &self,
        name: &str,
        flags: DISPATCH_FLAGS,
        args: Vec<ComVariant>,
    ) -> Result<ComVariant> {
        let dispid = self.dispid(name)?;
        let mut raw_args = args
            .into_iter()
            .rev()
            .map(ComVariant::into_raw)
            .collect::<Vec<_>>();
        let mut property_put_dispid = DISPID_PROPERTYPUT;
        let is_property_put = flags == DISPATCH_PROPERTYPUT;
        let params = DISPPARAMS {
            rgvarg: if raw_args.is_empty() {
                std::ptr::null_mut()
            } else {
                raw_args.as_mut_ptr()
            },
            rgdispidNamedArgs: if is_property_put {
                &mut property_put_dispid
            } else {
                std::ptr::null_mut()
            },
            cArgs: raw_args.len() as u32,
            cNamedArgs: u32::from(is_property_put),
        };
        let mut result = VARIANT::default();
        let invoke_result = unsafe {
            self.dispatch.Invoke(
                dispid,
                &GUID::zeroed(),
                LOCALE_USER_DEFAULT,
                flags,
                &params,
                Some(&mut result),
                None,
                None,
            )
        };
        clear_variants(&mut raw_args);

        invoke_result.map_err(|error| {
            AppError::Internal(format!("Excel automation failed on {name}: {error}"))
        })?;
        Ok(ComVariant(result))
    }

    fn dispid(&self, name: &str) -> Result<i32> {
        let wide_name = wide_null(name);
        let names = [PCWSTR(wide_name.as_ptr())];
        let mut dispid = 0;
        unsafe {
            self.dispatch.GetIDsOfNames(
                &GUID::zeroed(),
                names.as_ptr(),
                names.len() as u32,
                LOCALE_USER_DEFAULT,
                &mut dispid,
            )
        }
        .map_err(|error| {
            AppError::Internal(format!(
                "Excel automation could not resolve {name}: {error}"
            ))
        })?;
        Ok(dispid)
    }
}

// ── ComVariant ─────────────────────────────────────────────────────────────────

pub struct ComVariant(VARIANT);

impl ComVariant {
    pub fn empty() -> Self {
        Self(variant_from_type(VT_EMPTY, VARIANT_0_0_0 { lVal: 0 }))
    }

    pub fn i4(value: i32) -> Self {
        Self(variant_from_type(VT_I4, VARIANT_0_0_0 { lVal: value }))
    }

    pub fn bool(value: bool) -> Self {
        let value = if value {
            VARIANT_BOOL(-1)
        } else {
            VARIANT_BOOL(0)
        };
        Self(variant_from_type(VT_BOOL, VARIANT_0_0_0 { boolVal: value }))
    }

    pub fn bstr(value: &str) -> Self {
        Self(variant_from_type(
            VT_BSTR,
            VARIANT_0_0_0 {
                bstrVal: std::mem::ManuallyDrop::new(BSTR::from(value)),
            },
        ))
    }

    pub fn to_dispatch(&self) -> Result<ComObject> {
        if self.variant_type() != VT_DISPATCH {
            return Err(AppError::Internal(format!(
                "Excel automation returned {}, expected dispatch object",
                self.variant_type_name()
            )));
        }

        let dispatch = unsafe {
            let dispatch = &self.0.Anonymous.Anonymous.Anonymous.pdispVal;
            std::mem::ManuallyDrop::into_inner(dispatch.clone())
        }
        .ok_or_else(|| AppError::Internal("Excel returned a null object".to_string()))?;

        Ok(ComObject { dispatch })
    }

    pub fn to_string_value(&self) -> String {
        match self.variant_type() {
            VT_BSTR => unsafe { self.0.Anonymous.Anonymous.Anonymous.bstrVal.to_string() },
            VT_I4 => unsafe { self.0.Anonymous.Anonymous.Anonymous.lVal.to_string() },
            VT_I2 => unsafe { self.0.Anonymous.Anonymous.Anonymous.iVal.to_string() },
            VT_I8 => unsafe { self.0.Anonymous.Anonymous.Anonymous.llVal.to_string() },
            VT_INT => unsafe { self.0.Anonymous.Anonymous.Anonymous.intVal.to_string() },
            VT_UI2 => unsafe { self.0.Anonymous.Anonymous.Anonymous.uiVal.to_string() },
            VT_UI4 => unsafe { self.0.Anonymous.Anonymous.Anonymous.ulVal.to_string() },
            VT_UI8 => unsafe { self.0.Anonymous.Anonymous.Anonymous.ullVal.to_string() },
            VT_UINT => unsafe { self.0.Anonymous.Anonymous.Anonymous.uintVal.to_string() },
            VT_R4 => unsafe { self.0.Anonymous.Anonymous.Anonymous.fltVal.to_string() },
            VT_R8 => unsafe { self.0.Anonymous.Anonymous.Anonymous.dblVal.to_string() },
            VT_BOOL => {
                if self.to_bool().unwrap_or(false) {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            VT_EMPTY | VT_NULL => String::new(),
            _ => String::new(),
        }
    }

    pub fn to_i32(&self) -> Result<i32> {
        match self.variant_type() {
            VT_I4 => Ok(unsafe { self.0.Anonymous.Anonymous.Anonymous.lVal }),
            VT_I2 => Ok(i32::from(unsafe {
                self.0.Anonymous.Anonymous.Anonymous.iVal
            })),
            VT_I8 => i32::try_from(unsafe { self.0.Anonymous.Anonymous.Anonymous.llVal })
                .map_err(|_| self.integer_range_error()),
            VT_INT => Ok(unsafe { self.0.Anonymous.Anonymous.Anonymous.intVal }),
            VT_UI2 => Ok(i32::from(unsafe {
                self.0.Anonymous.Anonymous.Anonymous.uiVal
            })),
            VT_UI4 | VT_UINT => {
                i32::try_from(unsafe { self.0.Anonymous.Anonymous.Anonymous.ulVal })
                    .map_err(|_| self.integer_range_error())
            }
            VT_UI8 => i32::try_from(unsafe { self.0.Anonymous.Anonymous.Anonymous.ullVal })
                .map_err(|_| self.integer_range_error()),
            VT_R4 => {
                float_to_i32(unsafe { f64::from(self.0.Anonymous.Anonymous.Anonymous.fltVal) })
                    .map_err(|_| self.integer_range_error())
            }
            VT_R8 => float_to_i32(unsafe { self.0.Anonymous.Anonymous.Anonymous.dblVal })
                .map_err(|_| self.integer_range_error()),
            VT_BOOL => Ok(i32::from(self.to_bool()?)),
            _ => Err(AppError::Internal(format!(
                "Excel automation returned {}, expected integer",
                self.variant_type_name()
            ))),
        }
    }

    pub fn to_bool(&self) -> Result<bool> {
        match self.variant_type() {
            VT_BOOL => Ok(unsafe { self.0.Anonymous.Anonymous.Anonymous.boolVal }.0 != 0),
            VT_I4 => Ok(unsafe { self.0.Anonymous.Anonymous.Anonymous.lVal } != 0),
            VT_EMPTY | VT_NULL => Ok(false),
            _ => Err(AppError::Internal(format!(
                "Excel automation returned {}, expected boolean",
                self.variant_type_name()
            ))),
        }
    }

    fn variant_type(&self) -> VARENUM {
        unsafe { self.0.Anonymous.Anonymous.vt }
    }

    fn variant_type_name(&self) -> String {
        format!("VARIANT({})", self.variant_type().0)
    }

    fn integer_range_error(&self) -> AppError {
        AppError::Internal(format!(
            "Excel automation returned {}, but it is not a valid integer",
            self.variant_type_name()
        ))
    }

    fn into_raw(mut self) -> VARIANT {
        std::mem::take(&mut self.0)
    }
}

impl Drop for ComVariant {
    fn drop(&mut self) {
        unsafe {
            let _ = VariantClear(&mut self.0);
        }
    }
}

fn variant_from_type(vt: VARENUM, value: VARIANT_0_0_0) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: std::mem::ManuallyDrop::new(VARIANT_0_0 {
                vt,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: value,
            }),
        },
    }
}

fn clear_variants(variants: &mut [VARIANT]) {
    for variant in variants {
        unsafe {
            let _ = VariantClear(variant);
        }
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn float_to_i32(value: f64) -> std::result::Result<i32, ()> {
    if value.is_finite()
        && value.fract() == 0.0
        && value >= f64::from(i32::MIN)
        && value <= f64::from(i32::MAX)
    {
        Ok(value as i32)
    } else {
        Err(())
    }
}

// ── Utility functions ─────────────────────────────────────────────────────────

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

pub fn report_year(_school_year: &str, _month: u32) -> i32 {
    chrono::Local::now().year_ce().1 as i32
}

pub fn year_from_sheet_name(name: &str) -> i32 {
    name.split(|ch: char| !ch.is_ascii_digit())
        .find_map(|part| {
            (part.len() == 4 && part.starts_with("20"))
                .then(|| part.parse::<i32>().ok())
                .flatten()
        })
        .unwrap_or(0)
}

pub fn column_number_to_letter(mut column: i32) -> String {
    let mut letter = String::new();
    while column > 0 {
        let modulo = (column - 1) % 26;
        letter.insert(0, (b'A' + modulo as u8) as char);
        column = (column - modulo) / 26;
    }
    letter
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // ── month_number ──────────────────────────────────────────────────────

    #[test]
    fn month_number_standard_month_name() {
        assert_eq!(month_number("JUNE 2026"), 6);
    }

    #[test]
    fn month_number_lowercase() {
        assert_eq!(month_number("june 2026"), 6);
    }

    #[test]
    fn month_number_partial_match_march() {
        // "MAR" is inside "MARCH"
        assert_eq!(month_number("MARCH 2026"), 3);
    }

    #[test]
    fn month_number_problematic_sheet_name() {
        // The sheet name from the failing SF2 file
        assert_eq!(month_number("school_form_2_ver2014.2.1.1"), 0);
    }

    #[test]
    fn month_number_empty_string() {
        assert_eq!(month_number(""), 0);
    }

    #[test]
    fn month_number_no_match() {
        // "SUMMARY" contains "MAR" as substring (positions 3-5), so it
        // returns 3. Use a truly non-matching name for the "no match" test.
        assert_eq!(month_number("OVERVIEW"), 0);
    }

    #[test]
    fn month_number_all_months() {
        assert_eq!(month_number("JANUARY 2026"), 1);
        assert_eq!(month_number("FEBRUARY 2026"), 2);
        assert_eq!(month_number("MARCH 2026"), 3);
        assert_eq!(month_number("APRIL 2026"), 4);
        assert_eq!(month_number("MAY 2026"), 5);
        assert_eq!(month_number("JUNE 2026"), 6);
        assert_eq!(month_number("JULY 2026"), 7);
        assert_eq!(month_number("AUGUST 2026"), 8);
        assert_eq!(month_number("SEPTEMBER 2026"), 9);
        assert_eq!(month_number("OCTOBER 2026"), 10);
        assert_eq!(month_number("NOVEMBER 2026"), 11);
        assert_eq!(month_number("DECEMBER 2026"), 12);
    }

    // ── year_from_sheet_name ──────────────────────────────────────────────

    #[test]
    fn year_from_sheet_name_standard() {
        assert_eq!(year_from_sheet_name("JUNE 2026"), 2026);
    }

    #[test]
    fn year_from_sheet_name_problematic_name() {
        // The sheet name contains "2014" from the version string
        assert_eq!(year_from_sheet_name("school_form_2_ver2014.2.1.1"), 2014);
    }

    #[test]
    fn year_from_sheet_name_no_year() {
        assert_eq!(year_from_sheet_name("SUMMARY"), 0);
    }

    #[test]
    fn year_from_sheet_name_not_starting_with_20() {
        assert_eq!(year_from_sheet_name("SHEET_1999"), 0);
    }

    #[test]
    fn year_from_sheet_name_four_digit_starting_with_20() {
        assert_eq!(year_from_sheet_name("SHEET_2026"), 2026);
    }

    #[test]
    fn year_from_sheet_name_empty_string() {
        assert_eq!(year_from_sheet_name(""), 0);
    }

    // ── contains_ignore_ascii_case ────────────────────────────────────────

    #[test]
    fn contains_ignore_ascii_case_exact_match() {
        assert!(contains_ignore_ascii_case("School Form 2", "School Form 2"));
    }

    #[test]
    fn contains_ignore_ascii_case_case_insensitive() {
        assert!(contains_ignore_ascii_case("school form 2 (sf2)", "School Form 2"));
    }

    #[test]
    fn contains_ignore_ascii_case_substring() {
        assert!(contains_ignore_ascii_case(
            "School Form 2 (SF2) Daily Attendance Report of Learners",
            "School Form 2"
        ));
    }

    #[test]
    fn contains_ignore_ascii_case_no_match() {
        assert!(!contains_ignore_ascii_case("Something else entirely", "School Form 2"));
    }

    #[test]
    fn contains_ignore_ascii_case_empty_haystack() {
        assert!(!contains_ignore_ascii_case("", "School Form 2"));
    }

    #[test]
    fn contains_ignore_ascii_case_empty_needle() {
        assert!(contains_ignore_ascii_case("anything", ""));
    }

    // ── column_number_to_letter ───────────────────────────────────────────

    #[test]
    fn column_number_to_letter_a() {
        assert_eq!(column_number_to_letter(1), "A");
    }

    #[test]
    fn column_number_to_letter_z() {
        assert_eq!(column_number_to_letter(26), "Z");
    }

    #[test]
    fn column_number_to_letter_aa() {
        assert_eq!(column_number_to_letter(27), "AA");
    }

    #[test]
    fn column_number_to_letter_f() {
        assert_eq!(column_number_to_letter(6), "F");
    }

    #[test]
    fn column_number_to_letter_al() {
        assert_eq!(column_number_to_letter(38), "AL");
    }

    // ── sheet_is_analysis_candidate ───────────────────────────────────────

    #[test]
    fn sheet_is_analysis_candidate_monthly_sheet() {
        // Standard monthly sheet should always be a candidate
        assert!(sheet_is_analysis_candidate(
            "JUNE 2026",
            "School Form 2 (SF2) Daily Attendance Report of Learners",
            -1,
        ));
    }

    #[test]
    fn sheet_is_analysis_candidate_non_monthly_sf2_sheet() {
        // THIS IS THE BUG REPRODUCTION:
        // A visible sheet containing a genuine SF2 form but with a non-standard
        // sheet name (like the failing file) should be usable as a fallback.
        // Currently the code skips it entirely — this test expects it to work.
        assert!(sheet_is_analysis_candidate(
            "school_form_2_ver2014.2.1.1",
            "School Form 2 (SF2) Daily Attendance Report of Learners",
            -1,
        ));
    }

    #[test]
    fn sheet_is_analysis_candidate_non_monthly_non_sf2_sheet() {
        // A non-monthly sheet that is NOT an SF2 form should not be a candidate
        assert!(!sheet_is_analysis_candidate(
            "Summary",
            "Some random data",
            -1,
        ));
    }

    #[test]
    fn sheet_is_analysis_candidate_hidden_sheet() {
        // Hidden sheets should never be candidates
        assert!(!sheet_is_analysis_candidate(
            "JUNE 2026",
            "School Form 2 (SF2) Daily Attendance Report of Learners",
            0,
        ));
    }

    #[test]
    fn sheet_is_analysis_candidate_non_monthly_sf2_with_no_sf2_title() {
        // A non-monthly sheet that has a name matching the problematic pattern
        // but doesn't have "School Form 2" in the title should NOT be a candidate
        assert!(!sheet_is_analysis_candidate(
            "school_form_2_ver2014.2.1.1",
            "Some other data",
            -1,
        ));
    }
}

pub fn sf2_sheet_quality(sheet: &ComObject) -> Result<Sf2SheetQuality> {
    let learners = workbook_learners(sheet)?;
    let learner_count = learners
        .iter()
        .filter(|learner| crate::sf2::logic::is_learner_name(&learner.name))
        .count();
    let male_count = learners
        .iter()
        .filter(|learner| {
            learner.gender_block.as_deref() == Some("MALE") && crate::sf2::logic::is_learner_name(&learner.name)
        })
        .count();
    let female_count = learners
        .iter()
        .filter(|learner| {
            learner.gender_block.as_deref() == Some("FEMALE") && crate::sf2::logic::is_learner_name(&learner.name)
        })
        .count();
    let total_day_cells = sf2_total_day_cell_count(sheet)?;

    Ok(Sf2SheetQuality {
        total_day_cells,
        learner_count,
        male_count,
        female_count,
    })
}

fn sf2_total_day_cell_count(sheet: &ComObject) -> Result<usize> {
    let mut count = 0usize;
    for row in [29, 49] {
        for column in 6..=38 {
            if !crate::sf2::excel_com::worksheet::cell_text(sheet, row, column)?.trim().is_empty() {
                count += 1;
            }
        }
    }
    Ok(count)
}
