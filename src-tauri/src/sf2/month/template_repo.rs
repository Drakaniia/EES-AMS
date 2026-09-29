//! `sf2_month_templates` - the 12 rows that make up a school year.
//!
//! Every lookup here is keyed on `(class, school year, month)` or on the row
//! id, never on "the class's current month". That is the whole point of the
//! table: a month is a row, so switching months is a read rather than a
//! mutation of shared state.

use crate::domain::error::{AppError, Result};
use crate::infrastructure::database::DbPool;
use crate::sf2::month::first_school_day::normalize_school_year;
use crate::sf2::month::Sf2MonthTemplate;
use rusqlite::{params, OptionalExtension, Row};

const FIND_MONTH_TEMPLATE_SQL: &str = include_str!("../sql/month_find_template.sql");
const FIND_MONTH_TEMPLATE_BY_ID_SQL: &str = include_str!("../sql/month_find_template_by_id.sql");
const LIST_MONTHS_FOR_SCHOOL_YEAR_SQL: &str = include_str!("../sql/month_list_for_school_year.sql");
const LIST_ALL_MONTHS_SQL: &str = include_str!("../sql/month_list_all.sql");
const UPSERT_MONTH_TEMPLATE_SQL: &str = include_str!("../sql/month_upsert_template.sql");
const UPDATE_MONTH_TEMPLATE_SQL: &str = include_str!("../sql/month_update_template.sql");
const DELETE_MONTH_TEMPLATE_SQL: &str = include_str!("../sql/month_delete_template.sql");
const OVERRIDE_FIRST_SCHOOL_DAY_SQL: &str =
    include_str!("../sql/month_override_first_school_day.sql");
const DERIVE_FIRST_SCHOOL_DAY_SQL: &str = include_str!("../sql/month_derive_first_school_day.sql");
const CLEAR_FIRST_SCHOOL_DAY_OVERRIDE_SQL: &str =
    include_str!("../sql/month_clear_first_school_day_override.sql");
const SET_LAST_SYNCED_AT_SQL: &str = include_str!("../sql/month_set_last_synced_at.sql");
const CLEAR_LAST_SYNCED_AT_FOR_CLASS_SQL: &str =
    include_str!("../sql/month_clear_last_synced_at_for_class.sql");
const RECORD_WORKBOOK_X_COUNT_SQL: &str = include_str!("../sql/month_record_workbook_x_count.sql");

/// CRUD for the month workbook rows.
pub struct Sf2MonthTemplateRepo {
    pool: DbPool,
}

impl Sf2MonthTemplateRepo {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// The row for one month of one school year.
    ///
    /// This is the read a month switch performs.
    ///
    /// The school year is normalised on the way in as well as on the way out
    /// (see [`normalize_school_year`]). A row stored before v23 with
    /// `2026 - 2027` in it must still be found when the caller asks for
    /// `2026-2027`, because equality on the label is what the whole per-month
    /// model is keyed on and a mismatch makes the table silently unreachable.
    pub fn find(
        &self,
        active_class_id: &str,
        school_year: &str,
        report_month: &str,
    ) -> Result<Option<Sf2MonthTemplate>> {
        let conn = self.pool.get()?;
        conn.query_row(
            FIND_MONTH_TEMPLATE_SQL,
            params![
                active_class_id,
                normalize_school_year(school_year),
                report_month
            ],
            read_month_template,
        )
        .optional()
        .map_err(Into::into)
    }

    /// The row for a month file by id.
    pub fn find_by_id(&self, template_id: &str) -> Result<Option<Sf2MonthTemplate>> {
        let conn = self.pool.get()?;
        conn.query_row(
            FIND_MONTH_TEMPLATE_BY_ID_SQL,
            params![template_id],
            read_month_template,
        )
        .optional()
        .map_err(Into::into)
    }

    /// Every month row of one school year, ordered the way the school year runs
    /// (SEPTEMBER -> AUGUST). Fewer than 12 rows is normal: months are created
    /// as the split or the user creates them.
    ///
    /// Normalised on the way in, for the reason [`Self::find`] documents.
    pub fn list_for_school_year(
        &self,
        active_class_id: &str,
        school_year: &str,
    ) -> Result<Vec<Sf2MonthTemplate>> {
        let conn = self.pool.get()?;
        collect_month_templates(
            &conn,
            LIST_MONTHS_FOR_SCHOOL_YEAR_SQL,
            params![active_class_id, normalize_school_year(school_year)],
        )
        .map_err(Into::into)
    }

    /// Every month row on record, newest school year first.
    pub fn list_all(&self) -> Result<Vec<Sf2MonthTemplate>> {
        let conn = self.pool.get()?;
        let mut templates = collect_month_templates(&conn, LIST_ALL_MONTHS_SQL, [])?;
        templates.sort_by_key(|template| {
            (
                std::cmp::Reverse(template.school_year.clone()),
                template.school_year_order_key(),
            )
        });
        Ok(templates)
    }

    /// Create the row for a month file, or refresh the metadata of the row
    /// already there.
    ///
    /// Leaves the first attendance day, the sync timestamp and the X
    /// measurement alone when the row already exists: those are provenance and
    /// state, not metadata, and a retried split re-inserting its row must not
    /// move a month's first day or forget that it was already synced.
    ///
    /// The school year is stored in its canonical form, because the conflict
    /// target `(active_class_id, school_year, report_month)` is what decides
    /// whether this is a new row or a refresh. Storing `2026 - 2027` beside an
    /// existing `2026-2027` would insert a **second** row for the same month,
    /// and the unique index would not catch it because the strings differ.
    pub fn upsert(&self, template: &Sf2MonthTemplate) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            UPSERT_MONTH_TEMPLATE_SQL,
            params![
                template.id,
                template.active_class_id,
                normalize_school_year(&template.school_year),
                template.report_month,
                template.report_year,
                template.source_path,
                template.source_hash,
                template.school_id,
                template.school_name,
                template.grade_level,
                template.section,
                template.adviser_name,
                template.school_head_name,
                template.first_school_day,
                template.imported_at,
                template.last_synced_at,
                template.workbook_x_count,
                template.workbook_scanned_at,
            ],
        )?;
        Ok(())
    }

    /// Refresh a month file's metadata after it was re-opened.
    ///
    /// Normalised on the way in, for the reason [`Self::upsert`] documents.
    pub fn update(&self, template: &Sf2MonthTemplate) -> Result<()> {
        let conn = self.pool.get()?;
        let rows = conn.execute(
            UPDATE_MONTH_TEMPLATE_SQL,
            params![
                template.id,
                normalize_school_year(&template.school_year),
                template.report_month,
                template.report_year,
                template.source_path,
                template.source_hash,
                template.school_id,
                template.school_name,
                template.grade_level,
                template.section,
                template.adviser_name,
                template.school_head_name,
                template.imported_at,
                template.last_synced_at,
            ],
        )?;
        if rows == 0 {
            return Err(missing_month_row(&template.id));
        }
        Ok(())
    }

    /// Record a per-month first-attendance-day override (spec D16).
    ///
    /// From here on, re-deriving the value from `school_start_date` cannot
    /// change it - that is the promise the Reports sidebar override makes.
    pub fn override_first_school_day(&self, template_id: &str, day: u32) -> Result<bool> {
        let conn = self.pool.get()?;
        let rows = conn.execute(OVERRIDE_FIRST_SCHOOL_DAY_SQL, params![template_id, day])?;
        Ok(rows > 0)
    }

    /// Write a *derived* first attendance day.
    ///
    /// Returns `false` - without error - when the month already carries an
    /// override, because a caller re-deriving every month on startup must not
    /// treat a protected value as a failure. It also refuses to write the
    /// "not derived yet" sentinel over a real value.
    pub fn derive_first_school_day(&self, template_id: &str, day: u32) -> Result<bool> {
        let conn = self.pool.get()?;
        let rows = conn.execute(DERIVE_FIRST_SCHOOL_DAY_SQL, params![template_id, day])?;
        Ok(rows > 0)
    }

    /// Drop an override and go back to `derived_day`.
    ///
    /// Pass [`crate::sf2::month::FIRST_SCHOOL_DAY_UNDETERMINED`] to leave the
    /// month undated until `school_start_date` is known.
    pub fn clear_first_school_day_override(
        &self,
        template_id: &str,
        derived_day: u32,
    ) -> Result<bool> {
        let conn = self.pool.get()?;
        let rows = conn.execute(
            CLEAR_FIRST_SCHOOL_DAY_OVERRIDE_SQL,
            params![template_id, derived_day],
        )?;
        Ok(rows > 0)
    }

    /// Timestamp (seconds) of the last successful attendance -> workbook sync
    /// for this month.
    pub fn set_last_synced_at(&self, template_id: &str, synced_at: Option<i64>) -> Result<bool> {
        let conn = self.pool.get()?;
        let rows = conn.execute(SET_LAST_SYNCED_AT_SQL, params![template_id, synced_at])?;
        Ok(rows > 0)
    }

    /// Mark every one of the class's months as not-yet-synced.
    ///
    /// A grid correction writes to `events`, which belongs to no month, so all
    /// twelve of the class's files are stale with respect to it the moment it
    /// lands. Scoped by class for exactly that reason.
    ///
    /// This exists because the correction path used to reset only the *pre-split*
    /// row's timestamp. On an install that has been fully migrated, that row does
    /// not exist, so the reset silently reached nothing and a corrected mark
    /// could then be skipped by the next open's freshness check.
    pub fn clear_last_synced_at_for_class(&self, class_id: &str) -> Result<usize> {
        let conn = self.pool.get()?;
        Ok(conn.execute(CLEAR_LAST_SYNCED_AT_FOR_CLASS_SQL, params![class_id])?)
    }

    /// Store the X count last counted in the file, and when it was counted.
    ///
    /// `scanned_at` is what separates "the file has no X marks" from "the file
    /// was never measured". Only this method sets it, and a month whose file is
    /// missing is never measured at all.
    pub fn record_workbook_x_count(
        &self,
        template_id: &str,
        x_count: i64,
        scanned_at: i64,
    ) -> Result<bool> {
        let conn = self.pool.get()?;
        let rows = conn.execute(
            RECORD_WORKBOOK_X_COUNT_SQL,
            params![template_id, x_count, scanned_at],
        )?;
        Ok(rows > 0)
    }

    /// Forget a month row. Its date and student mappings cascade with it.
    ///
    /// Never deletes the `.xls` file - a missing row is recoverable by
    /// re-deriving it, a deleted workbook is not.
    pub fn delete(&self, template_id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let rows = conn.execute(DELETE_MONTH_TEMPLATE_SQL, params![template_id])?;
        Ok(rows > 0)
    }
}

fn collect_month_templates(
    conn: &rusqlite::Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> rusqlite::Result<Vec<Sf2MonthTemplate>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(params, read_month_template)?;
    let mut templates = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    templates.sort_by_key(Sf2MonthTemplate::school_year_order_key);
    Ok(templates)
}

fn missing_month_row(template_id: &str) -> AppError {
    AppError::InvalidInput(format!(
        "No SF2 month workbook is stored for `{template_id}`"
    ))
}

fn read_month_template(row: &Row<'_>) -> rusqlite::Result<Sf2MonthTemplate> {
    let school_year: String = row.get(2)?;
    Ok(Sf2MonthTemplate {
        id: row.get(0)?,
        active_class_id: row.get(1)?,
        // Normalised on read as well as on write. Normalising only the write
        // side would make every row this build creates findable and every row
        // an older build created invisible, which is the same class of silent
        // miss; normalising only the read side would leave two formats in one
        // database for every future reader to trip over. Both sides is the only
        // version that is not a latent bug.
        school_year: normalize_school_year(&school_year),
        report_month: row.get(3)?,
        report_year: row.get(4)?,
        source_path: row.get(5)?,
        source_hash: row.get(6)?,
        school_id: row.get(7)?,
        school_name: row.get(8)?,
        grade_level: row.get(9)?,
        section: row.get(10)?,
        adviser_name: row.get(11)?,
        school_head_name: row.get(12)?,
        first_school_day: row.get(13)?,
        first_school_day_override: row.get(14)?,
        imported_at: row.get(15)?,
        last_synced_at: row.get(16)?,
        workbook_x_count: row.get(17)?,
        workbook_scanned_at: row.get(18)?,
    })
}

#[cfg(test)]
#[path = "__tests__/template_repo_tests.rs"]
mod tests;
