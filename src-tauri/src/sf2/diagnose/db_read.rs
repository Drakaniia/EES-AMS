//! Every database read the diagnostic makes, and nothing else.
//!
//! ## The connection
//!
//! One [`rusqlite::Connection`] opened with `SQLITE_OPEN_READ_ONLY`, and no
//! other handle to the file. The app's own pool is deliberately not used: it is
//! a read-write pool, and a diagnostic that borrows it has already given up the
//! ability to prove it wrote nothing. A read-only handle makes the guarantee
//! structural rather than a matter of reviewing which statements were chosen -
//! SQLite itself refuses the write.
//!
//! `SQLITE_OPEN_NO_MUTEX` because the connection is used from one thread at a
//! time and the flag avoids taking a mutex the diagnostic has no use for.
//!
//! ## Why the SQL is in `.sql` files
//!
//! Module convention, and it earns its keep here: the whole read side of this
//! feature is greppable for `INSERT` / `UPDATE` / `DELETE` / `PRAGMA =`, and
//! "there are none" is a check somebody can actually run rather than a claim.

use super::model::{
    AbsentRecord, EventTypeCount, MappingTableState, RosterRow, SheetDayGridSummary, TableRowCount,
};
use crate::domain::error::{AppError, Result};
use rusqlite::{params, Connection, OpenFlags};
use std::collections::HashMap;
use std::path::Path;

const SCHEMA_VERSION_SQL: &str = include_str!("sql/diag_schema_version.sql");
const TABLE_EXISTS_SQL: &str = include_str!("sql/diag_table_exists.sql");
const TABLE_ROW_COUNT_SQL: &str = include_str!("sql/diag_table_row_count.sql");
const LEGACY_TEMPLATES_SQL: &str = include_str!("sql/diag_legacy_templates.sql");
const LEGACY_STUDENT_MAPPINGS_SQL: &str = include_str!("sql/diag_legacy_student_mappings.sql");
const LEGACY_DATE_MAPPINGS_SQL: &str = include_str!("sql/diag_legacy_date_mappings_by_sheet.sql");
const MONTH_TEMPLATES_SQL: &str = include_str!("sql/diag_month_templates.sql");
const MONTH_STUDENT_MAPPINGS_SQL: &str = include_str!("sql/diag_month_student_mappings.sql");
const MONTH_DATE_MAPPINGS_SQL: &str = include_str!("sql/diag_month_date_mappings_by_month.sql");
const ABSENT_EVENTS_SQL: &str = include_str!("sql/diag_absent_events.sql");
const EVENT_COUNTS_SQL: &str = include_str!("sql/diag_event_counts_by_type.sql");
const STUDENTS_SQL: &str = include_str!("sql/diag_students.sql");

/// One row of the `students` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudentRow {
    pub id: String,
    pub name: String,
    pub class_id: Option<String>,
}

/// The legacy per-class SF2 template row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyTemplateRow {
    pub id: String,
    pub source_path: String,
    pub school_year: String,
    pub report_month: String,
    pub active_class_id: String,
    pub grade_level: String,
    pub section: String,
    pub last_synced_at: Option<i64>,
}

/// One `sf2_month_templates` row (spec §6.2 v19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthTemplateRow {
    pub id: String,
    pub active_class_id: String,
    pub school_year: String,
    pub report_month: String,
    pub report_year: i32,
    pub source_path: String,
    pub first_school_day: u32,
    pub last_synced_at: Option<i64>,
    pub workbook_x_count: i64,
    pub workbook_scanned_at: Option<i64>,
}

/// Everything the diagnostic knows about the database before Excel is involved.
#[derive(Debug, Clone, Default)]
pub struct DbSnapshot {
    pub schema_version: Option<i32>,
    pub legacy_templates: Vec<LegacyTemplateRow>,
    pub month_templates: Vec<MonthTemplateRow>,
    /// Keyed by `template_id`.
    pub legacy_rosters: HashMap<String, Vec<RosterRow>>,
    /// Keyed by `template_id`.
    pub month_rosters: HashMap<String, Vec<RosterRow>>,
    pub absent_events: Vec<AbsentRecord>,
    pub event_counts: Vec<EventTypeCount>,
    pub students: Vec<StudentRow>,
    pub tables: MappingTableState,
    pub total_absent_events: i64,
}

impl DbSnapshot {
    /// The template the comparison is anchored on.
    ///
    /// The legacy row wins when it exists, because that is the row whose
    /// `source_path` names the one workbook the app considers its own, and
    /// because on an install that has never been split it is the only row
    /// there is.
    #[must_use]
    pub fn anchor_template(&self) -> Option<&LegacyTemplateRow> {
        self.legacy_templates.first()
    }

    /// The class this workbook is for, from whichever table has a row.
    #[must_use]
    pub fn active_class_id(&self) -> Option<&str> {
        self.anchor_template()
            .map(|template| template.active_class_id.as_str())
            .or_else(|| {
                self.month_templates
                    .first()
                    .map(|template| template.active_class_id.as_str())
            })
    }

    /// The school year the twelve months hang off.
    #[must_use]
    pub fn school_year(&self) -> Option<&str> {
        self.anchor_template()
            .map(|template| template.school_year.as_str())
            .or_else(|| {
                self.month_templates
                    .first()
                    .map(|template| template.school_year.as_str())
            })
    }

    /// The roster rows the comparison should use for `template_id`.
    ///
    /// Per-month first, legacy second. The reverse order would be wrong: the
    /// legacy roster is the one the *current* file was imported with, and the
    /// per-month roster only exists once a split has run for that month.
    #[must_use]
    pub fn roster_for(&self, template_id: &str) -> (Vec<RosterRow>, super::model::MappingSource) {
        use super::model::MappingSource;

        let per_month = self.month_rosters.get(template_id);
        if let Some(rows) = per_month.filter(|rows| !rows.is_empty()) {
            return (rows.clone(), MappingSource::PerMonthTables);
        }
        let legacy = self.legacy_rosters.get(template_id);
        if let Some(rows) = legacy.filter(|rows| !rows.is_empty()) {
            return (rows.clone(), MappingSource::LegacyTables);
        }
        (Vec::new(), MappingSource::None)
    }
}

/// Open `path` read-only and read every fact the diagnostic needs from it.
pub fn read_snapshot(path: &Path) -> Result<DbSnapshot> {
    if !path.exists() {
        return Err(AppError::InvalidInput(format!(
            "there is no database at {}",
            path.display()
        )));
    }
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;

    let mut snapshot = DbSnapshot {
        schema_version: read_schema_version(&conn)?,
        ..DbSnapshot::default()
    };
    snapshot.tables = read_table_state(&conn)?;
    snapshot.legacy_templates = read_legacy_templates(&conn)?;
    snapshot.month_templates = read_month_templates(&conn)?;

    for template in &snapshot.legacy_templates {
        snapshot.legacy_rosters.insert(
            template.id.clone(),
            read_roster(&conn, LEGACY_STUDENT_MAPPINGS_SQL, &template.id)?,
        );
    }
    for template in &snapshot.month_templates {
        snapshot.month_rosters.insert(
            template.id.clone(),
            read_roster(&conn, MONTH_STUDENT_MAPPINGS_SQL, &template.id)?,
        );
    }

    if table_exists(&conn, "sf2_date_mappings")? {
        if let Some(template) = snapshot.anchor_template() {
            snapshot.tables.legacy_date_mapping_sheets =
                read_grid_summaries(&conn, LEGACY_DATE_MAPPINGS_SQL, &template.id)?;
        }
    }
    if table_exists(&conn, "sf2_month_date_mappings")? {
        for template in &snapshot.month_templates {
            snapshot
                .tables
                .month_date_mapping_grids
                .extend(read_grid_summaries(
                    &conn,
                    MONTH_DATE_MAPPINGS_SQL,
                    &template.id,
                )?);
        }
    }

    if table_exists(&conn, "events")? {
        snapshot.absent_events = read_absent_events(&conn)?;
        snapshot.event_counts = read_event_counts(&conn)?;
        snapshot.total_absent_events = snapshot
            .event_counts
            .iter()
            .find(|count| count.event_type == ABSENT_EVENT_TYPE)
            .map_or(0, |count| count.rows);
    }
    if table_exists(&conn, "students")? {
        snapshot.students = read_students(&conn)?;
    }

    Ok(snapshot)
}

/// The `event_type` value that is an X mark (spec §3.2).
pub const ABSENT_EVENT_TYPE: &str = "absent";

/// The legacy tables, then the per-month tables (spec §6.2).
pub const LEGACY_TABLES: [&str; 3] = ["sf2_templates", "sf2_student_mappings", "sf2_date_mappings"];
pub const PER_MONTH_TABLES: [&str; 3] = [
    "sf2_month_templates",
    "sf2_month_student_mappings",
    "sf2_month_date_mappings",
];

fn read_schema_version(conn: &Connection) -> Result<Option<i32>> {
    let version = conn.query_row(SCHEMA_VERSION_SQL, [], |row| row.get::<_, i32>(0));
    Ok(version.ok())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let matches: i64 = conn.query_row(TABLE_EXISTS_SQL, params![table], |row| row.get(0))?;
    Ok(matches > 0)
}

/// Row counts for the six mapping tables, with an explicit "no such table" for
/// the ones a pre-v19 database has never heard of.
fn read_table_state(conn: &Connection) -> Result<MappingTableState> {
    Ok(MappingTableState {
        legacy: read_row_counts(conn, &LEGACY_TABLES)?,
        per_month: read_row_counts(conn, &PER_MONTH_TABLES)?,
        legacy_date_mapping_sheets: Vec::new(),
        month_date_mapping_grids: Vec::new(),
    })
}

fn read_row_counts(conn: &Connection, tables: &[&str]) -> Result<Vec<TableRowCount>> {
    tables
        .iter()
        .map(|table| {
            let exists = table_exists(conn, table)?;
            let rows = if exists {
                // `table` is a literal from `LEGACY_TABLES` / `PER_MONTH_TABLES`
                // in this module, never anything read out of the database.
                Some(
                    conn.query_row(&TABLE_ROW_COUNT_SQL.replace("{table}", table), [], |row| {
                        row.get::<_, i64>(0)
                    })?,
                )
            } else {
                None
            };
            Ok(TableRowCount {
                table: (*table).to_string(),
                exists,
                rows,
            })
        })
        .collect()
}

fn read_legacy_templates(conn: &Connection) -> Result<Vec<LegacyTemplateRow>> {
    if !table_exists(conn, "sf2_templates")? {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare(LEGACY_TEMPLATES_SQL)?;
    let rows = statement.query_map([], |row| {
        Ok(LegacyTemplateRow {
            id: row.get(0)?,
            source_path: row.get(1)?,
            school_year: row.get(2)?,
            report_month: row.get(3)?,
            active_class_id: row.get(4)?,
            grade_level: row.get(5)?,
            section: row.get(6)?,
            last_synced_at: row.get(7)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn read_month_templates(conn: &Connection) -> Result<Vec<MonthTemplateRow>> {
    if !table_exists(conn, "sf2_month_templates")? {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare(MONTH_TEMPLATES_SQL)?;
    let rows = statement.query_map([], |row| {
        Ok(MonthTemplateRow {
            id: row.get(0)?,
            active_class_id: row.get(1)?,
            school_year: row.get(2)?,
            report_month: row.get(3)?,
            report_year: row.get(4)?,
            source_path: row.get(5)?,
            first_school_day: row.get(6)?,
            last_synced_at: row.get(7)?,
            workbook_x_count: row.get(8)?,
            workbook_scanned_at: row.get(9)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn read_roster(conn: &Connection, sql: &str, template_id: &str) -> Result<Vec<RosterRow>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(params![template_id], |row| {
        Ok(RosterRow {
            student_id: row.get(0)?,
            workbook_name: row.get(1)?,
            row_index: row.get::<_, u32>(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn read_grid_summaries(
    conn: &Connection,
    sql: &str,
    template_id: &str,
) -> Result<Vec<SheetDayGridSummary>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(params![template_id], |row| {
        Ok(SheetDayGridSummary {
            sheet_name: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
            year_month: row.get(1)?,
            first_date: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            last_date: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            day_columns: row.get::<_, i64>(4)? as usize,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn read_absent_events(conn: &Connection) -> Result<Vec<AbsentRecord>> {
    let mut statement = conn.prepare(ABSENT_EVENTS_SQL)?;
    let rows = statement.query_map([], |row| {
        Ok(AbsentRecord {
            student_id: row.get(0)?,
            class_id: row.get(1)?,
            date: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn read_event_counts(conn: &Connection) -> Result<Vec<EventTypeCount>> {
    let mut statement = conn.prepare(EVENT_COUNTS_SQL)?;
    let rows = statement.query_map([], |row| {
        Ok(EventTypeCount {
            event_type: row.get(0)?,
            rows: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn read_students(conn: &Connection) -> Result<Vec<StudentRow>> {
    let mut statement = conn.prepare(STUDENTS_SQL)?;
    let rows = statement.query_map([], |row| {
        Ok(StudentRow {
            id: row.get(0)?,
            name: row.get(1)?,
            class_id: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}
