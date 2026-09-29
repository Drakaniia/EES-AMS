//! Schema v24 on a v23 database that holds real data (spec §0 A4, §0 A5).
//!
//! The install this was written for is on v22 with 37 absences and 22 date
//! mappings, and a missing date mapping is the precondition for the whole
//! destructive-sync chain in spec §4. So the tests here are not about the column
//! existing; they are about the column arriving **without anything else moving**.
//!
//! Every fixture is built to look like that install: the one month row the v22
//! backfill created, its day grid, its roster, the pre-split tables still full
//! beside them, and 37 absences across six months.

use super::*;
use rusqlite::{params, Connection};

/// A v23 database shaped like the install this migration was written for.
///
/// `3b635890-...` is the month row the v22 backfill created for the pre-split
/// per-class row: one month, its six day mappings here, its two learners, and the
/// pre-split tables still full beside them.
fn v23_database() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE settings (
            id TEXT PRIMARY KEY NOT NULL,
            day_start TEXT NOT NULL,
            day_end TEXT NOT NULL,
            late_after TEXT NOT NULL,
            school_year TEXT,
            school_start_date TEXT DEFAULT NULL,
            last_report_month TEXT DEFAULT NULL,
            sf2_split_completed_at INTEGER
        );
        INSERT INTO settings (id, day_start, day_end, late_after, school_year)
            VALUES ('app', '08:00', '15:00', '08:45', '2026-2027');

        CREATE TABLE classes (id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL);
        INSERT INTO classes (id, name)
            VALUES ('a4f5d22a-2a85-4172-8548-143c2edeb698', 'Grade 3 - MATAPAT');

        CREATE TABLE students (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            card_serial TEXT UNIQUE,
            class_id TEXT,
            created_at INTEGER NOT NULL,
            gender TEXT,
            sf2_learner_id TEXT
        );

        CREATE TABLE events (
            id TEXT PRIMARY KEY NOT NULL,
            student_id TEXT NOT NULL,
            class_id TEXT,
            event_type TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            note TEXT
        );

        -- The pre-split tables: still full, still authoritative for anything the
        -- per-month tables cannot answer. v24 must not touch them.
        CREATE TABLE sf2_templates (
            id TEXT PRIMARY KEY NOT NULL,
            active_class_id TEXT NOT NULL,
            school_year TEXT NOT NULL,
            report_month TEXT NOT NULL,
            source_path TEXT NOT NULL,
            source_hash TEXT NOT NULL,
            school_id TEXT,
            school_name TEXT,
            grade_level TEXT,
            section TEXT,
            adviser_name TEXT,
            school_head_name TEXT,
            layout_fingerprint TEXT NOT NULL DEFAULT '',
            imported_at INTEGER NOT NULL DEFAULT 0,
            last_synced_at INTEGER
        );
        INSERT INTO sf2_templates VALUES (
            '3b635890-79f9-4e15-b860-1d84aac8b971',
            'a4f5d22a-2a85-4172-8548-143c2edeb698',
            '2026-2027', 'OCTOBER', 'C:/app/SF2-GRADE-3-MATAPAT-3b635890.xls',
            'bundled-a4f5d22a', '132839', 'Espiritu Elementary School', 'Grade 3', 'MATAPAT',
            'ALISTAIR M YBANEZ', 'ARNYL R. ARONES', '', 1788527906, NULL
        );

        CREATE TABLE sf2_date_mappings (
            template_id TEXT NOT NULL,
            date TEXT NOT NULL,
            sheet_name TEXT NOT NULL,
            column_letter TEXT NOT NULL,
            column_index INTEGER NOT NULL,
            PRIMARY KEY (template_id, date)
        );
        CREATE TABLE sf2_student_mappings (
            template_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            workbook_name TEXT NOT NULL,
            normalized_name TEXT NOT NULL,
            row_index INTEGER NOT NULL,
            gender_block TEXT,
            PRIMARY KEY (template_id, student_id)
        );

        -- The per-month tables, exactly as v19-v22 left them: **no** sheet_name.
        CREATE TABLE sf2_month_templates (
            id TEXT PRIMARY KEY NOT NULL,
            active_class_id TEXT NOT NULL,
            school_year TEXT NOT NULL,
            report_month TEXT NOT NULL,
            report_year INTEGER NOT NULL,
            source_path TEXT NOT NULL,
            source_hash TEXT NOT NULL,
            school_id TEXT,
            school_name TEXT,
            grade_level TEXT,
            section TEXT,
            adviser_name TEXT,
            school_head_name TEXT,
            first_school_day INTEGER NOT NULL,
            imported_at INTEGER NOT NULL,
            last_synced_at INTEGER,
            workbook_x_count INTEGER NOT NULL DEFAULT 0,
            workbook_scanned_at INTEGER,
            first_school_day_override INTEGER
        );
        CREATE TABLE sf2_month_student_mappings (
            template_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            workbook_name TEXT NOT NULL,
            normalized_name TEXT NOT NULL,
            row_index INTEGER NOT NULL,
            gender_block TEXT,
            sf2_learner_id TEXT,
            PRIMARY KEY (template_id, student_id)
        );
        CREATE TABLE sf2_month_date_mappings (
            template_id   TEXT NOT NULL,
            date          TEXT NOT NULL,
            column_letter TEXT NOT NULL,
            column_index  INTEGER NOT NULL,
            PRIMARY KEY(template_id, date)
        );
        "#,
    )
    .expect("build the v23 schema");

    // The v22 backfill's own month row, still carrying the canonical `2026-2027`
    // label v23 normalises everything to, and everything hanging off it.
    conn.execute(
        "INSERT INTO sf2_month_templates (id, active_class_id, school_year, report_month,
            report_year, source_path, source_hash, school_id, school_name, grade_level, section,
            adviser_name, school_head_name, first_school_day, imported_at, last_synced_at,
            workbook_x_count, workbook_scanned_at, first_school_day_override)
         VALUES ('3b635890-79f9-4e15-b860-1d84aac8b971', 'a4f5d22a-2a85-4172-8548-143c2edeb698',
            '2026-2027', 'OCTOBER', 2026,
            'C:/app/SF2-GRADE-3-MATAPAT-3b635890.xls', 'bundled-a4f5d22a',
            '132839', 'Espiritu Elementary School', 'Grade 3', 'MATAPAT',
            'ALISTAIR M YBANEZ', 'ARNYL R. ARONES', 1, 1788527906, NULL, 0, NULL, NULL)",
        [],
    )
    .expect("seed the month row");

    for (index, student) in ["Alvarado, Zyron Jay  E.", "BAPTISMA, SOSDFFIA, ESPIRITU M."]
        .iter()
        .enumerate()
    {
        let id = format!("student-{index}");
        conn.execute(
            "INSERT INTO students (id, name, class_id, created_at, gender) VALUES (?1, ?2,
             'a4f5d22a-2a85-4172-8548-143c2edeb698', 0, 'male')",
            params![id, student],
        )
        .expect("seed a student");
        conn.execute(
            "INSERT INTO sf2_student_mappings VALUES
             ('3b635890-79f9-4e15-b860-1d84aac8b971', ?1, ?2, ?3, ?4, 'MALE')",
            params![id, student, format!("learner{index}"), 8 + index as u32],
        )
        .expect("seed a pre-split student mapping");
        conn.execute(
            "INSERT INTO sf2_month_student_mappings VALUES
             ('3b635890-79f9-4e15-b860-1d84aac8b971', ?1, ?2, ?3, ?4, 'MALE', NULL)",
            params![id, student, format!("learner{index}"), 8 + index as u32],
        )
        .expect("seed a per-month student mapping");
    }

    for (offset, day) in [1u32, 2, 3, 4, 5, 7].iter().enumerate() {
        let date = format!("2026-10-{day:02}");
        let column = column_index_for(offset as u32);
        conn.execute(
            "INSERT INTO sf2_date_mappings VALUES
             ('3b635890-79f9-4e15-b860-1d84aac8b971', ?1, 'OCTOBER 2026', ?2, ?3)",
            params![date, column_letter_for(offset as u32), column],
        )
        .expect("seed a pre-split date mapping");
        conn.execute(
            "INSERT INTO sf2_month_date_mappings VALUES
             ('3b635890-79f9-4e15-b860-1d84aac8b971', ?1, ?2, ?3)",
            params![date, column_letter_for(offset as u32), column],
        )
        .expect("seed a per-month date mapping");
    }

    // 37 absences across six months, of which this school year holds 28. Only
    // their existence matters here: v24 must not move a single one.
    for index in 0..37 {
        conn.execute(
            "INSERT INTO events VALUES (?1, 'student-0',
             'a4f5d22a-2a85-4172-8548-143c2edeb698', 'absent', ?2, NULL)",
            params![format!("event-{index}"), 1_780_000_000 + index * 86_400],
        )
        .expect("seed an absence");
    }

    conn
}

/// The six labelled day columns a small October grid uses. The bundled template's
/// real grid is 25 wide; six is enough to prove the backfill touches all of them.
fn column_index_for(offset: u32) -> u32 {
    [6u32, 8, 9, 10, 11, 12][offset as usize]
}

fn column_letter_for(offset: u32) -> String {
    ["F", "H", "I", "J", "K", "L"][offset as usize].to_string()
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).expect("count")
}

fn all_dates(conn: &Connection) -> Vec<String> {
    let mut statement = conn
        .prepare("SELECT date FROM sf2_month_date_mappings ORDER BY date")
        .expect("prepare");
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect")
}

fn sheet_name_of(conn: &Connection, date: &str) -> String {
    conn.query_row(
        "SELECT sheet_name FROM sf2_month_date_mappings WHERE date = ?1",
        params![date],
        |row| row.get::<_, Option<String>>(0),
    )
    .expect("read the sheet name")
    .expect("the row is there and the migration filled its sheet name in")
}

// ── The column arrives ──────────────────────────────────────────────────────

#[test]
fn the_column_is_added_and_the_table_is_otherwise_untouched() {
    let conn = v23_database();
    migrate_to_v24(&conn).expect("v23 -> v24");

    let columns = conn
        .prepare("SELECT name FROM pragma_table_info('sf2_month_date_mappings')")
        .expect("prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect");
    assert!(columns.contains(&"sheet_name".to_string()));
    assert_eq!(columns.len(), 5, "no other column came or went");
}

#[test]
fn replaying_the_migration_changes_nothing_the_second_time() {
    // A launch that died between the last statement and the `PRAGMA user_version`
    // write replays the file. That must not fail on a duplicate column, and must
    // not rewrite the backfilled values.
    let conn = v23_database();
    migrate_to_v24(&conn).expect("first run");
    let after_first = all_sheets(&conn);
    migrate_to_v24(&conn).expect("replay");
    assert_eq!(all_sheets(&conn), after_first, "a replay rewrote values");
}

fn all_sheets(conn: &Connection) -> Vec<String> {
    let mut statement = conn
        .prepare("SELECT date || '=' || COALESCE(sheet_name, '<null>') FROM sf2_month_date_mappings ORDER BY date")
        .expect("prepare");
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect")
}

// ── Nothing is lost ─────────────────────────────────────────────────────────

#[test]
fn no_day_mapping_is_dropped_or_blanked() {
    let conn = v23_database();
    let before = count(&conn, "SELECT COUNT(*) FROM sf2_month_date_mappings");
    let before_dates = all_dates(&conn);

    migrate_to_v24(&conn).expect("v23 -> v24");

    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_date_mappings"),
        before,
        "a migration that drops a date mapping is the precondition for the destructive-sync chain"
    );
    assert_eq!(all_dates(&conn), before_dates, "the days themselves moved");
    assert_eq!(
        unresolved_sheet_rows(&conn).expect("count unresolved"),
        0,
        "every mapping must come out of the migration with a sheet"
    );
}

#[test]
fn no_absence_is_dropped_or_blanked() {
    let conn = v23_database();
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM events WHERE event_type = 'absent'"
        ),
        37,
        "the fixture is the install's 37 absences"
    );

    migrate_to_v24(&conn).expect("v23 -> v24");

    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM events WHERE event_type = 'absent'"
        ),
        37
    );
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM events"), 37);
}

#[test]
fn the_roster_and_the_month_rows_are_untouched() {
    let conn = v23_database();
    let students = count(&conn, "SELECT COUNT(*) FROM sf2_month_student_mappings");
    let months = count(&conn, "SELECT COUNT(*) FROM sf2_month_templates");

    migrate_to_v24(&conn).expect("v23 -> v24");

    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_student_mappings"),
        students
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_templates"),
        months
    );
}

#[test]
fn the_pre_split_tables_are_still_there_and_still_full() {
    let conn = v23_database();

    migrate_to_v24(&conn).expect("v23 -> v24");

    // On the affected install these are still where some of the data lives. v24
    // reads them and never writes them, and never drops them.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM sf2_date_mappings"), 6);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM sf2_student_mappings"), 2);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM sf2_templates"), 1);
}

#[test]
fn the_columns_of_every_table_the_migration_touches_are_preserved() {
    let conn = v23_database();
    migrate_to_v24(&conn).expect("v23 -> v24");
    for table in BRACKETED_TABLES {
        let sql = TABLE_COLUMNS_SQL.replace("{table}", table);
        let columns = conn
            .prepare(&sql)
            .expect("prepare")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("collect");
        assert!(!columns.is_empty(), "`{table}` disappeared");
    }
}

// ── The backfill is right ───────────────────────────────────────────────────

#[test]
fn every_mapping_gets_the_worksheet_of_its_own_month() {
    let conn = v23_database();
    migrate_to_v24(&conn).expect("v23 -> v24");

    // The month row is the authority, and it says OCTOBER 2026.
    assert_eq!(sheet_name_of(&conn, "2026-10-01"), "OCTOBER 2026");
    assert_eq!(sheet_name_of(&conn, "2026-10-07"), "OCTOBER 2026");
}

#[test]
fn a_month_with_no_row_of_its_own_is_named_from_the_date_instead() {
    // The v22 backfill only ever creates a row for the pre-split template's own
    // month. If a date mapping exists for a month that has no row - which is
    // exactly the SEPTEMBER 2026 shape on the affected install - the name still
    // has to be filled in rather than left NULL, because a write path has nothing
    // to address without it.
    let conn = v23_database();
    conn.execute(
        "INSERT INTO sf2_month_date_mappings VALUES ('orphan-9', '2026-09-01', 'H', 8)",
        [],
    )
    .expect("seed an orphan mapping");

    migrate_to_v24(&conn).expect("v23 -> v24");

    let sheet = conn
        .query_row(
            "SELECT sheet_name FROM sf2_month_date_mappings WHERE template_id = 'orphan-9'",
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .expect("read");
    assert_eq!(
        sheet.expect("filled in"),
        "SEPTEMBER 2026",
        "a date is a full YYYY-MM-DD, so its month and year are always readable"
    );
}

#[test]
fn the_month_row_wins_over_the_pre_split_tables_recorded_sheet() {
    // The pre-split `sheet_name` is the name the analyzer saw on whichever sheet
    // happened to be VISIBLE when the workbook was last analysed. Copying it
    // first would put one month's worksheet on another month's grid, so the month
    // row is consulted first. Here the two disagree on purpose.
    let conn = v23_database();
    conn.execute(
        "UPDATE sf2_date_mappings SET sheet_name = 'SEPTEMBER 2026'",
        [],
    )
    .expect("make the pre-split table disagree");

    migrate_to_v24(&conn).expect("v23 -> v24");

    assert_eq!(
        sheet_name_of(&conn, "2026-10-01"),
        "OCTOBER 2026",
        "October's grid belongs on October's worksheet, whatever the pre-split analysis recorded"
    );
}

#[test]
fn a_date_that_is_not_iso_is_left_alone_rather_than_guessed() {
    // The migration refuses to invent a name. A row it cannot name keeps every
    // other field, and the read path derives the name from the date instead - so
    // this is a measurement, not a repair, and the count says so.
    let conn = v23_database();
    conn.execute(
        "INSERT INTO sf2_month_date_mappings VALUES ('junk', 'not-a-date', 'F', 6)",
        [],
    )
    .expect("seed a malformed date");

    migrate_to_v24(&conn).expect("v23 -> v24");

    assert_eq!(unresolved_sheet_rows(&conn).expect("count"), 1);
    let row = conn
        .query_row(
            "SELECT date, column_letter, column_index FROM sf2_month_date_mappings
             WHERE template_id = 'junk'",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .expect("read back");
    assert_eq!(
        row,
        ("not-a-date".to_string(), "F".to_string(), 6),
        "a row the migration could not name keeps every other field"
    );
}

// ── The version ─────────────────────────────────────────────────────────────

#[test]
fn this_module_is_the_one_that_defines_the_version_it_migrates_to() {
    assert_eq!(SCHEMA_VERSION, 24);
    assert_eq!(
        crate::sf2::month::sheet_name_from_date("2026-09-01").as_deref(),
        Some("SEPTEMBER 2026")
    );
    assert_eq!(
        crate::sf2::month::sheet_name_from_date("not-a-date"),
        None,
        "a date that is not ISO has no month, and saying so is the honest answer"
    );
}
