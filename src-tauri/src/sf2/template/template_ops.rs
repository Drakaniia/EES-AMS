use crate::domain::error::Result;
use crate::infrastructure::database::DbPool;
use crate::sf2::models::{Sf2ImportSummary, Sf2TemplateDraft};
use crate::sf2::template_create::create_workbook_from_template_in_dir;
use crate::sf2::workbook_files::sf2_workbook_dir;

/// Create a new SF2 workbook from the bundled template
pub fn create_workbook_from_template<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    pool: DbPool,
    draft: Sf2TemplateDraft,
) -> Result<Sf2ImportSummary> {
    crate::sf2::progress::emit_sf2_progress(&app, "create", 1, 2, "Creating SF2 working workbook");
    let workbook_dir = sf2_workbook_dir(&app)?;
    let summary = create_workbook_from_template_in_dir(&workbook_dir, pool, draft)?;
    crate::sf2::progress::emit_sf2_progress(&app, "create", 2, 2, "SF2 workbook ready");
    Ok(summary)
}

// `set_report_month`, `set_report_month_with_progress` and
// `set_report_month_impl` are gone (spec §7.2, acceptance #8-#10). They spent a
// full Excel COM session per switch - `write_metadata` -> `configure_sf2_calendar`
// -> `analyze` -> clear TOTAL rows -> rewrite COUNTIF/AM/AO formulas -> rewrite
// the AR/AS/AT summary block - and emitted a `month_switch` `sf2-progress` event
// stream a modal existed to cover. A month switch is now
// `crate::sf2::month_preview::month_preview`, one read-only SQL query with no
// Excel, no mutation and no progress events.
