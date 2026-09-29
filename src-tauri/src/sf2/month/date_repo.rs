//! `sf2_month_date_mappings` - one month's day-number grid, and the worksheet it
//! lives on.
//!
//! Every statement here is scoped by `template_id`. There is deliberately no
//! "delete this class's calendar": with twelve months sharing one workbook, a
//! re-analysis of September must not be able to reach October's columns (spec §4
//! factor 2).
//!
//! ## The `sheet_name` column
//!
//! One file holds twelve month worksheets, so a column letter is not an address
//! on its own - `F` means a different day on every sheet. v24 restored the column
//! v21 had dropped on the "one file per month, so the sheet is derivable"
//! reasoning; see [`crate::sf2::month::schema_v24`]. It is read here and written
//! here, and readers that need a name use
//! [`Sf2MonthDateMapping::resolved_sheet_name`], which derives the same string
//! from the date when the stored column is absent.

use crate::domain::error::{AppError, Result};
use crate::infrastructure::database::DbPool;
use crate::sf2::month::{Sf2MonthDateMapping, EMPTY_DATE_ANALYSIS_MESSAGE};
use rusqlite::{params, OptionalExtension};

const INSERT_DATE_MAPPING_SQL: &str = include_str!("../sql/month_insert_date_mapping.sql");
const DATE_MAPPINGS_FOR_TEMPLATE_SQL: &str =
    include_str!("../sql/month_date_mappings_for_template.sql");
const FIND_DATE_MAPPING_SQL: &str = include_str!("../sql/month_find_date_mapping.sql");
const DELETE_DATE_MAPPINGS_SQL: &str = include_str!("../sql/month_delete_date_mappings.sql");

/// CRUD for a month file's day-number grid.
pub struct Sf2MonthDateRepo {
    pool: DbPool,
}

impl Sf2MonthDateRepo {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Replace one month file's grid with `dates`, in a single transaction.
    ///
    /// An empty `dates` is rejected rather than committed. That is the guard
    /// the pre-split repository already applies, and it is not optional: an
    /// empty grid is the precondition for the destructive-sync chain, and a
    /// month that just lost its grid looks like a month with no school days -
    /// which is exactly the state SEPTEMBER 2026 was in on the install this
    /// project was written for.
    pub fn replace_for_template(
        &self,
        template_id: &str,
        dates: &[Sf2MonthDateMapping],
    ) -> Result<()> {
        if dates.is_empty() {
            return Err(AppError::InvalidInput(
                EMPTY_DATE_ANALYSIS_MESSAGE.to_string(),
            ));
        }

        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        transaction.execute(DELETE_DATE_MAPPINGS_SQL, params![template_id])?;
        {
            let mut statement = transaction.prepare(INSERT_DATE_MAPPING_SQL)?;
            for date in dates {
                statement.execute(params![
                    &date.template_id,
                    date.date,
                    date.column_letter,
                    date.column_index,
                    date.resolved_sheet_name(),
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// One month file's grid, in date order.
    pub fn for_template(&self, template_id: &str) -> Result<Vec<Sf2MonthDateMapping>> {
        let conn = self.pool.get()?;
        let mut statement = conn.prepare(DATE_MAPPINGS_FOR_TEMPLATE_SQL)?;
        let rows = statement.query_map(params![template_id], |row| {
            Ok(Sf2MonthDateMapping {
                template_id: row.get(0)?,
                date: row.get(1)?,
                column_letter: row.get(2)?,
                column_index: row.get::<_, u32>(3)?,
                sheet_name: row.get(4)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    /// The column a single date occupies in one month file.
    pub fn for_date(&self, template_id: &str, date: &str) -> Result<Option<Sf2MonthDateMapping>> {
        let conn = self.pool.get()?;
        conn.query_row(FIND_DATE_MAPPING_SQL, params![template_id, date], |row| {
            Ok(Sf2MonthDateMapping {
                template_id: row.get(0)?,
                date: row.get(1)?,
                column_letter: row.get(2)?,
                column_index: row.get::<_, u32>(3)?,
                sheet_name: row.get(4)?,
            })
        })
        .optional()
        .map_err(Into::into)
    }

    /// How many day columns a month file currently has. Zero is the "this month
    /// is not usable yet" signal, and the reason no write path may clear it.
    pub fn count_for_template(&self, template_id: &str) -> Result<usize> {
        Ok(self.for_template(template_id)?.len())
    }

    /// Drop one month file's grid and report how many rows went.
    ///
    /// Scoped to the template: another month's grid is not reachable from here.
    pub fn delete_for_template(&self, template_id: &str) -> Result<usize> {
        let conn = self.pool.get()?;
        conn.execute(DELETE_DATE_MAPPINGS_SQL, params![template_id])
            .map_err(Into::into)
    }
}

#[cfg(test)]
#[path = "__tests__/date_repo_tests.rs"]
mod tests;
