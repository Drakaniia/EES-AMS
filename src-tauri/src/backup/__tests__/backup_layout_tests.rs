use crate::backup::models::BackupWorkbookPreview as PreviewWorkbook;
use crate::backup::scheduling::ensure_daily_backup_at;
use crate::backup::{
    backup_ops, backup_service, file_ops, fingerprint, manifest, models::BackupKind, workbooks,
};
use crate::infrastructure::database::{init_db, DbPool};
use chrono::{Duration, Local};
use std::{
    fs,
    path::{Path, PathBuf},
};

// ── Harness ───────────────────────────────────────────────────────────

/// The schema version the previous build wrote into a flat `.db` backup.
const LEGACY_SCHEMA_VERSION: i32 = 18;

/// A throwaway app data directory holding a real migrated `attendance.db` and a
/// fake `sf2-workbooks/` tree.
struct TempApp {
    dir: tempfile::TempDir,
    pool: DbPool,
}

impl TempApp {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp app dir");
        let pool = init_db(dir.path().join("attendance.db")).expect("init test database");
        let app = Self { dir, pool };
        app.seed();
        app
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// One class, two students, three events, one of them an absence.
    ///
    /// Every NOT NULL column is supplied and the row counts are asserted: an
    /// `INSERT OR IGNORE` that silently drops a row would otherwise leave the
    /// fixtures quietly emptier than the tests assume.
    fn seed(&self) {
        let conn = self.pool.get().expect("connection");
        // `classes` and `students` both have NOT NULL columns with no default,
        // and `events.student_id` is a foreign key into `students`.
        conn.execute_batch(
            "INSERT OR IGNORE INTO classes (id, name, day_start, day_end, late_after, created_at)
                VALUES ('c1', 'Grade 1 - A', '08:30', '15:30', '08:45', 1);
             INSERT OR IGNORE INTO students (id, name, class_id, created_at)
                VALUES ('s1', 'Ana', 'c1', 1), ('s2', 'Bao', 'c1', 1);
             INSERT OR IGNORE INTO events (id, student_id, event_type, timestamp)
                VALUES ('e1', 's1', 'in', 1000), ('e2', 's2', 'in', 1000), ('e3', 's1', 'absent', 2000);",
        )
        .expect("seed data");
        for (table, expected) in [("classes", 1), ("students", 2), ("events", 3)] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap_or_else(|error| panic!("count {table}: {error}"));
            assert_eq!(count, expected, "the seed must actually land in {table}");
        }
    }

    /// Create a fake live workbook. It is not a real `.xls`, so counting its X
    /// marks fails and degrades to 0 — which is the path a machine without
    /// Excel takes, and worth exercising.
    fn write_live_workbook(&self, name: &str) -> PathBuf {
        let dir = workbooks::workbook_source_dir(self.path());
        fs::create_dir_all(&dir).expect("create sf2-workbooks");
        let path = dir.join(name);
        fs::write(&path, b"not-a-real-xls").expect("write fake workbook");
        path
    }

    fn write_live_legacy_workbook(&self) -> PathBuf {
        let dir = workbooks::workbook_source_dir(self.path()).join("_legacy");
        fs::create_dir_all(&dir).expect("create _legacy");
        let path = dir.join("SF2-OLD.xls");
        fs::write(&path, b"legacy").expect("write legacy workbook");
        path
    }

    /// A legacy flat `.db` backup, exactly as the previous build wrote it: one
    /// database file dropped straight into `backups/`, with no manifest and no
    /// `workbooks/` folder beside it.
    ///
    /// The schema is the live, fully migrated one on purpose. What these tests
    /// are about is the *shape* of the backup — bare file, no manifest, no
    /// workbooks — and a hand-rolled fixture with invented columns would rot the
    /// moment a migration added one.
    fn write_legacy_flat_backup(&self, name: &str) -> PathBuf {
        let dir = file_ops::backup_dir(self.path());
        fs::create_dir_all(&dir).expect("create backups dir");
        let path = dir.join(name);
        let conn = self.pool.get().expect("connection");
        conn.backup(
            rusqlite::DatabaseName::Main,
            &path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .expect("copy database into the legacy file");
        path
    }

    /// A flat `.db` holding an *old* schema version, as a pre-migration snapshot
    /// or a backup from an earlier build would. Preview only — restoring it would
    /// run the migration chain, which is not what these fixtures are for.
    fn write_legacy_schema_backup(&self, name: &str) -> PathBuf {
        let dir = file_ops::backup_dir(self.path());
        fs::create_dir_all(&dir).expect("create backups dir");
        let path = dir.join(name);
        let conn = rusqlite::Connection::open(&path).expect("open legacy db");
        conn.execute_batch(&format!(
            "CREATE TABLE classes (id TEXT PRIMARY KEY, name TEXT);
             CREATE TABLE students (id TEXT PRIMARY KEY, name TEXT);
             CREATE TABLE events (id TEXT PRIMARY KEY, student_id TEXT, event_type TEXT, timestamp INTEGER);
             CREATE TABLE settings (id TEXT PRIMARY KEY, day_start TEXT, day_end TEXT, late_after TEXT, quarter TEXT);
             INSERT INTO students VALUES ('s1', 'Ana'), ('s2', 'Bao');
             INSERT INTO events VALUES ('e3', 's1', 'absent', 2000);
             PRAGMA user_version = {LEGACY_SCHEMA_VERSION};"
        ))
        .expect("populate legacy db");
        path
    }
}

// ── B1: folder per backup ─────────────────────────────────────────────

#[test]
fn create_backup_writes_a_folder_with_a_database_a_manifest_and_workbooks() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");

    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::Manual, Local::now())
            .expect("create backup");

    assert!(
        summary
            .path
            .ends_with(&format!("-{}.db", summary.created_at))
            || !summary.path.ends_with(".db")
    );
    assert!(
        Path::new(&summary.path).is_dir(),
        "a backup is a folder, not a file"
    );
    assert!(manifest::backup_db_path(Path::new(&summary.path)).is_file());
    assert!(manifest::manifest_path(Path::new(&summary.path)).is_file());
    assert!(manifest::workbooks_path(Path::new(&summary.path)).is_dir());
    assert_eq!(summary.workbook_count, 1);
    assert!(summary.includes_database);
    // The finished folder must be the final name, never the `.tmp` sibling it
    // was assembled under. Assert on the folder's own name: the temp directory
    // this test runs in is itself called `.tmpXXXX`.
    let folder_name = Path::new(&summary.path)
        .file_name()
        .and_then(|name| name.to_str())
        .expect("folder name")
        .to_string();
    assert!(
        !folder_name.ends_with(".tmp"),
        "a half-assembled folder was listed as a backup: {folder_name}"
    );
}

#[test]
fn manifest_records_the_schema_version_the_counts_and_every_workbook() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    app.write_live_workbook("SF2-OCTOBER-2026.xls");

    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::Manual, Local::now())
            .expect("create backup");
    let folder = PathBuf::from(&summary.path);

    let written = manifest::read_manifest(&folder)
        .expect("read manifest")
        .expect("manifest exists");
    assert_eq!(
        written.schema_version,
        crate::infrastructure::database::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(written.kind, "manual");
    assert_eq!(written.counts.students, 2);
    assert_eq!(written.counts.events, 3);
    assert_eq!(written.counts.absent, 1);
    // A backup taken from a database older than v19 must still be readable, so
    // this is 0 rather than an error.
    assert_eq!(written.counts.sf2_month_templates, 0);
    assert_eq!(written.workbooks.len(), 2);
    for workbook in &written.workbooks {
        assert!(workbook.path.starts_with("workbooks/"), "{workbook:?}");
        assert!(folder.join(&workbook.path).is_file());
        assert!(workbook.bytes > 0);
    }
}

#[test]
fn manifest_survives_a_json_round_trip_with_camel_case_keys() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::Manual, Local::now())
            .expect("create backup");

    let raw = fs::read_to_string(manifest::manifest_path(Path::new(&summary.path)))
        .expect("read manifest json");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("parse manifest json");

    assert!(value.get("schemaVersion").is_some(), "{raw}");
    assert!(value.get("createdAt").is_some(), "{raw}");
    assert!(
        value
            .get("counts")
            .unwrap()
            .get("sf2MonthTemplates")
            .is_some(),
        "{raw}"
    );
    let first = &value["workbooks"][0];
    assert!(first.get("path").is_some(), "{raw}");
    assert!(first.get("bytes").is_some(), "{raw}");
    assert!(first.get("xCount").is_some(), "{raw}");
}

#[test]
fn the_snapshot_copies_a_nested_workbook_tree_verbatim() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    app.write_live_legacy_workbook();
    let destination = app.path().join("snapshot-target");
    fs::create_dir_all(&destination).expect("create destination");

    let snapshot = workbooks::snapshot_workbooks_into(app.path(), &destination).expect("snapshot");

    assert!(!snapshot.source_missing);
    assert_eq!(snapshot.entries.len(), 2);
    let relative = snapshot
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    assert!(relative.contains(&"workbooks/SF2-SEPTEMBER-2026.xls".to_string()));
    assert!(
        relative.contains(&"workbooks/_legacy/SF2-OLD.xls".to_string()),
        "a nested subfolder must be copied whole, not just the top level: {relative:?}"
    );
    for entry in &snapshot.entries {
        assert!(destination.join(&entry.path).is_file());
    }
}

#[test]
fn snapshotting_with_no_live_workbooks_still_produces_an_empty_workbooks_folder() {
    let app = TempApp::new();
    let destination = app.path().join("empty-snapshot");
    fs::create_dir_all(&destination).expect("create destination");

    let snapshot = workbooks::snapshot_workbooks_into(app.path(), &destination).expect("snapshot");

    assert!(snapshot.source_missing);
    assert!(snapshot.entries.is_empty());
    assert!(manifest::workbooks_path(&destination).is_dir());
}

// ── B2: legacy flat `.db` backups keep working ────────────────────────

#[test]
fn legacy_flat_backups_are_listed_alongside_folder_backups() {
    let app = TempApp::new();
    app.write_legacy_flat_backup("attendance-manual-20200101_010101.db");
    app.write_legacy_flat_backup("attendance-auto-20200102_010101.db");
    backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::Manual, Local::now())
        .expect("create folder backup");

    let listed = backup_ops::list_backups(app.path()).expect("list backups");

    assert_eq!(listed.len(), 3, "{listed:#?}");
    assert!(listed.iter().all(|backup| backup.includes_database));
    assert!(listed.iter().any(|b| b.kind == BackupKind::Auto));
    assert!(listed.iter().any(|b| b.kind == BackupKind::Manual));
    // The folder backup carries workbooks; the legacy files carry none.
    assert!(listed.iter().any(|b| b.workbook_count == 0));
}

#[test]
fn list_backups_ignores_unrelated_entries() {
    let app = TempApp::new();
    let dir = file_ops::backup_dir(app.path());
    fs::create_dir_all(&dir).expect("create backups dir");
    fs::write(dir.join("attendance-manual-20200101_010101.db.tmp"), b"x").expect("tmp file");
    fs::write(dir.join("somebody-elses.db"), b"x").expect("foreign file");
    fs::create_dir_all(dir.join("attendance-manual-20200101_010101.tmp")).expect("tmp dir");
    fs::create_dir_all(dir.join("not-a-backup")).expect("foreign dir");

    assert!(backup_ops::list_backups(app.path())
        .expect("list")
        .is_empty());
}

#[test]
fn preview_accepts_a_legacy_flat_database_file() {
    let app = TempApp::new();
    let legacy = app.write_legacy_flat_backup("attendance-manual-20200101_010101.db");

    let preview = backup_ops::preview_backup(&legacy).expect("preview legacy backup");

    assert!(preview.includes_database);
    // A legacy `.db` is a real database: the preview must report its real
    // counts, not a row of zeros.
    assert_eq!(preview.absent_count, 1);
    assert_eq!(preview.event_count, 3);
    assert_eq!(preview.student_count, 2);
    assert_eq!(preview.class_count, 1);
    assert_eq!(
        preview.schema_version,
        crate::infrastructure::database::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(preview.database_path, legacy.to_string_lossy());
    assert!(preview.workbooks.is_empty());
    assert!(
        preview
            .warnings
            .iter()
            .any(|warning| warning.contains("predates workbook backup")),
        "{:?}",
        preview.warnings
    );
    assert!(
        preview
            .warnings
            .iter()
            .any(|warning| warning.contains("no SF2 workbooks")),
        "{:?}",
        preview.warnings
    );
}

#[test]
fn preview_of_a_legacy_flat_backup_from_an_older_schema_says_it_will_be_migrated() {
    let app = TempApp::new();
    let legacy = app.write_legacy_schema_backup("attendance-manual-20180101_010101.db");

    let preview = backup_ops::preview_backup(&legacy).expect("preview legacy backup");

    assert_eq!(preview.schema_version, LEGACY_SCHEMA_VERSION);
    assert_eq!(preview.absent_count, 1);
    assert!(
        preview
            .warnings
            .iter()
            .any(|warning| warning.contains("will be migrated from schema version")),
        "{:?}",
        preview.warnings
    );
}

#[test]
fn preview_of_a_legacy_flat_backup_raises_no_xcount_mismatch_warning() {
    // The brief's B2-1 trap: a backup with no manifest must not be modelled as
    // "one workbook holding 0 X marks", because 0 vs 0 absences would look like a
    // workbook that is behind its database and warn spuriously.
    let app = TempApp::new();
    let legacy = app.write_legacy_flat_backup("attendance-manual-20200101_010101.db");

    let preview = backup_ops::preview_backup(&legacy).expect("preview legacy backup");

    assert!(preview.workbooks.is_empty());
    assert!(
        backup_ops::workbook_absence_warning(&preview.workbooks, preview.absent_count).is_none()
    );
    assert!(
        !preview.warnings.iter().any(|w| w.contains("X mark")),
        "a legacy backup must not warn about X-mark counts it never recorded: {:?}",
        preview.warnings
    );
}

#[test]
fn preview_rejects_a_legacy_flat_file_that_is_not_an_ees_ams_database() {
    let app = TempApp::new();
    let dir = file_ops::backup_dir(app.path());
    fs::create_dir_all(&dir).expect("create backups dir");
    let path = dir.join("attendance-manual-20200101_010101.db");
    rusqlite::Connection::open(&path)
        .expect("open")
        .execute_batch("CREATE TABLE unrelated (id TEXT); PRAGMA user_version = 18;")
        .expect("populate");

    let error = backup_ops::preview_backup(&path).expect_err("must reject");

    assert!(
        error.to_string().contains("not an EES-AMS database"),
        "{error}"
    );
}

#[test]
fn preview_rejects_a_legacy_flat_file_from_a_newer_schema() {
    let app = TempApp::new();
    let dir = file_ops::backup_dir(app.path());
    fs::create_dir_all(&dir).expect("create backups dir");
    let path = dir.join("attendance-manual-20200101_010101.db");
    rusqlite::Connection::open(&path)
        .expect("open")
        .execute_batch(&format!(
            "CREATE TABLE classes (id TEXT PRIMARY KEY, name TEXT);
             CREATE TABLE students (id TEXT PRIMARY KEY, name TEXT);
             CREATE TABLE events (id TEXT PRIMARY KEY, student_id TEXT, event_type TEXT, timestamp INTEGER);
             CREATE TABLE settings (id TEXT PRIMARY KEY);
             PRAGMA user_version = {};",
            crate::infrastructure::database::CURRENT_SCHEMA_VERSION + 1
        ))
        .expect("populate");

    let error = backup_ops::preview_backup(&path).expect_err("must reject");

    assert!(
        error.to_string().contains("newer than this app supports"),
        "{error}"
    );
}

#[test]
fn preview_of_a_folder_resolves_the_database_inside_it() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::Manual, Local::now())
            .expect("create backup");
    let folder = PathBuf::from(&summary.path);

    let preview = backup_ops::preview_backup(&folder).expect("preview folder backup");

    assert_eq!(preview.source_path, folder.to_string_lossy());
    assert_eq!(
        preview.database_path,
        manifest::backup_db_path(&folder).to_string_lossy()
    );
    assert_eq!(preview.absent_count, 1);
    assert_eq!(preview.workbooks.len(), 1);
    assert_eq!(preview.workbooks[0].file_name, "SF2-SEPTEMBER-2026.xls");
    assert!(preview
        .workbooks
        .iter()
        .all(|workbook| workbook.relative_path.starts_with("workbooks/")));
}

#[test]
fn preview_rejects_a_folder_with_no_database() {
    let app = TempApp::new();
    let folder = app.path().join("not-a-backup");
    fs::create_dir_all(&folder).expect("create folder");

    let error = backup_ops::preview_backup(&folder).expect_err("must reject");

    assert!(
        error.to_string().contains("attendance.db"),
        "the error must name what is missing: {error}"
    );
}

// ── B2 critical: the xCount / absence-count warning ────────────────────

fn preview_workbook(x_count: i64) -> PreviewWorkbook {
    PreviewWorkbook {
        file_name: "SF2-SEPTEMBER-2026.xls".to_string(),
        relative_path: "workbooks/SF2-SEPTEMBER-2026.xls".to_string(),
        bytes: 1024,
        x_count,
    }
}

#[test]
fn a_workbook_ahead_of_its_database_raises_a_pre_restore_warning() {
    let warning = backup_ops::workbook_absence_warning(&[preview_workbook(12)], 4)
        .expect("must warn when the workbook holds more absences than the database");

    assert!(warning.contains("12"), "{warning}");
    assert!(warning.contains("SF2-SEPTEMBER-2026.xls"), "{warning}");
    assert!(warning.contains("4"), "{warning}");
}

#[test]
fn no_warning_when_the_database_is_not_behind_the_workbook() {
    assert!(backup_ops::workbook_absence_warning(&[preview_workbook(12)], 12).is_none());
    assert!(backup_ops::workbook_absence_warning(&[preview_workbook(12)], 40).is_none());
    assert!(backup_ops::workbook_absence_warning(&[preview_workbook(0)], 0).is_none());
    assert!(backup_ops::workbook_absence_warning(&[], 3).is_none());
}

#[test]
fn the_warning_is_raised_by_preview_before_any_restore_runs() {
    let app = TempApp::new();
    // A folder backup whose manifest claims 12 X marks while its database holds
    // 1 absence.
    let folder = app.path().join("attendance-manual-20200101_010101");
    fs::create_dir_all(&folder).expect("create folder");
    let database = app.pool.get().expect("conn");
    database
        .backup(
            rusqlite::DatabaseName::Main,
            manifest::backup_db_path(&folder),
            None::<fn(rusqlite::backup::Progress)>,
        )
        .expect("copy database");
    manifest::write_manifest(
        &folder,
        &manifest::BackupManifest {
            schema_version: crate::infrastructure::database::CURRENT_SCHEMA_VERSION,
            created_at: "2020-01-01T01:01:01+08:00".to_string(),
            kind: "manual".to_string(),
            counts: manifest::ManifestCounts::default(),
            workbooks: vec![manifest::ManifestWorkbook {
                path: "workbooks/SF2-SEPTEMBER-2026.xls".to_string(),
                bytes: 2048,
                x_count: 12,
            }],
        },
    )
    .expect("write manifest");
    fs::create_dir_all(manifest::workbooks_path(&folder)).expect("workbooks dir");
    fs::write(
        manifest::workbooks_path(&folder).join("SF2-SEPTEMBER-2026.xls"),
        b"x",
    )
    .expect("workbook");

    let preview = backup_ops::preview_backup(&folder).expect("preview");

    assert_eq!(preview.absent_count, 1);
    assert_eq!(preview.workbooks[0].x_count, 12);
    assert!(
        preview
            .warnings
            .iter()
            .any(|warning| warning.contains("12 X mark")),
        "{:?}",
        preview.warnings
    );
}

// ── B2: restore ───────────────────────────────────────────────────────

#[test]
fn restoring_a_folder_backup_writes_the_workbooks_back() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::Manual, Local::now())
            .expect("create backup");

    // Change the live workbook after the backup, then restore.
    let live = workbooks::workbook_source_dir(app.path()).join("SF2-SEPTEMBER-2026.xls");
    fs::write(&live, b"changed-after-backup").expect("mutate live workbook");
    fs::write(
        workbooks::workbook_source_dir(app.path()).join("SF2-DECEMBER-2026.xls"),
        b"added-after-backup",
    )
    .expect("add later workbook");

    let result = backup_service::restore_backup(app.pool(), app.path(), Path::new(&summary.path))
        .expect("restore");

    assert!(result.workbooks_restored, "{result:#?}");
    assert_eq!(
        fs::read(&live).expect("read restored workbook"),
        b"not-a-real-xls"
    );
    assert!(result
        .pre_restore_backup_path
        .contains("attendance-pre-restore"));
    assert!(Path::new(&result.pre_restore_backup_path).is_dir());
}

#[test]
fn the_pre_restore_safety_backup_also_holds_the_workbooks() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::Manual, Local::now())
            .expect("create backup");

    let result = backup_service::restore_backup(app.pool(), app.path(), Path::new(&summary.path))
        .expect("restore");

    let safety = PathBuf::from(&result.pre_restore_backup_path);
    assert!(manifest::backup_db_path(&safety).is_file());
    assert!(manifest::workbooks_path(&safety).is_dir());
    assert_eq!(
        fs::read_dir(manifest::workbooks_path(&safety))
            .expect("read")
            .count(),
        1
    );
}

#[test]
fn restoring_a_legacy_flat_backup_leaves_the_live_workbooks_alone() {
    let app = TempApp::new();
    let live = app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    let legacy = app.write_legacy_flat_backup("attendance-manual-20200101_010101.db");

    let result = backup_service::restore_backup(app.pool(), app.path(), &legacy).expect("restore");

    assert!(!result.workbooks_restored);
    assert_eq!(result.restored_path, legacy.to_string_lossy());
    assert_eq!(
        result
            .warnings
            .iter()
            .filter(|warning| warning.contains("no SF2 workbooks"))
            .count(),
        1,
        "the user is told once, not twice: {result:#?}"
    );
    assert_eq!(fs::read(&live).expect("read workbook"), b"not-a-real-xls");
}

#[test]
fn restoring_a_legacy_flat_backup_restores_the_database_without_warning_about_x_marks() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    let legacy = app.write_legacy_flat_backup("attendance-manual-20200101_010101.db");

    // Diverge the live database from the legacy backup after the backup was
    // taken, so a restore that silently did nothing would be caught.
    let conn = app.pool.get().expect("connection");
    conn.execute(
        "INSERT INTO events (id, student_id, event_type, timestamp) VALUES ('e9', 's1', 'absent', 3000)",
        [],
    )
    .expect("insert a later absence");
    assert_eq!(absent_events(&app), 2);

    let result = backup_service::restore_backup(app.pool(), app.path(), &legacy).expect("restore");

    assert_eq!(
        absent_events(&app),
        1,
        "the legacy database was not written back over the live one"
    );
    assert!(
        !result.warnings.iter().any(|w| w.contains("X mark")),
        "a backup with no manifest must never raise the xCount mismatch warning: {result:#?}"
    );
    assert!(
        !result.warnings.iter().any(|w| w.contains("ahead of")),
        "{result:#?}"
    );
}

fn absent_events(app: &TempApp) -> i64 {
    app.pool
        .get()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM events WHERE event_type = 'absent'",
            [],
            |row| row.get(0),
        )
        .expect("count absences")
}

#[test]
fn a_legacy_flat_backup_still_produces_a_safety_backup_that_can_be_restored() {
    let app = TempApp::new();
    let legacy = app.write_legacy_flat_backup("attendance-manual-20200101_010101.db");

    let result = backup_service::restore_backup(app.pool(), app.path(), &legacy).expect("restore");
    let safety = PathBuf::from(&result.pre_restore_backup_path);

    assert!(safety.is_dir());
    let listed = backup_ops::list_backups(app.path()).expect("list");
    assert!(listed
        .iter()
        .any(|backup| backup.path == safety.to_string_lossy()));
    let preview = backup_ops::preview_backup(&safety).expect("preview the safety backup");
    assert!(preview.includes_database);
}

#[test]
fn a_pre_wipe_backup_already_holds_the_workbooks_without_an_extra_snapshot_call() {
    // D12 "before wipe". `take_pre_wipe_backup` in `commands/data_transfer.rs`
    // calls `create_backup_at` and nothing else, so the workbook snapshot has to
    // be part of `create_backup_at` itself — this test is what proves the wipe
    // path is covered without that file needing an edit.
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    app.write_live_workbook("SF2-OCTOBER-2026.xls");
    app.write_live_legacy_workbook();

    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::PreWipe, Local::now())
            .expect("create pre-wipe backup");
    let folder = PathBuf::from(&summary.path);

    assert!(summary.includes_database);
    assert_eq!(summary.kind, BackupKind::PreWipe);
    assert_eq!(summary.workbook_count, 3, "the nested tree counts too");
    let written = manifest::read_manifest(&folder)
        .expect("read")
        .expect("manifest");
    assert_eq!(written.kind, "pre-wipe");
    assert!(written
        .workbooks
        .iter()
        .any(|workbook| workbook.path == "workbooks/_legacy/SF2-OLD.xls"));
    assert!(manifest::workbooks_path(&folder)
        .join("_legacy/SF2-OLD.xls")
        .is_file());
}

// ── B2: retention and sync folder ──────────────────────────────────────

#[test]
fn retention_counts_folders_and_removes_the_oldest() {
    let app = TempApp::new();
    let now = Local::now();
    let limit = backup_ops::RETENTION_LIMIT as i64;

    // Oldest first, so the last one created really is the newest and is never
    // the entry retention prunes.
    for offset in (0..=limit).rev() {
        backup_ops::create_backup_at(
            app.pool(),
            app.path(),
            BackupKind::Auto,
            now - Duration::minutes(offset),
        )
        .expect("create backup");
    }
    assert_eq!(
        backup_ops::list_backups(app.path()).expect("list").len(),
        backup_ops::RETENTION_LIMIT,
        "retention runs as part of every backup"
    );

    let listed = backup_ops::list_backups(app.path()).expect("list");
    assert_eq!(listed.len(), backup_ops::RETENTION_LIMIT);
    // The folder written first was `now - limit` minutes old; the oldest
    // survivor must be newer than that, which is only true if it really was
    // pruned.
    let oldest_kept = listed.last().expect("oldest kept").created_at;
    assert!(
        oldest_kept > (now - Duration::minutes(limit)).timestamp(),
        "the oldest folder should have been pruned; the oldest survivor is {oldest_kept}"
    );
}

#[test]
fn retention_counts_legacy_files_and_folders_together() {
    let app = TempApp::new();
    let now = Local::now();
    let limit = backup_ops::RETENTION_LIMIT as i64;

    // One more than the limit, alternating the two shapes. The legacy file names
    // carry a distinct valid date so none of them collide on disk.
    for offset in 0..=limit {
        if offset % 2 == 0 {
            app.write_legacy_flat_backup(&format!(
                "attendance-auto-202001{:02}_010101.db",
                offset + 1
            ));
        } else {
            backup_ops::create_backup_at(
                app.pool(),
                app.path(),
                BackupKind::Auto,
                now - Duration::minutes(offset),
            )
            .expect("create backup");
        }
    }
    let mixed = backup_ops::list_backups(app.path()).expect("list");
    assert_eq!(mixed.len(), limit as usize + 1);
    assert!(mixed.iter().any(|backup| backup.path.ends_with(".db")));
    assert!(mixed.iter().any(|backup| !backup.path.ends_with(".db")));

    // Writing a legacy file bypasses `create_backup_at`, so retention has not run
    // since the last one; run it and both shapes must be counted together.
    backup_ops::enforce_retention(app.path()).expect("enforce retention");

    let listed = backup_ops::list_backups(app.path()).expect("list");
    assert_eq!(listed.len(), backup_ops::RETENTION_LIMIT);
}

#[test]
fn the_sync_folder_receives_the_whole_backup_folder() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    let sync = app.path().join("SyncTarget");
    fs::create_dir_all(&sync).expect("create sync target");

    backup_service::set_sync_folder(app.path(), Some(sync.clone())).expect("set sync folder");
    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::Manual, Local::now())
            .expect("create backup");

    let mirrored = sync
        .join("EES-AMS Backups")
        .join(Path::new(&summary.path).file_name().expect("name"));
    assert!(
        mirrored.is_dir(),
        "the sync folder mirrors the folder, not one file"
    );
    assert!(manifest::backup_db_path(&mirrored).is_file());
    assert!(manifest::manifest_path(&mirrored).is_file());
    assert!(manifest::workbooks_path(&mirrored).is_dir());
}

#[test]
fn snapshot_workbooks_for_fills_in_a_pre_wipe_backup_folder() {
    // The exact call Brief A's `take_pre_wipe_backup` will make: the backup
    // folder already exists (create_backup_at made it), and the workbooks are
    // added on top.
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::PreWipe, Local::now())
            .expect("create pre-wipe backup");
    fs::write(
        workbooks::workbook_source_dir(app.path()).join("SF2-DECEMBER-2026.xls"),
        b"made-after-the-backup",
    )
    .expect("add a later workbook");

    let snapshot = backup_service::snapshot_workbooks_for(app.path(), Path::new(&summary.path))
        .expect("snapshot");

    assert_eq!(snapshot.entries.len(), 2);
    assert!(!snapshot.source_missing);
    assert_eq!(
        snapshot.source_dir,
        workbooks::workbook_source_dir(app.path())
    );
    assert!(manifest::workbooks_path(Path::new(&summary.path))
        .join("SF2-DECEMBER-2026.xls")
        .is_file());
    // The manifest written by create_backup_at is not rewritten by the extra
    // snapshot, so its count still reflects what that backup captured.
    let written = manifest::read_manifest(Path::new(&summary.path))
        .expect("read")
        .expect("manifest");
    assert_eq!(written.workbooks.len(), 1);
}

// ── B3: "Back up workbooks now" ────────────────────────────────────────

#[test]
fn the_workbooks_only_backup_has_no_database() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    app.write_live_workbook("SF2-OCTOBER-2026.xls");

    let status = backup_ops::create_workbooks_backup(app.path()).expect("workbooks backup");

    let path = status
        .last_workbooks_backup_path
        .expect("workbooks backup path");
    let folder = PathBuf::from(&path);
    assert!(folder.is_dir());
    assert!(
        !manifest::backup_db_path(&folder).exists(),
        "D13 must not duplicate the database"
    );
    assert!(manifest::manifest_path(&folder).is_file());
    assert_eq!(
        fs::read_dir(manifest::workbooks_path(&folder))
            .expect("read")
            .count(),
        2
    );

    let written = manifest::read_manifest(&folder)
        .expect("read")
        .expect("manifest");
    assert_eq!(written.kind, "manual-workbooks");
    assert_eq!(written.workbooks.len(), 2);
    assert!(written
        .workbooks
        .iter()
        .all(|workbook| workbook.path.starts_with("workbooks/")));
}

#[test]
fn a_workbooks_only_backup_is_listed_without_a_database() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    backup_ops::create_workbooks_backup(app.path()).expect("workbooks backup");

    let listed = backup_ops::list_backups(app.path()).expect("list");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].kind, BackupKind::ManualWorkbooks);
    assert!(!listed[0].includes_database);
    assert_eq!(listed[0].workbook_count, 1);
}

#[test]
fn a_workbooks_only_backup_does_not_satisfy_the_daily_backup_check() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    backup_ops::create_workbooks_backup(app.path()).expect("workbooks backup");

    let now = Local::now();
    ensure_daily_backup_at(app.pool(), app.path(), now).expect("daily backup");

    let listed = backup_ops::list_backups(app.path()).expect("list");
    assert_eq!(
        listed.len(),
        2,
        "a workbooks-only folder is not a daily backup"
    );
    assert!(listed.iter().any(|backup| backup.includes_database));
}

// ── B4: the update-time attendance fingerprint ─────────────────────────

#[test]
fn the_fingerprint_reads_events_absences_and_the_absent_month_mapping_count() {
    let app = TempApp::new();
    let conn = app.pool.get().expect("connection");

    let read = fingerprint::read_db_fingerprint(&conn).expect("read fingerprint");

    assert_eq!(read.events, 3);
    assert_eq!(read.absent, 1);
    // The fixture has no month date mappings, so this is 0. The point of the
    // assertion is that reading the table never fails — including on a database
    // from before the v21 migration created it.
    assert_eq!(read.month_date_mappings, 0);
}

#[test]
fn the_fingerprint_is_recorded_once_and_then_left_alone() {
    let app = TempApp::new();
    assert!(fingerprint::load_db_fingerprint(app.path())
        .expect("load")
        .is_none());

    let conn = app.pool.get().expect("connection");
    fingerprint::record_db_fingerprint_if_absent(app.path(), &conn).expect("record");
    let first = fingerprint::load_db_fingerprint(app.path())
        .expect("load")
        .expect("recorded");

    conn.execute("INSERT INTO events (id, student_id, event_type, timestamp) VALUES ('e9', 's1', 'absent', 3000)", [])
        .expect("insert absence");
    fingerprint::record_db_fingerprint_if_absent(app.path(), &conn).expect("record again");
    let second = fingerprint::load_db_fingerprint(app.path())
        .expect("load")
        .expect("recorded");

    assert_eq!(first.absent, 1);
    assert_eq!(
        second.absent, 1,
        "an existing baseline must not be overwritten"
    );
    assert_eq!(second.events, first.events);
}

#[test]
fn a_drop_in_absences_is_reported_and_a_rise_is_not() {
    let previous = fingerprint::DbFingerprint {
        events: 812,
        absent: 63,
        month_date_mappings: 30,
    };

    let dropped = fingerprint::DbFingerprint {
        events: 400,
        absent: 12,
        month_date_mappings: 30,
    };
    let notice = fingerprint::DbFingerprint::decrease_notice(&previous, &dropped)
        .expect("a drop must be reported");
    assert!(notice.contains("63"), "{notice}");
    assert!(notice.contains("12"), "{notice}");

    let same = previous;
    assert!(fingerprint::DbFingerprint::decrease_notice(&previous, &same).is_none());

    let grown = fingerprint::DbFingerprint {
        events: 900,
        absent: 70,
        month_date_mappings: 31,
    };
    assert!(fingerprint::DbFingerprint::decrease_notice(&previous, &grown).is_none());
}

// ── A database ahead of the running app ───────────────────────────────
//
// What a build from a later branch — or a rollback to an older installer —
// leaves on disk: the schema is newer than the running binary understands. A
// backup must still be possible in that state; refusing one deadlocks the
// update, because the pre-install backup is what gates the install that would
// bring the version able to read the database.

fn bump_live_schema_ahead_of_the_app(app: &TempApp) -> i32 {
    let ahead = crate::infrastructure::database::CURRENT_SCHEMA_VERSION + 1;
    let conn = app.pool.get().expect("connection");
    conn.execute_batch(&format!("PRAGMA user_version = {ahead};"))
        .expect("bump the live schema version");
    ahead
}

#[test]
fn a_database_ahead_of_this_build_is_still_backed_up() {
    let app = TempApp::new();
    let ahead = bump_live_schema_ahead_of_the_app(&app);

    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::PreInstall, Local::now())
            .expect("a newer schema must not fail the backup");

    let folder = PathBuf::from(&summary.path);
    let copy = rusqlite::Connection::open_with_flags(
        manifest::backup_db_path(&folder),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open the backup copy");
    let copied_version: i32 = copy
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("read the copy's schema version");

    assert_eq!(
        copied_version, ahead,
        "the copy must carry the live schema, not a downgraded one"
    );
}

#[test]
fn previewing_a_backup_ahead_of_this_build_is_still_refused() {
    let app = TempApp::new();
    let ahead = bump_live_schema_ahead_of_the_app(&app);
    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::Manual, Local::now())
            .expect("create backup");

    let error = backup_ops::preview_backup(&PathBuf::from(&summary.path))
        .expect_err("restoring a database this build cannot read must be refused");

    assert!(
        error.to_string().contains("newer than this app supports"),
        "ahead={ahead}, {error}"
    );
}

// ── Google Drive: the folder is zipped ─────────────────────────────────

#[test]
fn a_backup_folder_zips_into_a_readable_archive() {
    let app = TempApp::new();
    app.write_live_workbook("SF2-SEPTEMBER-2026.xls");
    let summary =
        backup_ops::create_backup_at(app.pool(), app.path(), BackupKind::Manual, Local::now())
            .expect("create backup");

    let archive = super::super::zip_writer::zip_directory_to_bytes(Path::new(&summary.path))
        .expect("zip backup folder");

    assert_eq!(&archive[0..4], b"PK\x03\x04", "local file header signature");
    // The end-of-central-directory record is the last 22 bytes, and its
    // signature must be intact.
    let eocd = &archive[archive.len() - 22..archive.len() - 18];
    assert_eq!(eocd, b"PK\x05\x06", "end of central directory signature");

    let text = String::from_utf8_lossy(&archive);
    assert!(text.contains("workbooks/SF2-SEPTEMBER-2026.xls"), "{text}");
    assert!(text.contains("attendance.db"), "{text}");
    assert!(text.contains("manifest.json"), "{text}");
}
