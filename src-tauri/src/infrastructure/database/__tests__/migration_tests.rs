use super::*;

/// An in-memory database with no schema at all - the starting point for
/// replaying a migration against a fixture.
fn empty_db() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .expect("disable foreign keys for the fixture");
    conn
}

/// `students` plus an `events` table. `check_clause` is the v10 constraint when
/// `None` (absence is not yet a record); pass `Some("")` for the widened column
/// a database written by a newer build carries.
fn create_events_schema(conn: &rusqlite::Connection, check_clause: Option<&str>) {
    let event_type_column = match check_clause {
        Some("") => "event_type TEXT NOT NULL".to_string(),
        Some(_) => "event_type TEXT NOT NULL CHECK(event_type IN ('in'))".to_string(),
        None => "event_type TEXT NOT NULL".to_string(),
    };

    conn.execute_batch(&format!(
        r#"
        CREATE TABLE students (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            card_serial TEXT UNIQUE,
            class_id TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE events (
            id TEXT PRIMARY KEY NOT NULL,
            student_id TEXT NOT NULL,
            class_id TEXT,
            {event_type_column},
            timestamp INTEGER NOT NULL,
            note TEXT,
            FOREIGN KEY (student_id) REFERENCES students(id) ON DELETE CASCADE
        );
        INSERT INTO students (id, name, card_serial, class_id, created_at)
        VALUES ('student-1', 'Juan Dela Cruz', NULL, 'class-1', 1);
        "#
    ))
    .expect("create the fixture schema");
}

fn insert_event(conn: &rusqlite::Connection, id: &str, event_type: &str) {
    conn.execute(
        "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note)
         VALUES (?1, 'student-1', 'class-1', ?2, 1783000000, NULL)",
        params![id, event_type],
    )
    .expect("insert event");
}

/// The three columns the v14 migration adds. v17 copies them, so a fixture
/// crossing straight into v17 needs them.
fn add_v14_event_columns(conn: &rusqlite::Connection) {
    conn.execute_batch(
        "ALTER TABLE events ADD COLUMN session_key TEXT;
         ALTER TABLE events ADD COLUMN override_reason TEXT;
         ALTER TABLE events ADD COLUMN updated_at INTEGER;",
    )
    .expect("add the v14 event columns");
}

fn event_types(conn: &rusqlite::Connection) -> Vec<String> {
    let mut statement = conn
        .prepare("SELECT event_type FROM events ORDER BY id")
        .expect("prepare events query");
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query events");
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .expect("collect event types")
}

fn count_events(conn: &rusqlite::Connection) -> i64 {
    row_count(conn, "events").expect("count events")
}

#[test]
fn v11_keeps_absence_rows_written_by_a_newer_build() {
    // The landmine: a database crossing v10 -> v11 that already carries
    // 'absent' rows. Absence is only a first-class event type from v17, so
    // v11's rebuild used to DELETE those rows outright - the attendance marks
    // were destroyed by the upgrade itself.
    let conn = empty_db();
    create_events_schema(&conn, Some(""));
    insert_event(&conn, "event-a", "in");
    insert_event(&conn, "event-b", "absent");
    // An event type no schema version knows about: the guarded delete is still
    // allowed to drop this one.
    insert_event(&conn, "event-c", "half-day");

    let before = count_events(&conn);
    migrate_to_v11(&conn).expect("v11 must succeed");
    let after = count_events(&conn);

    assert_eq!(before, 3, "fixture should hold three events");
    assert_eq!(after, 2, "only the unknown event type may be dropped");
    assert_eq!(
        event_types(&conn),
        vec!["in".to_string(), "absent".to_string()],
        "the absence must survive the v10 -> v11 crossing"
    );
}

#[test]
fn v11_accepts_the_widened_event_type_check() {
    // v17 rebuilds this table again, but a database can legitimately sit at
    // v11 with absence rows in it, so the v11 CHECK itself has to admit them.
    let conn = empty_db();
    create_events_schema(&conn, Some(""));
    insert_event(&conn, "event-a", "absent");
    migrate_to_v11(&conn).expect("v11 must succeed");

    let insert_absent = conn.execute(
        "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note)
         VALUES ('event-b', 'student-1', 'class-1', 'absent', 1783000000, NULL)",
        [],
    );
    assert!(
        insert_absent.is_ok(),
        "the rebuilt v11 events table must accept an 'absent' row"
    );
}

#[test]
fn v11_preserves_the_student_row_count() {
    let conn = empty_db();
    create_events_schema(&conn, None);
    conn.execute(
        "INSERT INTO students (id, name, card_serial, class_id, created_at)
         VALUES ('student-2', 'Maria Santos', NULL, 'class-1', 1)",
        [],
    )
    .expect("insert a second student");
    insert_event(&conn, "event-a", "in");

    let students_before = row_count(&conn, "students").expect("count students");
    let events_before = count_events(&conn);
    migrate_to_v11(&conn).expect("v11 must succeed");

    assert_eq!(
        row_count(&conn, "students").expect("count students"),
        students_before
    );
    assert_eq!(count_events(&conn), events_before);
}

#[test]
fn v17_keeps_every_event_row() {
    let conn = empty_db();
    create_events_schema(&conn, None);
    add_v14_event_columns(&conn);
    insert_event(&conn, "event-a", "in");
    insert_event(&conn, "event-b", "in");
    let before = count_events(&conn);

    migrate_to_v17(&conn).expect("v17 must succeed");

    assert_eq!(count_events(&conn), before);
}

#[test]
fn v11_then_v17_carries_an_absence_across_both_rebuilds() {
    // The realistic order for a database that already recorded absences: it is
    // replayed through both rebuilds. Neither may drop the row.
    let conn = empty_db();
    create_events_schema(&conn, Some(""));
    insert_event(&conn, "event-a", "in");
    insert_event(&conn, "event-b", "absent");
    let before = count_events(&conn);

    migrate_to_v11(&conn).expect("v11 must succeed");
    // v11 rebuilds `events` without the v14 columns, and v14 is what adds them,
    // so they arrive between the two rebuilds - never before v11.
    add_v14_event_columns(&conn);
    migrate_to_v17(&conn).expect("v17 must succeed");

    assert_eq!(count_events(&conn), before);
    assert_eq!(
        event_types(&conn),
        vec!["in".to_string(), "absent".to_string()]
    );
}

#[test]
fn assert_row_count_preserved_fails_loudly_on_a_lost_row() {
    let error = assert_row_count_preserved("events", 2, 1, "v17")
        .expect_err("a lost row must not pass silently");

    let message = error.to_string();
    assert!(
        message.contains("v17"),
        "message names the migration: {message}"
    );
    assert!(
        message.contains("events"),
        "message names the table: {message}"
    );
    assert!(
        message.contains('2') && message.contains('1'),
        "message reports both counts: {message}"
    );
}

#[test]
fn assert_row_count_preserved_accepts_a_matching_count() {
    assert_row_count_preserved("events", 1, 1, "v17").expect("an unchanged count must pass");
}

// -- v11 must not drop a column a newer build had already added -------------

#[test]
fn v11_refuses_to_rebuild_over_a_table_a_newer_build_extended() {
    // A database whose `user_version` was lost - a partially applied upgrade, a
    // restored `.db` from a newer build, a hand-edited stamp - while its tables
    // already carry the v13, v14 and v20 columns. `migrate_to_v11.sql` rebuilds
    // both tables from the v11 shape, so replaying it here used to drop
    // `session_key`, `override_reason`, `updated_at`, `gender` and
    // `sf2_learner_id` with no error at all. The exception audit trail lives in
    // the first three.
    let conn = empty_db();
    create_events_schema(&conn, Some(""));
    conn.execute_batch(
        "ALTER TABLE events ADD COLUMN session_key TEXT;
         ALTER TABLE events ADD COLUMN override_reason TEXT;
         ALTER TABLE events ADD COLUMN updated_at INTEGER;
         ALTER TABLE students ADD COLUMN gender TEXT;
         ALTER TABLE students ADD COLUMN sf2_learner_id TEXT;",
    )
    .expect("extend the fixture to a v14+/v20 shape");
    insert_event(&conn, "event-a", "absent");

    let error = migrate_to_v11(&conn).expect_err(
        "a rebuild that drops the exception audit columns must not be allowed to finish",
    );
    let message = error.to_string();
    for column in [
        "session_key",
        "override_reason",
        "updated_at",
        "gender",
        "sf2_learner_id",
    ] {
        assert!(
            message.contains(column),
            "the refusal must name `{column}` so the report says what was at risk: {message}"
        );
    }
}

#[test]
fn the_column_guard_is_silent_when_nothing_is_lost() {
    let before: BTreeSet<String> = ["id", "name"].iter().map(|c| (*c).to_string()).collect();
    let same = before.clone();
    assert_columns_preserved(&[("students", &before, &same)], "v11")
        .expect("an unchanged column list must pass");

    // Columns the rebuild *adds* are fine - a migration is allowed to widen.
    let wider: BTreeSet<String> = ["id", "name", "gender"]
        .iter()
        .map(|c| (*c).to_string())
        .collect();
    assert_columns_preserved(&[("students", &before, &wider)], "v11")
        .expect("adding a column is fine");
}

#[test]
fn the_column_guard_names_the_lost_column() {
    let before: BTreeSet<String> = ["id", "session_key"]
        .iter()
        .map(|c| (*c).to_string())
        .collect();
    let after: BTreeSet<String> = ["id"].iter().map(|c| (*c).to_string()).collect();

    let error = assert_columns_preserved(&[("events", &before, &after)], "v11")
        .expect_err("a dropped column must fail the migration");
    assert!(error.to_string().contains("session_key"), "{}", error);
}
