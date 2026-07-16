use crate::domain::error::{AppError, Result};
use crate::sf2::excel_com::workbook::ComObject;

/// Get a worksheet cell by row and column.
pub fn worksheet_cell(sheet: &ComObject, row: i32, column: i32) -> Result<ComObject> {
    let cells = sheet.get_object("Cells")?;
    cells.get_object_with_args("Item", vec![crate::sf2::excel_com::workbook::ComVariant::i4(row), crate::sf2::excel_com::workbook::ComVariant::i4(column)])
}

/// Get text content of a specific cell.
pub fn cell_text(sheet: &ComObject, row: i32, column: i32) -> Result<String> {
    worksheet_cell(sheet, row, column)?.get_string("Text")
}

/// Set a cell value with optional text formatting.
pub fn set_sf2_cell(
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

/// Set a cell value by address (e.g., "B2"), allowing overwriting formula cells.
pub fn set_sf2_mark_force(sheet: &ComObject, cell_address: &str, value: &str) -> Result<()> {
    let cell = sheet.get_object_with_args("Range", vec![crate::sf2::excel_com::workbook::ComVariant::bstr(cell_address)])?;
    let target = merged_target(&cell)?;
    put_cell_value_numeric(&target, value)
}

/// Set an Excel formula on a cell by address (e.g., "F29"), overwriting any existing value.
pub fn set_sf2_formula(sheet: &ComObject, cell_address: &str, formula: &str) -> Result<()> {
    let cell = sheet.get_object_with_args("Range", vec![crate::sf2::excel_com::workbook::ComVariant::bstr(cell_address)])?;
    let target = merged_target(&cell)?;
    target.put_string("Formula", formula)
}

/// Set a cell value by address (e.g., "B2"), refusing to overwrite formula cells.
pub fn set_sf2_mark(sheet: &ComObject, cell_address: &str, value: &str) -> Result<()> {
    let cell = sheet.get_object_with_args("Range", vec![crate::sf2::excel_com::workbook::ComVariant::bstr(cell_address)])?;
    let target = merged_target(&cell)?;
    ensure_not_formula(sheet, &target)?;
    put_cell_value(&target, value)
}

/// Get the merged area target cell, or the cell itself if not merged.
pub fn merged_target(cell: &ComObject) -> Result<ComObject> {
    if !cell.get_bool("MergeCells")? {
        return Ok(cell.clone());
    }

    let merge_area = cell.get_object("MergeArea")?;
    let cells = merge_area.get_object("Cells")?;
    cells.get_object_with_args("Item", vec![crate::sf2::excel_com::workbook::ComVariant::i4(1), crate::sf2::excel_com::workbook::ComVariant::i4(1)])
}

/// Refuse to overwrite a formula cell.
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
            vec![crate::sf2::excel_com::workbook::ComVariant::bool(false), crate::sf2::excel_com::workbook::ComVariant::bool(false)],
        )
        .map(|value| value.to_string_value())
        .unwrap_or_else(|_| "?".to_string());
    Err(AppError::Internal(format!(
        "Refusing to overwrite formula cell {sheet_name}!{address}"
    )))
}

/// Write a string value to a cell, or clear it if empty.
fn put_cell_value(cell: &ComObject, value: &str) -> Result<()> {
    if value.is_empty() {
        cell.put_variant("Value2", crate::sf2::excel_com::workbook::ComVariant::empty())
    } else {
        cell.put_string("Value2", value)
    }
}

/// Write a value to a cell as a number if it parses as an integer, otherwise fall back to string.
/// Used by `set_sf2_mark_force` so that TOTAL Per Day counts (e.g., "13") are stored as numbers
/// that Excel formulas (e.g., Combined TOTAL = F29+F49) can calculate correctly.
fn put_cell_value_numeric(cell: &ComObject, value: &str) -> Result<()> {
    if value.is_empty() {
        cell.put_variant("Value2", crate::sf2::excel_com::workbook::ComVariant::empty())
    } else if let Ok(num) = value.parse::<i32>() {
        cell.put_variant("Value2", crate::sf2::excel_com::workbook::ComVariant::i4(num))
    } else {
        cell.put_string("Value2", value)
    }
}

/// Rename a worksheet to a unique name within the 31-char limit.
pub fn rename_sheet_unique(sheet: &ComObject, base_name: &str) -> Result<()> {
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

