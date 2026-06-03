use crate::domain::error::{AppError, Result};
use crate::sf2::logic::Sf2CellMark;
use crate::sf2::models::{
    Sf2WorkbookAnalysis, Sf2WorkbookDate, Sf2WorkbookLearner, Sf2WorkbookMetadata, Sf2WorkbookSheet,
};
use chrono::{Datelike, NaiveDate};
use std::cell::Cell;
use std::mem::ManuallyDrop;
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
const EXCEL_SHEET_HIDDEN: i32 = 0;
const EXCEL_ALIGN_LEFT: i32 = -4131;
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

                sheet_infos.push(Sf2WorkbookSheet {
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

            let learners = match first_monthly_sheet {
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

fn workbook_learners(sheet: &ComObject) -> Result<Vec<Sf2WorkbookLearner>> {
    let used_range = sheet.get_object("UsedRange")?;
    let rows = used_range.get_object("Rows")?;
    let row_count = rows.get_i32("Count")?;
    let mut gender_block = Some("MALE".to_string());
    let mut learners = Vec::new();

    for row in 1..=row_count {
        let name = cell_text(sheet, row, 3)?.trim().to_string();
        if name.is_empty() {
            continue;
        }

        let upper_name = name.to_uppercase();
        if upper_name.contains("MALE") && upper_name.contains("TOTAL") {
            gender_block = Some("FEMALE".to_string());
            continue;
        }
        if upper_name.contains("FEMALE") && upper_name.contains("TOTAL") {
            gender_block = None;
            continue;
        }

        learners.push(Sf2WorkbookLearner {
            row_index: row as u32,
            name,
            gender_block: gender_block.clone(),
        });
    }

    Ok(learners)
}

fn configure_sf2_calendar(
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
    let target_sheet = monthly_sheets
        .iter()
        .find(|sheet| {
            sheet
                .get_string("Name")
                .is_ok_and(|name| name == target_sheet_name)
        })
        .cloned()
        .unwrap_or_else(|| monthly_sheets[0].clone());

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
        if month_number(&sheet_name) > 0 && year_from_sheet_name(&sheet_name) > 0 {
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

fn sf2_weekday_slots(sheet: &ComObject) -> Result<Vec<Sf2WeekdaySlot>> {
    let mut slots = Vec::new();
    let mut week_index = 0;
    let mut previous_weekday = None;

    for column in 6..=38 {
        let weekday_text = cell_text(sheet, 7, column)?.trim().to_string();
        let Some(weekday_index) = weekday_index(&weekday_text) else {
            continue;
        };

        if previous_weekday.is_some_and(|previous| weekday_index <= previous) {
            week_index += 1;
        }

        slots.push(Sf2WeekdaySlot {
            column,
            week_index,
            weekday_index,
            label: weekday_label(weekday_index).to_string(),
        });
        previous_weekday = Some(weekday_index);
    }

    Ok(slots)
}

fn set_sf2_date_cell(sheet: &ComObject, column: i32, value: &str) -> Result<()> {
    set_sf2_cell(sheet, 6, column, value, true)?;
    let cell = worksheet_cell(sheet, 6, column)?;
    let target = merged_target(&cell)?;

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

fn set_sf2_cell(
    sheet: &ComObject,
    row: i32,
    column: i32,
    value: &str,
    text_format: bool,
) -> Result<()> {
    let cell = worksheet_cell(sheet, row, column)?;
    let target = merged_target(&cell)?;
    ensure_not_formula(sheet, &target)?;

    if text_format {
        let _ = target.put_string("NumberFormat", "@");
    }
    put_cell_value(&target, value)
}

fn set_sf2_mark(sheet: &ComObject, cell_address: &str, value: &str) -> Result<()> {
    let cell = sheet.get_object_with_args("Range", vec![ComVariant::bstr(cell_address)])?;
    let target = merged_target(&cell)?;
    ensure_not_formula(sheet, &target)?;
    put_cell_value(&target, value)
}

fn merged_target(cell: &ComObject) -> Result<ComObject> {
    if !cell.get_bool("MergeCells")? {
        return Ok(cell.clone());
    }

    let merge_area = cell.get_object("MergeArea")?;
    let cells = merge_area.get_object("Cells")?;
    cells.get_object_with_args("Item", vec![ComVariant::i4(1), ComVariant::i4(1)])
}

fn ensure_not_formula(sheet: &ComObject, target: &ComObject) -> Result<()> {
    if !target.get_bool("HasFormula")? {
        return Ok(());
    }

    let sheet_name = sheet
        .get_string("Name")
        .unwrap_or_else(|_| "Sheet".to_string());
    let address = target
        .get_with_args(
            "Address",
            vec![ComVariant::bool(false), ComVariant::bool(false)],
        )
        .map(|value| value.to_string_value())
        .unwrap_or_else(|_| "?".to_string());
    Err(AppError::Internal(format!(
        "Refusing to overwrite formula cell {sheet_name}!{address}"
    )))
}

fn put_cell_value(cell: &ComObject, value: &str) -> Result<()> {
    if value.is_empty() {
        cell.put_variant("Value2", ComVariant::empty())
    } else {
        cell.put_string("Value2", value)
    }
}

fn worksheet_cell(sheet: &ComObject, row: i32, column: i32) -> Result<ComObject> {
    let cells = sheet.get_object("Cells")?;
    cells.get_object_with_args("Item", vec![ComVariant::i4(row), ComVariant::i4(column)])
}

fn cell_text(sheet: &ComObject, row: i32, column: i32) -> Result<String> {
    worksheet_cell(sheet, row, column)?.get_string("Text")
}

fn rename_sheet_unique(sheet: &ComObject, base_name: &str) -> Result<()> {
    let name = truncate_sheet_name(base_name);
    if sheet.put_string("Name", &name).is_ok() {
        return Ok(());
    }

    for suffix in 1..=99 {
        let tail = format!("-{suffix}");
        let base_len = 31usize.saturating_sub(tail.len());
        let mut candidate = name.chars().take(base_len).collect::<String>();
        candidate.push_str(&tail);
        if sheet.put_string("Name", &candidate).is_ok() {
            return Ok(());
        }
    }

    sheet.put_string("Name", &name)
}

fn truncate_sheet_name(name: &str) -> String {
    name.chars().take(31).collect()
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

#[derive(Clone)]
struct ComObject {
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

    fn get_object(&self, name: &str) -> Result<Self> {
        self.get(name)?.to_dispatch()
    }

    fn get_object_with_args(&self, name: &str, args: Vec<ComVariant>) -> Result<Self> {
        self.invoke(name, DISPATCH_PROPERTYGET, args)?.to_dispatch()
    }

    fn method_object(&self, name: &str, args: Vec<ComVariant>) -> Result<Self> {
        self.method(name, args)?.to_dispatch()
    }

    fn get_string(&self, name: &str) -> Result<String> {
        Ok(self.get(name)?.to_string_value())
    }

    fn get_i32(&self, name: &str) -> Result<i32> {
        self.get(name)?.to_i32()
    }

    fn get_bool(&self, name: &str) -> Result<bool> {
        self.get(name)?.to_bool()
    }

    fn put_bool(&self, name: &str, value: bool) -> Result<()> {
        self.put_variant(name, ComVariant::bool(value))
    }

    fn put_i4(&self, name: &str, value: i32) -> Result<()> {
        self.put_variant(name, ComVariant::i4(value))
    }

    fn put_string(&self, name: &str, value: &str) -> Result<()> {
        self.put_variant(name, ComVariant::bstr(value))
    }

    fn put_variant(&self, name: &str, value: ComVariant) -> Result<()> {
        self.invoke(name, DISPATCH_PROPERTYPUT, vec![value])?;
        Ok(())
    }

    fn get(&self, name: &str) -> Result<ComVariant> {
        self.invoke(name, DISPATCH_PROPERTYGET, Vec::new())
    }

    fn get_with_args(&self, name: &str, args: Vec<ComVariant>) -> Result<ComVariant> {
        self.invoke(name, DISPATCH_PROPERTYGET, args)
    }

    fn method(&self, name: &str, args: Vec<ComVariant>) -> Result<ComVariant> {
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

struct ComVariant(VARIANT);

impl ComVariant {
    fn empty() -> Self {
        Self(variant_from_type(VT_EMPTY, VARIANT_0_0_0 { lVal: 0 }))
    }

    fn i4(value: i32) -> Self {
        Self(variant_from_type(VT_I4, VARIANT_0_0_0 { lVal: value }))
    }

    fn bool(value: bool) -> Self {
        let value = if value {
            VARIANT_BOOL(-1)
        } else {
            VARIANT_BOOL(0)
        };
        Self(variant_from_type(VT_BOOL, VARIANT_0_0_0 { boolVal: value }))
    }

    fn bstr(value: &str) -> Self {
        Self(variant_from_type(
            VT_BSTR,
            VARIANT_0_0_0 {
                bstrVal: ManuallyDrop::new(BSTR::from(value)),
            },
        ))
    }

    fn to_dispatch(&self) -> Result<ComObject> {
        if self.variant_type() != VT_DISPATCH {
            return Err(AppError::Internal(format!(
                "Excel automation returned {}, expected dispatch object",
                self.variant_type_name()
            )));
        }

        let dispatch = unsafe {
            let dispatch = &self.0.Anonymous.Anonymous.Anonymous.pdispVal;
            ManuallyDrop::into_inner(dispatch.clone())
        }
        .ok_or_else(|| AppError::Internal("Excel returned a null object".to_string()))?;

        Ok(ComObject { dispatch })
    }

    fn to_string_value(&self) -> String {
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

    fn to_i32(&self) -> Result<i32> {
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

    fn to_bool(&self) -> Result<bool> {
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
            Anonymous: ManuallyDrop::new(VARIANT_0_0 {
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

fn month_number(name: &str) -> u32 {
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

fn month_name(month: u32) -> &'static str {
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

fn report_year(_school_year: &str, _month: u32) -> i32 {
    chrono::Local::now().year()
}

fn year_from_sheet_name(name: &str) -> i32 {
    name.split(|ch: char| !ch.is_ascii_digit())
        .find_map(|part| {
            (part.len() == 4 && part.starts_with("20"))
                .then(|| part.parse::<i32>().ok())
                .flatten()
        })
        .unwrap_or(0)
}

fn weekday_index(label: &str) -> Option<i64> {
    match label.to_uppercase().as_str() {
        "M" => Some(0),
        "T" => Some(1),
        "W" => Some(2),
        "TH" => Some(3),
        "F" => Some(4),
        _ => None,
    }
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

fn column_number_to_letter(mut column: i32) -> String {
    let mut letter = String::new();
    while column > 0 {
        let modulo = (column - 1) % 26;
        letter.insert(0, (b'A' + modulo as u8) as char);
        column = (column - modulo) / 26;
    }
    letter
}

fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    haystack.to_uppercase().contains(&needle.to_uppercase())
}

#[derive(Debug)]
struct Sf2WeekdaySlot {
    column: i32,
    week_index: i32,
    weekday_index: i64,
    label: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sf2::logic::Sf2CellMark;

    const BUNDLED_TEMPLATE_BYTES: &[u8] =
        include_bytes!("../../resources/sf2/TEMPLATE_AUTOMATED_SF2.xls");

    #[test]
    fn accepts_excel_double_for_integer_properties() {
        let value = ComVariant(variant_from_type(VT_R8, VARIANT_0_0_0 { dblVal: 42.0 }));

        assert_eq!(value.to_i32().unwrap(), 42);
    }

    #[test]
    fn report_year_uses_current_calendar_year() {
        assert_eq!(report_year("2025-2026", 6), chrono::Local::now().year());
    }

    #[test]
    #[ignore = "requires Microsoft Excel COM automation"]
    fn sf2_template_com_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let workbook_path = dir.path().join("sf2-template-round-trip.xls");
        std::fs::write(&workbook_path, BUNDLED_TEMPLATE_BYTES).unwrap();

        let metadata = Sf2WorkbookMetadata {
            school_id: "123456".to_string(),
            school_name: "Sample Integrated School".to_string(),
            school_year: "2026-2027".to_string(),
            report_month: "JUNE".to_string(),
            grade_level: "7".to_string(),
            section: "Rose".to_string(),
            adviser_name: "Teacher Adviser".to_string(),
            school_head_name: "School Head".to_string(),
            configure_calendar: true,
            first_school_day: Some(8),
        };

        write_metadata(&workbook_path, &metadata).unwrap();
        let analysis = analyze_workbook(&workbook_path).unwrap();

        assert_eq!(analysis.school_id, "123456");
        assert_eq!(analysis.school_name, "Sample Integrated School");
        assert_eq!(analysis.school_year, "2026-2027");
        assert_eq!(analysis.report_month, "JUNE");
        assert_eq!(analysis.grade_level, "7");
        assert_eq!(analysis.section, "Rose");
        assert_eq!(analysis.adviser_name, "Teacher Adviser");
        assert_eq!(analysis.school_head_name, "School Head");
        assert!(analysis
            .sheets
            .iter()
            .any(|sheet| sheet.name == "JUNE 2026" && sheet.visible == EXCEL_SHEET_VISIBLE));
        assert_eq!(
            analysis.dates.iter().map(|date| date.date.as_str()).min(),
            Some("2026-06-08")
        );

        write_marks(
            &workbook_path,
            &[
                Sf2CellMark {
                    sheet_name: "JUNE 2026".to_string(),
                    cell_address: "C8".to_string(),
                    value: "Learner, One".to_string(),
                },
                Sf2CellMark {
                    sheet_name: "JUNE 2026".to_string(),
                    cell_address: "F8".to_string(),
                    value: "X".to_string(),
                },
            ],
        )
        .unwrap();

        let updated = analyze_workbook(&workbook_path).unwrap();
        assert!(updated
            .learners
            .iter()
            .any(|learner| learner.row_index == 8 && learner.name == "Learner, One"));
    }
}
