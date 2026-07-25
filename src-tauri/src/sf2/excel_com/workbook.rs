use crate::domain::error::Result;
use crate::sf2::excel_com::com_session::{ComVariant, ExcelSession, ComObject};
use std::path::Path;

/// An open Excel workbook session with its owning Excel application object.
///
/// When this session is dropped, the workbook is closed WITHOUT saving and
/// Excel is quit. Use `batch_operations` to coordinate save/close correctly.
pub struct WorkbookSession {
    pub(super) excel: ExcelSession,
    pub(super) workbook: ComObject,
}

impl WorkbookSession {
    /// Open a workbook and return a session handle.
    pub(super) fn open(path: &Path, read_only: bool) -> Result<Self> {
        let excel = ExcelSession::new()?;
        let workbook = excel.open_workbook(path, read_only)?;
        Ok(Self { excel, workbook })
    }

    /// Close the workbook and quit Excel, optionally saving first.
    pub(super) fn close(mut self, save: bool) -> Result<()> {
        let close_result = self.workbook.method("Close", vec![ComVariant::bool(save)]);
        let quit_result = self.excel.quit();
        match (close_result, quit_result) {
            (Ok(_), Ok(_)) => Ok(()),
            (Err(e), _) => Err(e),
            (_, Err(e)) => Err(e),
        }
    }

    /// Save the workbook to disk.
    pub fn save(&self) -> Result<()> {
        self.workbook.method("Save", Vec::new())?;
        Ok(())
    }

    /// Full recalculation of all formulas.
    pub fn calculate(&self) -> Result<()> {
        self.excel.calculate_full_rebuild()?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "__tests__/workbook_tests.rs"]
mod tests;
