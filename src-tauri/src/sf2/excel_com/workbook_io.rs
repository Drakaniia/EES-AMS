use crate::domain::error::Result;
use crate::sf2::excel_com::calendar::configure_sf2_calendar;
use crate::sf2::excel_com::com_session::{run_excel_task, with_workbook, ComVariant};
use crate::sf2::excel_com::workbook::WorkbookSession;
use crate::sf2::excel_com::worksheet::{cell_text, set_sf2_cell, set_sf2_formula, set_sf2_mark, set_sf2_mark_force};
use crate::sf2::excel_com::workbook_utils::*;
use crate::sf2::logic::Sf2CellMark;
use crate::sf2::models::Sf2WorkbookMetadata;
use std::path::Path;

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

impl WorkbookSession {
    /// Write attendance marks to the open workbook.
    pub fn write_marks(&self, marks: &[Sf2CellMark]) -> Result<()> {
        let sheets = self.workbook.get_object("Worksheets")?;
        for mark in marks {
            let sheet =
                sheets.get_object_with_args("Item", vec![ComVariant::bstr(&mark.sheet_name)])?;
            set_sf2_mark(&sheet, &mark.cell_address, &mark.value)?;
        }
        self.calculate()?;
        Ok(())
    }

    /// Write attendance marks to the open workbook, overwriting formula cells.
    pub fn write_marks_force(&self, marks: &[Sf2CellMark]) -> Result<()> {
        let sheets = self.workbook.get_object("Worksheets")?;
        for mark in marks {
            let sheet =
                sheets.get_object_with_args("Item", vec![ComVariant::bstr(&mark.sheet_name)])?;
            set_sf2_mark_force(&sheet, &mark.cell_address, &mark.value)?;
        }
        self.calculate()?;
        Ok(())
    }

    /// Write Excel formulas to the open workbook.
    pub fn write_formulas(&self, formula_marks: &[Sf2CellMark]) -> Result<()> {
        let sheets = self.workbook.get_object("Worksheets")?;
        for mark in formula_marks {
            let sheet =
                sheets.get_object_with_args("Item", vec![ComVariant::bstr(&mark.sheet_name)])?;
            set_sf2_formula(&sheet, &mark.cell_address, &mark.value)?;
        }
        self.calculate()?;
        Ok(())
    }

    /// Write metadata (school info, headers) to all SF2 sheets in the open workbook.
    pub fn write_metadata(&self, metadata: &Sf2WorkbookMetadata) -> Result<()> {
        let sheets = self.workbook.get_object("Worksheets")?;
        let sheet_count = sheets.get_i32("Count")?;
        let mut sf2_sheets = Vec::new();
        let mut monthly_sheets = Vec::new();
        let mut sheets_updated = 0usize;

        for sheet_index in 1..=sheet_count {
            let sheet = sheets.get_object_with_args("Item", vec![ComVariant::i4(sheet_index)])?;
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
            configure_sf2_calendar(&monthly_sheets, &sf2_sheets, metadata)?;
        }

        self.calculate()?;
        log::debug!("updated SF2 metadata on {sheets_updated} sheets");
        Ok(())
    }
}
