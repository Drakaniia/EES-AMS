use crate::domain::error::Result;
use crate::sf2::excel::excel_com::com_session::{
    run_excel_task, with_workbook, ComObject, ComVariant,
};
use std::collections::HashMap;
use std::path::Path;

/// Read the display text of many A1-addressed cells in one Excel session.
///
/// `cells` is a list of `(sheet_name, cell_address)` pairs, e.g.
/// `("SEPTEMBER 2026", "H9")`. The workbook is opened **read-only** and closed
/// without saving, so a scan can never modify the file it is reading.
///
/// Sheets are resolved once each and the `Range` dispatch is reused per sheet,
/// which keeps the COM round-trip count at two per cell instead of three.
pub fn read_cell_texts(
    path: &Path,
    cells: &[(String, String)],
) -> Result<HashMap<(String, String), String>> {
    let path = path.to_path_buf();
    let cells = cells.to_vec();
    run_excel_task(move || {
        with_workbook(&path, true, false, |_excel, workbook| {
            let sheets = workbook.get_object("Worksheets")?;
            // Insert-then-get because the cached value is a reference into the
            // map; entries are never removed, so the reference stays valid.
            let mut sheet_cache: HashMap<String, ComObject> = HashMap::new();
            let mut values = HashMap::with_capacity(cells.len());

            for (sheet_name, address) in &cells {
                if !sheet_cache.contains_key(sheet_name) {
                    let sheet =
                        sheets.get_object_with_args("Item", vec![ComVariant::bstr(sheet_name)])?;
                    sheet_cache.insert(sheet_name.clone(), sheet);
                }
                let sheet = sheet_cache
                    .get(sheet_name)
                    .expect("sheet was inserted into the cache above");
                let cell = sheet
                    .get_object_with_args("Range", vec![ComVariant::bstr(address.as_str())])?;
                values.insert(
                    (sheet_name.clone(), address.clone()),
                    cell.get_string("Text")?,
                );
            }

            Ok(values)
        })
    })
}
