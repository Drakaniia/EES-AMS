//! `SettingsRepository::update` must never touch a column the `Settings` model
//! does not own (spec §9.1, spec v22).
//!
//! The three columns below were added by the v22 migration and are read and
//! written by the per-month workbook model, never by the Settings page. A save
//! that replaced the whole row would null all three, and `sf2_split_completed_at`
//! going NULL re-runs the entire 12-month split on the next launch - which
//! rewrites every month file. That is a destructive-on-save bug in an app whose
//! subject is not destroying data, so the test below is the regression that
//! keeps it fixed.

use super::{init_db, DbPool, SettingsRepository};
use crate::domain::error::Result;
use crate::domain::models::{AttendanceMode, Settings};
use rusqlite::params;

/// A migrated, empty database in a throwaway directory.
fn test_pool() -> DbPool {
    let dir = tempfile::tempdir().expect("temp dir for the test database");
    let pool = init_db(dir.path().join("attendance.db")).expect("migrate test database");
    std::mem::forget(dir);
    pool
}

/// Read the three v22 columns straight out of the row, because `Settings` does
/// not carry them and the point of the test is that the *database* kept them.
fn v22_columns(pool: &DbPool) -> (Option<String>, Option<String>, Option<i64>) {
    let conn = pool.get().expect("connection");
    conn.query_row(
        "SELECT school_start_date, last_report_month, sf2_split_completed_at
         FROM settings WHERE id = 'app'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .expect("the app settings row")
}

fn seed_v22_columns(pool: &DbPool) {
    let conn = pool.get().expect("connection");
    conn.execute(
        "UPDATE settings
         SET school_start_date = ?1, last_report_month = ?2, sf2_split_completed_at = ?3
         WHERE id = 'app'",
        params![
            "2026-08-03".to_string(),
            "JUNE".to_string(),
            1_700_000_000i64
        ],
    )
    .expect("seed the v22 columns");
}

fn save(pool: &DbPool) -> Result<Settings> {
    let repo = SettingsRepository::new(pool.clone());
    let mut settings = repo.get()?;
    settings.school_name = Some("Mabini Elementary School".to_string());
    settings.report_month = Some("JUNE".to_string());
    settings.attendance_mode = AttendanceMode::CardReader;
    repo.update(settings)
}

#[test]
fn a_settings_save_keeps_the_v22_per_month_columns() {
    let pool = test_pool();
    seed_v22_columns(&pool);

    let saved = save(&pool).expect("save the settings page");
    assert_eq!(
        saved.school_name.as_deref(),
        Some("Mabini Elementary School"),
        "the save itself must land"
    );

    let (school_start_date, last_report_month, split_completed_at) = v22_columns(&pool);
    assert_eq!(
        school_start_date.as_deref(),
        Some("2026-08-03"),
        "`school_start_date` is the input every month's first_school_day is derived from; \
         a Settings-page save must not clear it"
    );
    assert_eq!(
        last_report_month.as_deref(),
        Some("JUNE"),
        "`last_report_month` is the E1 fallback month; a Settings-page save must not clear it"
    );
    assert_eq!(
        split_completed_at,
        Some(1_700_000_000),
        "`sf2_split_completed_at` gates the one-time 12-month split. Nulling it re-runs the \
         whole split, which rewrites every month file - a destructive save"
    );
}

#[test]
fn a_settings_save_still_writes_every_column_the_model_owns() {
    // The other half of the contract: making the write targeted must not have
    // turned it into a partial update that silently drops a real setting.
    let pool = test_pool();
    let repo = SettingsRepository::new(pool.clone());

    let mut settings = repo.get().expect("read the defaults");
    settings.school_id = Some("DEPED-4021".to_string());
    settings.adviser_name = Some("Dela Cruz, Juan".to_string());
    settings.branding_title = "  Mabini EES  ".to_string();
    settings.q1_start = Some("2026-06-01".to_string());
    settings.q2_end = Some("2026-10-31".to_string());
    repo.update(settings).expect("save");

    let reloaded = repo.get().expect("re-read");
    assert_eq!(reloaded.school_id.as_deref(), Some("DEPED-4021"));
    assert_eq!(reloaded.adviser_name.as_deref(), Some("Dela Cruz, Juan"));
    assert_eq!(reloaded.q1_start.as_deref(), Some("2026-06-01"));
    assert_eq!(reloaded.q2_end.as_deref(), Some("2026-10-31"));
    assert_eq!(
        reloaded.branding_title, "Mabini EES",
        "branding_title keeps its existing trim-and-default behaviour"
    );
}

#[test]
fn a_settings_save_on_a_fresh_install_still_creates_the_row() {
    // The targeted write is an upsert, not an update: an install whose `settings`
    // row does not exist yet must still get one.
    let pool = test_pool();
    save(&pool).expect("save against an empty database");

    let conn = pool.get().expect("connection");
    let rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM settings WHERE id = 'app'",
            [],
            |row| row.get(0),
        )
        .expect("count the app settings row");
    assert_eq!(rows, 1, "the app settings row must exist after a save");
}
