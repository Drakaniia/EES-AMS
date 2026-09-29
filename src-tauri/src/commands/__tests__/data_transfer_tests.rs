use super::*;
use crate::commands::common::collect_export_data;
use crate::infrastructure::database::init_db;

const CLASS_ID: &str = "class-1";
/// Student and event ids are parsed as UUIDs when they are read back, so the
/// fixtures have to be real UUIDs.
const STUDENT_ID: &str = "11111111-1111-4111-8111-111111111111";
const EVENT_ID: &str = "22222222-2222-4222-8222-222222222222";
/// 2026-07-01 08:00 local, expressed as a UTC timestamp for the fixture.
const ABSENCE_TIMESTAMP: i64 = 1_783_000_000;
const SESSION_KEY: &str = "2026-07-01|class-1|day";

/// A migrated, empty database in a throwaway directory.
///
/// The directory handle is leaked on purpose: the pool keeps the `.db` file
/// open for the whole test and Windows will not delete a file that is still
/// open.
fn test_pool() -> Pool<SqliteConnectionManager> {
    let dir = tempfile::tempdir().expect("temp dir for the test database");
    let pool = init_db(dir.path().join("attendance.db")).expect("migrate test database");
    std::mem::forget(dir);
    pool
}

/// A database holding one learner marked absent - the X mark.
fn pool_with_an_absence() -> Pool<SqliteConnectionManager> {
    let pool = test_pool();
    {
        let conn = pool.get().expect("connection");
        conn.execute(
            "INSERT INTO classes (id, name, room, day_start, day_end, late_after, sessions, days, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                CLASS_ID,
                "Grade 3 - A",
                "N/A",
                "08:30",
                "15:30",
                "08:45",
                "[]",
                "[1,2,3,4,5]",
                1_i64
            ],
        )
        .expect("insert class");
        conn.execute(
            "INSERT INTO students (id, name, class_id, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![STUDENT_ID, "Juan Dela Cruz", CLASS_ID, 1_i64],
        )
        .expect("insert student");
        conn.execute(
            "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                EVENT_ID,
                STUDENT_ID,
                CLASS_ID,
                "absent",
                ABSENCE_TIMESTAMP,
                Option::<String>::None,
                SESSION_KEY,
                Option::<String>::None,
                Option::<i64>::None,
            ],
        )
        .expect("insert absent event");
    }
    pool
}

/// Mirror of the class and student upserts `import_all` performs. Only the
/// event loop is shared with production code, because that is the loop whose
/// hardcoded `"in"` used to turn every absence into a present.
fn import_classes_and_students(
    transaction: &rusqlite::Transaction<'_>,
    data: &ExportData,
) -> std::result::Result<(), String> {
    for class in &data.classes {
        let sessions_json = serde_json::to_string(&class.sessions).map_err(|e| e.to_string())?;
        let days_json = serde_json::to_string(&class.days).map_err(|e| e.to_string())?;
        transaction
            .execute(
                "INSERT INTO classes (id, name, room, day_start, day_end, late_after, sessions, days, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    room = excluded.room,
                    day_start = excluded.day_start,
                    day_end = excluded.day_end,
                    late_after = excluded.late_after,
                    sessions = excluded.sessions,
                    days = excluded.days,
                    created_at = excluded.created_at",
                params![
                    class.id,
                    class.name,
                    class.room,
                    class.day_start,
                    class.day_end,
                    class.late_after,
                    sessions_json,
                    days_json,
                    class.created_at.timestamp(),
                ],
            )
            .map_err(|e| e.to_string())?;
    }

    for student in &data.students {
        transaction
            .execute(
                "INSERT INTO students (id, name, gender, card_serial, class_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    gender = excluded.gender,
                    card_serial = excluded.card_serial,
                    class_id = excluded.class_id,
                    created_at = excluded.created_at",
                params![
                    student.id.0.to_string(),
                    student.name,
                    student.gender.map(StudentGender::as_db_value),
                    student.card_serial,
                    student.class_id,
                    student.created_at.timestamp(),
                ],
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[test]
fn export_includes_absent_events_with_their_type() {
    let pool = pool_with_an_absence();

    let export = collect_export_data(&pool).expect("collect export data");

    let absence = export
        .events
        .iter()
        .find(|event| event.id.0.to_string() == EVENT_ID)
        .expect("the absent event must be exported, not filtered out");
    assert_eq!(absence.event_type, AttendanceType::Absent);
    assert_eq!(absence.timestamp.timestamp(), ABSENCE_TIMESTAMP);
    assert_eq!(absence.session_key.as_deref(), Some(SESSION_KEY));
    assert_eq!(absence.class_id.as_deref(), Some(CLASS_ID));
}

#[test]
fn json_round_trip_preserves_the_absence() {
    // Export JSON -> wipe all -> Import JSON must not turn absences into
    // presents. That path is one click away in Settings, and it is the only
    // place the attendance marks live besides the workbook mirror.
    let source_pool = pool_with_an_absence();
    let export = collect_export_data(&source_pool).expect("collect export data");

    // Go through JSON, exactly as the saved backup file does.
    let json = serde_json::to_string(&export).expect("serialize export data");
    let payload: ExportData = serde_json::from_str(&json).expect("deserialize export data");

    let target_pool = test_pool();
    {
        let mut conn = target_pool.get().expect("connection");
        let transaction = conn.transaction().expect("begin transaction");
        import_classes_and_students(&transaction, &payload).expect("import classes and students");
        for event in &payload.events {
            insert_imported_event(&transaction, event).expect("import event");
        }
        transaction.commit().expect("commit import");
    }

    let conn = target_pool.get().expect("connection");
    let (event_type, timestamp, class_id, session_key): (
        String,
        i64,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT event_type, timestamp, class_id, session_key FROM events WHERE id = ?1",
            params![EVENT_ID],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("the absent event must survive the import");

    assert_eq!(event_type, "absent");
    assert_eq!(timestamp, ABSENCE_TIMESTAMP);
    assert_eq!(class_id.as_deref(), Some(CLASS_ID));
    assert_eq!(session_key.as_deref(), Some(SESSION_KEY));
}
