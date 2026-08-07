use crate::domain::error::{AppError, Result};
use crate::infrastructure::database::DbPool;
use crate::sf2::models::{Sf2ImportSummary, Sf2TemplateDraft};
use crate::sf2::repository::Sf2Repository;
use crate::sf2::template_create::create_workbook_from_template_in_dir;
use crate::sf2::workbook_files::sf2_workbook_dir;

/// Create a new SF2 workbook from the bundled template
pub fn create_workbook_from_template<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    pool: DbPool,
    draft: Sf2TemplateDraft,
) -> Result<Sf2ImportSummary> {
    super::progress::emit_sf2_progress(&app, "create", 1, 2, "Creating SF2 working workbook");
    let workbook_dir = sf2_workbook_dir(&app)?;
    let summary = create_workbook_from_template_in_dir(&workbook_dir, pool, draft)?;
    super::progress::emit_sf2_progress(&app, "create", 2, 2, "SF2 workbook ready");
    Ok(summary)
}

/// Switch the active report month for an existing workbook.  This updates the
/// report month in the database, reconfigures the Excel workbook for the new
/// month (sheet visibility, date headers), and re-creates the date mappings in
/// the database.
///
/// Date mappings are cached per-month: once a month's mappings have been
/// computed via Excel COM, they persist in the DB across month switches.
/// Subsequent switches to that month skip Excel entirely (fast path).
///
/// Attendance marks are NOT written during month switch — that is deferred to
/// `sync_and_open_sf2_workbook` or `export_workbook`. This makes month
/// switching on the report page fast (no Excel I/O) and prevents potential
/// data loss from clearing marks on other months' sheets.
///
/// The Excel operations are targeted at calendar/date-mappings only — much
/// lighter than a full `update_workbook_settings` call (which also handles
/// roster metadata, student name sync, and row expansion).
pub fn set_report_month(pool: DbPool, class_id: &str, report_month: &str) -> Result<()> {
    set_report_month_impl(pool, class_id, report_month, &|_, _, _| {})
}

/// Same as [`set_report_month`] but emits fine-grained progress events so the
/// frontend can show a determinate progress bar during month switch.
pub fn set_report_month_with_progress<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    pool: DbPool,
    class_id: &str,
    report_month: &str,
) -> Result<()> {
    use super::progress::emit_sf2_progress;
    let app = app.clone();
    let class_id_owned = class_id.to_string();
    let report_month_owned = report_month.to_string();
    set_report_month_impl(
        pool,
        &class_id_owned,
        &report_month_owned,
        &move |current, total, message| {
            emit_sf2_progress(&app, "month_switch", current, total, message);
        },
    )
}

fn set_report_month_impl(
    pool: DbPool,
    class_id: &str,
    report_month: &str,
    emit: &dyn Fn(u32, u32, &str),
) -> Result<()> {
    emit(1, 8, "Updating report month in database…");
    if report_month.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Report month is required".to_string(),
        ));
    }

    let sf2_repo = Sf2Repository::new(pool.clone());

    // 1. Persist the new report month in the DB first so downstream reads see
    //    the correct value.
    let template = sf2_repo
        .latest_template_for_class(class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("No SF2 workbook imported for this class".to_string())
        })?;
    sf2_repo.set_report_month(&template.id, report_month)?;
    emit(2, 8, "Month saved — preparing the workbook…");

    // 2. Reload the template so we have the current report_month.
    let updated_template = sf2_repo
        .latest_template_for_class(class_id)?
        .ok_or_else(|| {
            AppError::InvalidInput("SF2 workbook was removed while switching month".to_string())
        })?;

    // 3. Let the existing refresh function handle the heavy lifting:
    //    - Writes metadata to the Excel workbook (new report month → sheet visible)
    //    - Re-analyzes the workbook to create fresh date mappings
    //    - Persists the new mappings in the DB
    //    - Returns the fully refreshed template with current date mappings
    //
    //    Steps 3-6 span the Excel COM operations (the bulk of the time).
    //    The bar jumps quickly from 3→4 (emitted before the call) and
    //    6→7→8 (emitted after), with steps 4-5 implied during the COM work.
    emit(3, 8, "Opening Excel to reconfigure the calendar…");
    let refreshed = super::excel_service::refresh_template_calendar_from_saved_month(
        pool.clone(),
        &updated_template,
        true,
    )?;
    emit(6, 8, "Calendar reconfigured — verifying date mappings…");

    // 4. Verify date mappings exist for the new month.
    //    We intentionally DO NOT write attendance marks here — the Excel
    //    workbook marks are only needed when the user opens or exports the
    //    workbook, both of which call write_template_marks_for_days internally.
    //    Skipping marks here makes month switching fast (no Excel I/O) and
    //    prevents potential data loss from clearing marks on other months.
    let date_mappings = sf2_repo.date_mappings_for_template(&refreshed.id)?;
    let report_mappings =
        crate::sf2::sf2_metadata::sf2_date_mappings_for_report_month(&refreshed, &date_mappings);
    if report_mappings.is_empty() {
        log::warn!("No date mappings found for report month {report_month} (class {class_id})");
    }

    emit(7, 8, "Saving date mappings…");
    // Mappings already saved by refresh_template_calendar_from_saved_month

    emit(8, 8, "Month switch complete!");
    Ok(())
}
