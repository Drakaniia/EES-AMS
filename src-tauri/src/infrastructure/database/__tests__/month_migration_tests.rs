//! The v19 - v23 migration chain, end to end.
//!
//! The single most important thing these tests exist to prove: the app starts
//! on a brand-new database *and* on a v18 database that already holds SF2 data,
//! with nothing lost on the way to v23.

use super::*;

/// A migrated, empty database in a throwaway directory.
fn test_pool() -> DbPool {
    let dir = tempfile::tempdir().expect("temp dir for the test database");
    let pool = init_db(dir.path().join("attendance.db")).expect("migrate test database");
    std::mem::forget(dir);
    pool
}

fn schema_version(conn: &rusqlite::Connection) -> i32 {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("read user_version")
}

fn table_exists(conn: &rusqlite::Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table],
        |row| row.get::<_, i64>(0),
    )
    .expect("query sqlite_master")
        > 0
}

fn column_exists_in(conn: &rusqlite::Connection, table: &str, column: &str) -> bool {
    conn.query_row(
        &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = '{column}'"),
        [],
        |row| row.get::<_, i64>(0),
    )
    .expect("query pragma_table_info")
        > 0
}

fn count(conn: &rusqlite::Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get::<_, i64>(0))
        .expect("count rows")
}

/// Replay v1 - v18 so a fixture can be written at v18 and then migrated.
///
/// `migrate_db` always goes to the current version, so the older steps are
/// called one at a time - the same functions `migrate_db` itself calls.
fn replay_to_v18(conn: &rusqlite::Connection) {
    migrate_to_v1(conn).expect("v1");
    migrate_to_v2(conn).expect("v2");
    migrate_to_v3(conn).expect("v3");
    migrate_to_v4(conn).expect("v4");
    migrate_to_v5(conn).expect("v5");
    migrate_to_v6(conn).expect("v6");
    migrate_to_v7(conn).expect("v7");
    migrate_to_v8(conn).expect("v8");
    migrate_to_v9(conn).expect("v9");
    migrate_to_v10(conn).expect("v10");
    migrate_to_v11(conn).expect("v11");
    migrate_to_v12(conn).expect("v12");
    migrate_to_v13(conn).expect("v13");
    migrate_to_v14(conn).expect("v14");
    migrate_to_v15(conn).expect("v15");
    migrate_to_v16(conn).expect("v16");
    migrate_to_v17(conn).expect("v17");
    migrate_to_v18(conn).expect("v18");
    conn.execute("PRAGMA user_version = 18", [])
        .expect("stamp v18");
}

/// A v18 database with a realistic SF2 install in it: one class, two students,
/// absences, a legacy template pointing at the pre-split per-class workbook,
/// its roster, and its September day-number grid.
fn v18_database_with_sf2_data() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign keys");
    replay_to_v18(&conn);

    conn.execute_batch(
        r#"
        INSERT INTO classes (id, name, day_start, day_end, late_after, created_at)
        VALUES ('class-1', 'Grade 1 - MATAPAT', '07:00', '13:00', '07:30', 1);

        INSERT INTO students (id, name, class_id, created_at)
        VALUES ('student-1', 'CUARES, JAIRO', 'class-1', 1);

        INSERT INTO students (id, name, class_id, created_at)
        VALUES ('student-2', 'ESPENIDO, JANLEE', 'class-1', 1);

        INSERT INTO events (id, student_id, class_id, event_type, timestamp, note)
        VALUES ('event-1', 'student-1', 'class-1', 'absent', 1783000000, NULL);
        INSERT INTO events (id, student_id, class_id, event_type, timestamp, note)
        VALUES ('event-2', 'student-2', 'class-1', 'absent', 1783000100, NULL);
        INSERT INTO events (id, student_id, class_id, event_type, timestamp, note)
        VALUES ('event-3', 'student-1', 'class-1', 'in', 1783000200, NULL);

        -- v1 already seeded the single `app` settings row, so a real install
        -- configures that row rather than adding a second one.
        UPDATE settings
        SET day_start = '08:00', day_end = '15:00', late_after = '08:45',
            quarter = '1st Quarter', report_month = 'SEPTEMBER',
            school_year = '2026-2027'
        WHERE id = 'app';

        INSERT INTO sf2_templates (
            id, source_path, source_hash, school_year, grade_level, section,
            layout_fingerprint, active_class_id, imported_at, school_id, school_name,
            report_month, adviser_name, school_head_name, last_synced_at
        ) VALUES (
            '1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d',
            'C:/Users/teacher/AppData/Roaming/com.ees.ams/sf2-workbooks/SF2-1-MATAPAT-1a2b3c4d.xls',
            'abc123', '2026-2027', '1', 'MATAPAT', 'fingerprint', 'class-1', 1782000000,
            '132839', 'ESPIRITU ELEMENTARY SCHOOL', 'SEPTEMBER', 'DELA CRUZ, JUAN',
            'SANTOS, MARIA', 1782500000
        );

        INSERT INTO sf2_student_mappings (
            template_id, student_id, workbook_name, normalized_name, row_index, gender_block
        ) VALUES (
            '1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d', 'student-1',
            'CUARES,JAIRO, ESPIRITU', 'CUARES,JAIRO, ESPIRITU', 8, 'MALE'
        );

        INSERT INTO sf2_student_mappings (
            template_id, student_id, workbook_name, normalized_name, row_index, gender_block
        ) VALUES (
            '1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d', 'student-2',
            'ESPENIDO,JANLEE, TENIO', 'ESPENIDO,JANLEE, TENIO', 9, 'MALE'
        );

        INSERT INTO sf2_date_mappings (template_id, sheet_name, date, column_letter, column_index)
        VALUES ('1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d', 'SEPTEMBER 2026', '2026-09-01', 'F', 6);
        INSERT INTO sf2_date_mappings (template_id, sheet_name, date, column_letter, column_index)
        VALUES ('1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d', 'SEPTEMBER 2026', '2026-09-02', 'H', 8);
        INSERT INTO sf2_date_mappings (template_id, sheet_name, date, column_letter, column_index)
        VALUES ('1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d', 'SEPTEMBER 2026', '2026-09-03', 'I', 9);
        "#,
    )
    .expect("seed a realistic v18 SF2 install");

    conn
}

#[test]
fn a_brand_new_database_migrates_all_the_way_to_the_current_version() {
    // Most installs never used SF2 at all, so the whole chain has to be safe on
    // a database with no SF2 data whatsoever.
    let pool = test_pool();
    let conn = pool.get().expect("connection");

    // Stated as `CURRENT_SCHEMA_VERSION` rather than a literal, so adding a
    // migration cannot leave this test quietly asserting a version the app no
    // longer comes up on - which is exactly how a stale assertion hides.
    assert_eq!(
        schema_version(&conn),
        CURRENT_SCHEMA_VERSION,
        "the app must come up on the current schema version"
    );

    for table in [
        "sf2_month_templates",
        "sf2_month_student_mappings",
        "sf2_month_date_mappings",
    ] {
        assert!(
            table_exists(&conn, table),
            "{table} must exist at the current version"
        );
    }

    for column in [
        "school_start_date",
        "last_report_month",
        "sf2_split_completed_at",
    ] {
        assert!(
            column_exists_in(&conn, "settings", column),
            "settings.{column} must exist at the current version"
        );
    }
    assert!(column_exists_in(&conn, "students", "sf2_learner_id"));
    assert!(column_exists_in(
        &conn,
        "sf2_month_templates",
        "first_school_day_override"
    ));

    // An empty install stays empty: the backfill has nothing to copy and must
    // not invent a month row for a template that does not exist.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM sf2_month_templates"), 0);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_student_mappings"),
        0
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_date_mappings"),
        0
    );
    // `school_start_date` stays NULL: unset means unset, and a fresh install has
    // not been asked yet.
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM settings WHERE school_start_date IS NOT NULL"
        ),
        0
    );
}

#[test]
fn migrating_a_v18_database_with_sf2_data_lands_on_the_current_version_with_nothing_lost() {
    let conn = v18_database_with_sf2_data();
    assert_eq!(schema_version(&conn), 18);

    let events_before = count(&conn, "SELECT COUNT(*) FROM events");
    let students_before = count(&conn, "SELECT COUNT(*) FROM students");

    migrate_db(&conn).expect("migrate to the current version");

    assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);

    // The attendance record is untouched - the whole point.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM events"), events_before);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM students"),
        students_before
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM events WHERE event_type = 'absent'"
        ),
        2
    );

    // The legacy template became a SEPTEMBER month row pointing at the same
    // file, so the install keeps working before the split runs (E14).
    let month: (String, String, i32, String) = conn
        .query_row(
            "SELECT report_month, school_year, report_year, source_path
             FROM sf2_month_templates",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("the backfilled month row");
    assert_eq!(month.0, "SEPTEMBER");
    assert_eq!(month.1, "2026-2027");
    assert_eq!(month.2, 2026, "September is in the start year");
    assert!(
        month.3.ends_with("SF2-1-MATAPAT-1a2b3c4d.xls"),
        "the pre-split file is still the source: {}",
        month.3
    );

    // Every mapping came across, for that month only.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_student_mappings"),
        students_before
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_date_mappings"),
        3
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM sf2_month_date_mappings WHERE column_letter = 'H'"
        ),
        1,
        "the column letters came across intact"
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM sf2_month_date_mappings WHERE date = '2026-09-01'"
        ),
        1
    );

    // The first attendance day is the earliest Monday-Friday date the legacy
    // analysis recorded: 1 September 2026 is a Tuesday, so 1.
    let first_school_day: i64 = conn
        .query_row(
            "SELECT first_school_day FROM sf2_month_templates",
            [],
            |row| row.get(0),
        )
        .expect("read the derived first school day");
    assert_eq!(first_school_day, 1);
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM sf2_month_templates WHERE first_school_day_override IS NOT NULL"
        ),
        0,
        "a backfilled value is derived, never an override"
    );

    // The legacy tables are still there. They are dropped in a later migration,
    // once the split has been verified - not here.
    for table in ["sf2_templates", "sf2_student_mappings", "sf2_date_mappings"] {
        assert!(
            table_exists(&conn, table),
            "{table} must survive the migration"
        );
    }
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM sf2_templates"), 1);
}

#[test]
fn the_backfill_creates_exactly_one_month_row_per_legacy_template() {
    // A legacy template has exactly one month, so the backfill must not fan out
    // into twelve rows pointing at one file.
    let conn = v18_database_with_sf2_data();
    migrate_db(&conn).expect("migrate to the current version");

    assert_eq!(count(&conn, "SELECT COUNT(*) FROM sf2_month_templates"), 1);
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(DISTINCT report_month) FROM sf2_month_templates"
        ),
        1
    );
    // The mappings landed on that one row, not on eleven empty ones.
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(DISTINCT template_id) FROM sf2_month_student_mappings"
        ),
        1
    );
}

#[test]
fn the_v22_backfill_is_replayable() {
    // A launch that dies after the backfill but before `user_version` is
    // written replays the whole migration. It has to be a no-op, not a
    // duplicate-column crash - otherwise the app no longer starts.
    let conn = v18_database_with_sf2_data();
    migrate_db(&conn).expect("first run");
    conn.execute("PRAGMA user_version = 18", [])
        .expect("pretend the version stamp was lost");

    migrate_db(&conn).expect("replay");

    assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_templates"),
        1,
        "the replay must not duplicate the month row"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_student_mappings"),
        2
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_date_mappings"),
        3
    );
}

#[test]
fn a_template_whose_class_is_gone_does_not_stop_the_migration() {
    // The month row has a foreign key to the class. A dangling template must be
    // skipped, not crash the upgrade - an app that will not start helps nobody.
    let conn = v18_database_with_sf2_data();
    conn.execute(
        "UPDATE sf2_templates SET active_class_id = 'class-vanished'",
        [],
    )
    .expect("point the template at a class that is not there");
    migrate_db(&conn).expect("the upgrade must still finish");

    assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_templates"),
        0,
        "no month row can exist without its class"
    );
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM events"),
        3,
        "the attendance record is untouched"
    );
}

#[test]
fn a_legacy_template_with_an_empty_report_month_falls_back_to_settings() {
    let conn = v18_database_with_sf2_data();
    conn.execute("UPDATE sf2_templates SET report_month = ''", [])
        .expect("blank the template's own month");
    conn.execute(
        "UPDATE sf2_date_mappings SET date = '2026-10-01' WHERE date = '2026-09-01'",
        [],
    )
    .expect("move one mapping into the settings month");
    migrate_db(&conn).expect("migrate");

    let report_month: String = conn
        .query_row("SELECT report_month FROM sf2_month_templates", [], |row| {
            row.get(0)
        })
        .expect("the settings month is the fallback");
    assert_eq!(report_month, "SEPTEMBER");
    // Only September dates are copied into the September month, so the single
    // October row is deliberately left behind.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_date_mappings"),
        2,
        "only September's day columns are copied into the September month"
    );
}

#[test]
fn the_migration_sql_is_split_into_statements_with_its_markers() {
    let statements = parse_migration_statements(MIGRATE_TO_V22_SQL);

    let named = statements
        .iter()
        .filter_map(|statement| statement.name.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        named,
        vec![
            "template",
            "students",
            "dates",
            "students_expected",
            "students_actual",
            "dates_expected",
            "dates_actual",
        ],
        "every backfill statement the runner looks up by name must be present"
    );

    // The DDL is unmarked, and every `ADD COLUMN` in it is guarded so a replay
    // cannot fail on a duplicate column.
    let ddl = statements
        .iter()
        .filter(|statement| statement.name.is_none())
        .collect::<Vec<_>>();
    assert_eq!(ddl.len(), 4, "three settings columns and the D16 override");
    for statement in ddl {
        let (table, column) = added_column(&statement.sql)
            .unwrap_or_else(|| panic!("expected an ADD COLUMN, got {}", statement.sql));
        assert!(matches!(table.as_str(), "settings" | "sf2_month_templates"));
        assert!(!column.is_empty());
    }
}

#[test]
fn the_add_column_guard_reads_the_table_and_column_out_of_a_statement() {
    assert_eq!(
        added_column("ALTER TABLE settings ADD COLUMN school_start_date TEXT DEFAULT NULL"),
        Some(("settings".to_string(), "school_start_date".to_string()))
    );
    assert_eq!(
        added_column(
            "  alter table sf2_month_templates add column first_school_day_override integer  "
        ),
        Some((
            "sf2_month_templates".to_string(),
            "first_school_day_override".to_string()
        ))
    );
    assert_eq!(added_column("CREATE TABLE IF NOT EXISTS x (id TEXT)"), None);
    assert_eq!(
        added_column("ALTER TABLE settings DROP COLUMN school_start_date"),
        None
    );
    assert_eq!(added_column(""), None);
}

#[test]
fn the_add_column_guard_skips_a_column_that_is_already_there() {
    let pool = test_pool();
    let conn = pool.get().expect("connection");
    assert!(column_exists(&conn, "settings", "school_start_date").expect("probe"));

    // Re-running the DDL over an already-migrated database is a no-op, which is
    // what makes the whole chain replayable.
    execute_migration_ddl(&conn, MIGRATE_TO_V22_SQL).expect("replay the v22 DDL");
    execute_migration_ddl(&conn, MIGRATE_TO_V20_SQL).expect("replay the v20 DDL");

    assert_eq!(schema_version(&conn), CURRENT_SCHEMA_VERSION);
    assert!(column_exists(&conn, "settings", "sf2_split_completed_at").expect("probe"));
    assert!(column_exists(&conn, "students", "sf2_learner_id").expect("probe"));
}

// -- v23: one canonical school-year label --------------------------------
//
// The real install stored `2026 - 2027` - spaces around the dash, the way the
// DepEd form prints it. Every per-month read matches the label by exact
// equality and the "which school year does this class have months for" query
// GLOBs for `NNNN-NNNN`, so that label matched nothing and the whole per-month
// table was silently unreachable. The reports grid fell back to the legacy
// tables for a whole school year and nothing reported an error.
//
// The backfill is Rust, not SQL, and these tests pin the properties that
// decision is for: it must call the *same* canonicaliser the read and write
// paths call, it must never invent a year, and it must be safe to run twice.

/// A migrated database whose school-year labels are the ones the real install
/// has: `2026 - 2027` in every table that stores one.
fn database_with_spaced_school_year() -> rusqlite::Connection {
    let conn = v18_database_with_sf2_data();
    migrate_db(&conn).expect("migrate the fixture to the current version");
    store_the_spaced_label(&conn);
    conn
}

/// Put the pre-v23 state back, so a test can prove the v23 step is what repairs
/// it rather than inheriting a repair from `migrate_db`.
fn store_the_spaced_label(conn: &rusqlite::Connection) {
    conn.execute("UPDATE sf2_templates SET school_year = '2026 - 2027'", [])
        .expect("store the spaced label the way the user typed it");
    conn.execute(
        "UPDATE sf2_month_templates SET school_year = '2026 - 2027'",
        [],
    )
    .expect("store the spaced label on the month rows too");
    conn.execute("UPDATE settings SET school_year = '2026 - 2027'", [])
        .expect("store the spaced label in the app settings too");
}

fn school_year_of(conn: &rusqlite::Connection, sql: &str) -> Vec<String> {
    let mut statement = conn.prepare(sql).expect("prepare");
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query");
    rows.collect::<rusqlite::Result<Vec<String>>>()
        .expect("collect")
}

#[test]
fn v23_rewrites_a_spaced_school_year_into_the_one_form_the_lookups_use() {
    let conn = database_with_spaced_school_year();

    migrate_to_v23(&conn).expect("the v23 step is what repairs the labels");

    assert_eq!(
        school_year_of(&conn, "SELECT school_year FROM sf2_templates"),
        vec!["2026-2027".to_string()],
        "the legacy template row is read by school year too, so it must be repaired"
    );
    let months = school_year_of(&conn, "SELECT school_year FROM sf2_month_templates");
    assert!(!months.is_empty(), "the fixture has month rows");
    assert!(
        months.iter().all(|year| year == "2026-2027"),
        "every month row must carry the canonical label, got {months:?}"
    );
    assert_eq!(
        school_year_of(&conn, "SELECT school_year FROM settings"),
        vec!["2026-2027".to_string()],
        "the app-level copy is kept in step, so the two cannot drift apart again"
    );
}

#[test]
fn after_v23_the_latest_school_year_query_can_actually_find_the_row() {
    // The GLOB, verbatim from month_latest_school_year.sql. Before v23 this
    // returned nothing and the month switch had no school year to resolve to -
    // so every per-month read silently fell back to the legacy tables.
    let conn = database_with_spaced_school_year();

    let before = conn
        .query_row(
            "SELECT school_year FROM sf2_month_templates
             WHERE active_class_id = 'class-1'
               AND school_year GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9]'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();
    assert!(
        before.is_none(),
        "the spaced label must miss the GLOB, or this test proves nothing: {before:?}"
    );

    migrate_to_v23(&conn).expect("v23");

    let found = conn
        .query_row(
            "SELECT school_year FROM sf2_month_templates
             WHERE active_class_id = 'class-1'
               AND school_year GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9]'
             ORDER BY CAST(SUBSTR(school_year, 1, 4) AS INTEGER) DESC
             LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("the latest-school-year query must resolve to a row");
    assert_eq!(found, "2026-2027");
}

#[test]
fn v23_leaves_a_label_that_is_not_a_school_year_alone() {
    let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
    conn.execute_batch(
        "CREATE TABLE sf2_month_templates (id TEXT, school_year TEXT NOT NULL);
         CREATE TABLE sf2_templates (id TEXT, school_year TEXT NOT NULL);
         CREATE TABLE settings (id TEXT PRIMARY KEY, school_year TEXT);
         INSERT INTO sf2_month_templates VALUES ('a', '2026'), ('b', 'S.Y.'), ('c', '');
         INSERT INTO sf2_templates VALUES ('a', '2026'), ('b', 'S.Y.'), ('c', '2026 - 2027');
         INSERT INTO settings VALUES ('app', '2026');",
    )
    .expect("seed labels");

    migrate_to_v23(&conn).expect("v23");

    assert_eq!(
        school_year_of(
            &conn,
            "SELECT school_year FROM sf2_month_templates ORDER BY id"
        ),
        vec!["2026".to_string(), "S.Y.".to_string(), "".to_string()],
        "normalisation is not validation: a label with fewer than two four-digit \
         years is not a school year, and rewriting it would destroy the only copy \
         of what the user entered"
    );
    assert_eq!(
        school_year_of(&conn, "SELECT school_year FROM sf2_templates ORDER BY id"),
        vec![
            "2026".to_string(),
            "S.Y.".to_string(),
            "2026-2027".to_string()
        ],
        "and the one that is a school year is repaired"
    );
    assert_eq!(
        school_year_of(&conn, "SELECT school_year FROM settings"),
        vec!["2026".to_string()],
        "the app-level copy follows the same rule: a half-typed label is kept"
    );
}

#[test]
fn v23_agrees_with_the_read_and_write_normaliser_on_every_spelling() {
    // The property that makes the backfill Rust rather than SQL: there is only
    // one definition of "canonical". If these two ever diverge, the rows on disk
    // and the rows a read looks for are spelled differently again - which is the
    // bug, restated one layer down.
    use crate::sf2::month::first_school_day::normalize_school_year;

    for written in [
        "2026 - 2027",
        "2026  -  2027",
        " 2026-2027 ",
        "2026-2027",
        "SY 2026-2027",
        "2026 / 2027",
        "2026",
        "S.Y.",
        "",
    ] {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        conn.execute_batch(
            "CREATE TABLE sf2_month_templates (id TEXT, school_year TEXT NOT NULL);
             CREATE TABLE sf2_templates (id TEXT, school_year TEXT NOT NULL);
             CREATE TABLE settings (id TEXT PRIMARY KEY, school_year TEXT);",
        )
        .expect("create the tables");
        conn.execute(
            "INSERT INTO sf2_month_templates VALUES ('a', ?1)",
            params![written],
        )
        .expect("seed the month row");
        conn.execute(
            "INSERT INTO sf2_templates VALUES ('a', ?1)",
            params![written],
        )
        .expect("seed the legacy row");
        conn.execute("INSERT INTO settings VALUES ('app', ?1)", params![written])
            .expect("seed the settings row");

        migrate_to_v23(&conn).expect("v23");

        let expected = normalize_school_year(written);
        for table in ["sf2_month_templates", "sf2_templates"] {
            assert_eq!(
                school_year_of(&conn, &format!("SELECT school_year FROM {table}")),
                vec![expected.clone()],
                "{table}: the backfill stored {written:?} and the normaliser would look for \
                 {expected:?}, so one of them is wrong"
            );
        }
    }
}

#[test]
fn v23_is_idempotent() {
    // `migrate_db` can be replayed, and a snapshot-then-migrate that is retried
    // must not keep editing the same rows. The first replay legitimately
    // changes the label; every replay after it must change nothing.
    let conn = database_with_spaced_school_year();
    migrate_to_v23(&conn).expect("first replay");
    let after_first = school_year_of(&conn, "SELECT school_year FROM sf2_month_templates");

    migrate_to_v23(&conn).expect("second replay");
    migrate_to_v23(&conn).expect("third replay");

    assert_eq!(
        school_year_of(&conn, "SELECT school_year FROM sf2_month_templates"),
        after_first,
        "replaying the backfill must change nothing once the label is canonical"
    );
}

#[test]
fn v23_collapses_two_rows_for_one_month_spelled_two_ways() {
    // The duplication this defect produces: the same month imported twice, once
    // with the label the user typed and once with the canonical one. The unique
    // index on (class, school year, month) cannot catch it, because the two
    // strings differ - so both rows exist and neither is reachable.
    let conn = database_with_spaced_school_year();
    // One row is already there with the spaced label. Add the second spelling
    // of the same month by way of the pre-v23 state, so v23 has to collapse them.
    conn.execute(
        "UPDATE sf2_month_templates SET school_year = '2026 - 2027'",
        [],
    )
    .expect("restore the spaced label");
    let month_rows = count(&conn, "SELECT COUNT(*) FROM sf2_month_templates");
    conn.execute(
        "UPDATE sf2_month_templates SET report_month = 'OCTOBER', imported_at = 1
         WHERE report_month = 'SEPTEMBER'",
        [],
    )
    .expect("make room for a second row of the same month");
    conn.execute(
        "UPDATE sf2_month_templates SET report_month = 'SEPTEMBER'
         WHERE report_month = 'OCTOBER'",
        [],
    )
    .expect("restore the month");
    // Now insert a genuine second row for the same month under the other label.
    conn.execute(
        "INSERT INTO sf2_month_templates
         (id, active_class_id, school_year, report_month, report_year, source_path,
          source_hash, first_school_day, imported_at)
         SELECT 'older-duplicate', active_class_id, '2026-2027', report_month, report_year,
                source_path, 'older-hash', first_school_day, imported_at - 1
         FROM sf2_month_templates WHERE id <> 'older-duplicate'
         ORDER BY imported_at LIMIT 1",
        [],
    )
    .expect("insert the older duplicate");
    let before = count(&conn, "SELECT COUNT(*) FROM sf2_month_templates");
    assert_eq!(
        before,
        month_rows + 1,
        "the fixture now holds a duplicate month"
    );

    migrate_to_v23(&conn).expect("v23 collapses the duplicate");

    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM sf2_month_templates"),
        month_rows,
        "one month is one row, whichever way its school year was spelled"
    );
    assert_eq!(
        school_year_of(
            &conn,
            "SELECT DISTINCT school_year FROM sf2_month_templates"
        ),
        vec!["2026-2027".to_string()],
        "and what is left carries the one canonical label"
    );
}

#[test]
fn v23_does_not_change_any_row_count_when_there_is_nothing_to_collapse() {
    // It is a text rewrite, so the assertion the rebuild migrations carry does
    // not apply - but the row counts must still be identical, because a month
    // row that vanished here is a school year the user cannot reach.
    let conn = database_with_spaced_school_year();
    let tables = [
        "sf2_month_templates",
        "sf2_month_date_mappings",
        "sf2_month_student_mappings",
        "sf2_templates",
        "sf2_student_mappings",
        "sf2_date_mappings",
        "events",
        "students",
        "classes",
    ];
    let before: Vec<i64> = tables
        .iter()
        .map(|table| count(&conn, &format!("SELECT COUNT(*) FROM {table}")))
        .collect();

    migrate_to_v23(&conn).expect("run v23 once");

    let after: Vec<i64> = tables
        .iter()
        .map(|table| count(&conn, &format!("SELECT COUNT(*) FROM {table}")))
        .collect();
    assert_eq!(
        before, after,
        "v23 rewrites a label and must not move a row"
    );
}
