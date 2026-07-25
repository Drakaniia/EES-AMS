use crate::domain::error::Result;
use crate::sf2::excel_com::com_session::{run_excel_task, with_workbook, ComVariant};
use crate::sf2::excel_com::workbook::WorkbookSession;
use crate::sf2::excel_com::workbook_utils::*;
use std::collections::HashSet;
use std::path::Path;

const EXCEL_SHEET_VISIBLE: i32 = -1;

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
                let sheet =
                    sheets.get_object_with_args("Item", vec![ComVariant::i4(sheet_index)])?;
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
                        entire_row
                            .method("Insert", vec![ComVariant::i4(-4121), ComVariant::i4(0)])?;
                    }
                }

                // Insert extra female rows before FEMALE TOTAL row.
                if extra_female_rows > 0 {
                    let female_base = female_total_row.unwrap_or(49) + extra_male_rows;
                    let range = sheet.get_object_with_args(
                        "Range",
                        vec![ComVariant::bstr(&format!("{female_base}:{female_base}"))],
                    )?;
                    let entire_row = range.get_object("EntireRow")?;
                    for _ in 0..extra_female_rows {
                        entire_row
                            .method("Insert", vec![ComVariant::i4(-4121), ComVariant::i4(0)])?;
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
                let sheet =
                    sheets.get_object_with_args("Item", vec![ComVariant::i4(sheet_index)])?;
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

impl WorkbookSession {
    /// Expand the roster area by inserting extra rows before MALE/FEMALE TOTAL.
    pub fn expand_roster_rows(
        &self,
        extra_male_rows: u32,
        extra_female_rows: u32,
        male_total_row: Option<u32>,
        female_total_row: Option<u32>,
    ) -> Result<()> {
        if extra_male_rows == 0 && extra_female_rows == 0 {
            return Ok(());
        }

        let male_base = male_total_row.unwrap_or(29);
        let sheets = self.workbook.get_object("Worksheets")?;
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

            if extra_male_rows > 0 {
                let insert_start = male_base as i32;
                let range = sheet.get_object_with_args(
                    "Range",
                    vec![ComVariant::bstr(&format!("{insert_start}:{insert_start}"))],
                )?;
                let entire_row = range.get_object("EntireRow")?;
                for _ in 0..extra_male_rows {
                    entire_row.method("Insert", vec![ComVariant::i4(-4121), ComVariant::i4(0)])?;
                }
            }

            if extra_female_rows > 0 {
                let female_base = female_total_row.unwrap_or(49) + extra_male_rows;
                let range = sheet.get_object_with_args(
                    "Range",
                    vec![ComVariant::bstr(&format!("{female_base}:{female_base}"))],
                )?;
                let entire_row = range.get_object("EntireRow")?;
                for _ in 0..extra_female_rows {
                    entire_row.method("Insert", vec![ComVariant::i4(-4121), ComVariant::i4(0)])?;
                }
            }
        }
        Ok(())
    }

    /// Hide empty learner rows on all monthly sheets.
    pub fn hide_empty_learner_rows(
        &self,
        male_total_row: u32,
        female_total_row: u32,
        occupied_rows: &HashSet<u32>,
    ) -> Result<()> {
        let sheets = self.workbook.get_object("Worksheets")?;
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
        Ok(())
    }
}

/// Execute multiple Excel operations within a single Excel process.
///
/// Opens the workbook at `path`, calls `action` with a `&WorkbookSession` that
/// exposes `analyze()`, `write_marks()`, `write_metadata()`, `write_formulas()`,
/// `expand_roster_rows()`, and `hide_empty_learner_rows()` — all operating on
/// the same open workbook without starting/stopping Excel between calls.
///
/// When `save_on_close` is true the workbook is saved before closing.
///
/// ⚠️ Cleanup is always performed even when `action` fails, to prevent orphan
/// Excel processes from accumulating. The action error takes precedence over
/// any close/quit errors in the returned `Result`.
pub fn batch_operations<T, F>(path: &Path, save_on_close: bool, action: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&WorkbookSession) -> Result<T> + Send + 'static,
{
    let path = path.to_path_buf();
    run_excel_task(move || {
        let session = WorkbookSession::open(&path, !save_on_close)?;
        let action_result = action(&session);
        let save_result = if save_on_close {
            session.save()
        } else {
            Ok(())
        };
        let close_result = session.close(save_on_close);
        // Always clean up (save + close) even on action failure, then
        // propagate results preferring action error (same as with_workbook).
        match (action_result, save_result, close_result) {
            (Ok(value), Ok(_), Ok(_)) => Ok(value),
            (Err(error), _, _) => Err(error),
            (_, Err(error), _) => Err(error),
            (_, _, Err(error)) => Err(error),
        }
    })
}
