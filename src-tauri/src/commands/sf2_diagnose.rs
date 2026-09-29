use super::*;
use crate::sf2::diagnose::model::Sf2MarkDiagnostic;

/// Measure, per month, what the SF2 workbook holds against what the database
/// holds — without writing to either (spec §0 A5, §9.1).
///
/// Read-only by construction, not by convention:
///
/// * the database is opened on its own `SQLITE_OPEN_READ_ONLY` connection, so no
///   statement this command could run is able to write;
/// * the workbook is opened `ReadOnly:=True` and closed without saving.
///
/// It deliberately does **not** call `record_command_audit`. That helper writes
/// an `audit_events` row, and a diagnostic whose whole purpose is to be able to
/// say "nothing was written" cannot be the thing that writes something.
///
/// A user who has the workbook open in Excel gets `ExcelUnavailable` for the
/// months that file would have covered, not an error: that is a state, and the
/// point of the command is to report the state.
#[tauri::command]
pub fn diagnose_sf2_marks(app: tauri::AppHandle) -> std::result::Result<Sf2MarkDiagnostic, String> {
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to get the app data directory: {error}"))?
        .join("attendance.db");

    log::info!(
        "running the read-only SF2 mark diagnostic against {}",
        db_path.display()
    );
    let diagnostic =
        crate::sf2::diagnose::diagnose_sf2_marks(&db_path).map_err(|error| error.to_string())?;

    // The verdict is logged at `warn` unless it cleared the comparison, because
    // "the workbook holds marks the database lacks" and "the database could not
    // be shown to hold them" both mean a write path must not run.
    let summary = format!(
        "{} - {} (database holds {} absent event(s); {} month(s) measured, {} not)",
        diagnostic.verdict,
        diagnostic.verdict_reason,
        diagnostic.total_absent_events,
        diagnostic
            .months
            .iter()
            .filter(|month| month.source_status.is_comparable())
            .count(),
        diagnostic.incomparable_months.len(),
    );
    if diagnostic.verdict.permits_write() {
        log::info!("SF2 mark diagnostic: {summary}");
    } else {
        log::warn!("SF2 mark diagnostic: {summary}");
    }
    for line in &diagnostic.incomparable_months {
        log::warn!("SF2 mark diagnostic, unmeasured month: {line}");
    }
    for sheet in &diagnostic.unplaced_sheets {
        log::warn!(
            "SF2 mark diagnostic, unplaced worksheet '{}' in {}: {}",
            sheet.sheet_name,
            sheet.workbook_path,
            sheet.reason
        );
    }

    Ok(diagnostic)
}
