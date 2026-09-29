//! Opening a workbook for the user without writing to it (spec §9.1; E4, E5, E6).
//!
//! The `Unmeasured` branch of the guard ends here. The user still gets to see
//! their marks - the whole point of refusing to clear is that the marks exist -
//! but the app never opens the file for writing, so the refusal is visible as a
//! read-only window rather than as a failed click.
//!
//! Handing the path to the shell is deliberately *not* what this does: that
//! opens the file read-write, which is the capability the guard just denied.

use crate::domain::error::{AppError, Result};
use std::path::Path;

/// Show the workbook to the user, read-only.
///
/// The workbook is opened with `ReadOnly:=True` through Excel's own `Open`, so
/// Excel shows the marks and refuses to save over them. Nothing is written and
/// the file's mtime is left exactly as it was - which is how the behaviour is
/// verified: attempt a write, assert the mtime has not moved.
pub fn open_workbook_read_only(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(AppError::InvalidInput(
            "The SF2 workbook no longer exists on disk. Restore it from a backup.".to_string(),
        ));
    }
    open_read_only_impl(path)
}

#[cfg(target_os = "windows")]
fn open_read_only_impl(path: &Path) -> Result<()> {
    use crate::sf2::excel::excel_com::com_session::{run_excel_task, ExcelSession};

    let path = path.to_path_buf();
    run_excel_task(move || {
        // `ExcelSession`'s `Drop` quits Excel, which would close the window we
        // are about to hand to the user, so the session is deliberately
        // leaked instead of dropped. The COM apartment still tears down with
        // the thread; only the Excel process outlives this call, which is the
        // point - from here the workbook belongs to the user.
        let excel = ExcelSession::new()?;
        excel.app.put_bool("Visible", true)?;
        let _workbook = excel.open_workbook(&path, true)?;
        std::mem::forget(excel);
        Ok(())
    })
}

#[cfg(not(target_os = "windows"))]
fn open_read_only_impl(path: &Path) -> Result<()> {
    // No Excel automation off Windows, so the OS default app is the only way to
    // show the file. It cannot be told to open read-only here; the guard still
    // holds, because the app itself never writes.
    crate::sf2::workbook_files::open_path_in_default_app(path)
}
