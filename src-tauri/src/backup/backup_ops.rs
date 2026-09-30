use super::file_ops::{
    backup_dir, copy_path_recursive, is_app_backup_file, is_app_backup_folder, load_state,
    resolve_backup_database, save_state, summary_from_path, unique_backup_path, BackupState,
    SYNC_BACKUP_DIR_NAME,
};
use super::fingerprint::read_db_fingerprint;
use super::manifest::{
    backup_db_path, manifest_kind, read_manifest, workbooks_path, write_manifest, BackupManifest,
    ManifestCounts,
};
use super::models::{
    BackupKind, BackupPreview, BackupStatus, BackupSummary, BackupWorkbookPreview,
};
use super::sqlite_utils::{
    count_table_rows, open_table_exists, read_schema_version, require_core_tables,
    run_integrity_check,
};
use super::workbooks::{self, WorkbookSnapshot};
use crate::infrastructure::database::DbPool;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local};
use rusqlite::{Connection, DatabaseName, OpenFlags};
use std::{
    fs,
    path::{Path, PathBuf},
};

// ── Public API ────────────────────────────────────────────────────────

pub fn get_status(app_dir: &Path) -> Result<BackupStatus> {
    let backups = list_backups(app_dir)?;
    let backup_dir = backup_dir(app_dir);
    let state = load_state(app_dir).unwrap_or_else(|error| BackupState {
        last_error: Some(format!("Failed to read backup settings: {error}")),
        ..BackupState::default()
    });
    let latest = backups
        .iter()
        .find(|backup| backup.includes_database)
        .or_else(|| backups.first());
    let google_drive = state.google_drive.clone();
    let last_workbooks_backup_path = state.last_workbooks_backup_path.clone().or_else(|| {
        backups
            .iter()
            .find(|backup| backup.kind == BackupKind::ManualWorkbooks)
            .map(|backup| backup.path.clone())
    });

    Ok(BackupStatus {
        local_backup_dir: backup_dir.to_string_lossy().to_string(),
        backup_count: backups.len(),
        retention_limit: RETENTION_LIMIT,
        last_backup_at: state
            .last_backup_at
            .or_else(|| latest.map(|backup| backup.created_at)),
        last_backup_path: state
            .last_backup_path
            .or_else(|| latest.map(|backup| backup.path.clone())),
        last_workbooks_backup_path,
        sync_folder_path: state.sync_folder_path,
        last_error: state.last_error,
        last_sync_error: state.last_sync_error,
        google_drive_configured: google_drive_client_id().is_ok(),
        google_drive_connected: google_drive.is_some(),
        google_drive_folder_id: google_drive.as_ref().map(|drive| drive.folder_id.clone()),
        google_drive_folder_name: google_drive.as_ref().map(|drive| drive.folder_name.clone()),
        last_google_drive_backup_at: google_drive.as_ref().and_then(|drive| drive.last_backup_at),
        last_google_drive_file_id: google_drive
            .as_ref()
            .and_then(|drive| drive.last_file_id.clone()),
        last_google_drive_error: google_drive
            .as_ref()
            .and_then(|drive| drive.last_error.clone()),
    })
}

/// List every backup, newest first.
///
/// Accepts both shapes and lists them together: folder-shaped backups (today's
/// format) and the legacy flat `*.db` files the previous build left behind.
/// Retention counts both.
pub fn list_backups(app_dir: &Path) -> Result<Vec<BackupSummary>> {
    let backup_dir = backup_dir(app_dir);
    fs::create_dir_all(&backup_dir)
        .with_context(|| format!("failed to create backup directory {}", backup_dir.display()))?;

    let mut backups = Vec::new();
    for entry in fs::read_dir(&backup_dir)
        .with_context(|| format!("failed to read backup directory {}", backup_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string)
        else {
            continue;
        };

        let recognised = match path.is_dir() {
            true => is_app_backup_folder(&file_name),
            false => is_app_backup_file(&file_name),
        };
        if !recognised {
            continue;
        }

        backups.push(summary_from_path(&path)?);
    }

    backups.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.file_name.cmp(&left.file_name))
    });

    Ok(backups)
}

pub fn create_manual_backup(pool: &DbPool, app_dir: &Path) -> Result<BackupStatus> {
    create_backup_at(pool, app_dir, BackupKind::Manual, Local::now())?;
    get_status(app_dir)
}

/// The D13 "Back up workbooks now" backup.
///
/// Writes `backups/attendance-manual-workbooks-<ts>/` holding only the
/// `workbooks/` subtree and its manifest. Deliberately does *not* copy
/// `attendance.db`: the database has its own backups, and duplicating it here
/// would let the two drift apart.
pub fn create_workbooks_backup(app_dir: &Path) -> Result<BackupStatus> {
    let backup_root = backup_dir(app_dir);
    fs::create_dir_all(&backup_root).with_context(|| {
        format!(
            "failed to create backup directory {}",
            backup_root.display()
        )
    })?;

    let final_path = unique_backup_path(&backup_root, BackupKind::ManualWorkbooks, Local::now());
    let temp_path = temp_sibling(&final_path);
    remove_if_exists(&temp_path)?;
    fs::create_dir_all(&temp_path)
        .with_context(|| format!("failed to create {}", temp_path.display()))?;

    let snapshot = workbooks::snapshot_workbooks_into(app_dir, &temp_path)?;
    if snapshot.source_missing {
        log::info!(
            "no SF2 workbooks to snapshot yet ({} does not exist)",
            snapshot.source_dir.display()
        );
    }
    let counts = read_manifest_counts_from_live(app_dir);
    write_manifest(
        &temp_path,
        &BackupManifest {
            schema_version: crate::infrastructure::database::CURRENT_SCHEMA_VERSION,
            created_at: Local::now().to_rfc3339(),
            kind: manifest_kind(BackupKind::ManualWorkbooks).to_string(),
            counts,
            workbooks: snapshot.entries.clone(),
        },
    )?;

    fs::rename(&temp_path, &final_path).with_context(|| {
        format!(
            "failed to finalize workbook backup {} -> {}",
            temp_path.display(),
            final_path.display()
        )
    })?;

    // Summarise before pruning, for the same reason `create_backup_at` does.
    let summary = summary_from_path(&final_path)?;
    enforce_retention(app_dir)?;

    let mut state = load_state(app_dir).unwrap_or_default();
    state.last_workbooks_backup_path = Some(summary.path.clone());
    state.last_error = None;
    state.last_sync_error = copy_to_sync_folder(&state, &final_path).err().map(|error| {
        log::warn!("workbook backup sync failed: {error}");
        error.to_string()
    });
    save_state(app_dir, &state)?;

    log::info!(
        "created workbook-only backup {} with {} workbook(s)",
        summary.path,
        summary.workbook_count
    );

    get_status(app_dir)
}

pub fn backup_database_to_path(pool: &DbPool, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("backup destination has no parent directory"))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;

    let temp_path = destination.with_extension("db.tmp");
    if temp_path.exists() {
        fs::remove_file(&temp_path).with_context(|| {
            format!("failed to remove stale temp backup {}", temp_path.display())
        })?;
    }

    let source = pool.get().context("failed to get database connection")?;
    source
        .backup(
            DatabaseName::Main,
            &temp_path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .with_context(|| format!("failed to export database {}", temp_path.display()))?;
    inspect_backup(&temp_path, true).context("exported database failed validation")?;

    if destination.exists() {
        fs::remove_file(destination)
            .with_context(|| format!("failed to replace {}", destination.display()))?;
    }
    fs::rename(&temp_path, destination).with_context(|| {
        format!(
            "failed to finalize database export {} -> {}",
            temp_path.display(),
            destination.display()
        )
    })?;

    Ok(())
}

pub fn set_sync_folder(app_dir: &Path, folder_path: Option<PathBuf>) -> Result<BackupStatus> {
    let mut state = load_state(app_dir).unwrap_or_default();
    state.sync_folder_path = folder_path
        .map(|path| prepare_sync_folder(&path))
        .transpose()?
        .map(|path| path.to_string_lossy().to_string());
    state.last_sync_error = None;
    save_state(app_dir, &state)?;
    get_status(app_dir)
}

/// Inspect a backup without restoring it.
///
/// Accepts either shape. A folder resolves to the `attendance.db` inside it and
/// reads the per-file `xCount` from `manifest.json`; a legacy flat `*.db` file
/// resolves to itself and has no manifest at all, so its workbook list is empty
/// rather than a list of `xCount: 0` entries that would read as "this workbook
/// holds no marks" and trip the §10.2 mismatch warning spuriously.
///
/// Both shapes go through the same validation and the same counts — a legacy
/// backup is a real database and deserves a real preview, not a row of zeros.
pub fn preview_backup(source_path: &Path) -> Result<BackupPreview> {
    inspect_backup(source_path, false)
}

/// `allow_ahead` decides how a schema newer than this build is treated. Snapshotting
/// the live database passes `true`: the copy is a faithful image of what is already
/// on disk, and refusing it here deadlocks the update — the pre-install backup fails,
/// so the install never runs, so the version that reads the database never arrives.
/// A database ahead of the app is exactly the state left behind by a build from a
/// later branch, or by a rollback to an older installer. Restoring passes `false`:
/// a database this app cannot read is refused up front, before anything touches the
/// live data.
fn inspect_backup(source_path: &Path, allow_ahead: bool) -> Result<BackupPreview> {
    if !source_path.exists() {
        bail!("Backup does not exist: {}", source_path.display());
    }

    let metadata = fs::metadata(source_path)
        .with_context(|| format!("failed to inspect backup {}", source_path.display()))?;
    let modified_at = super::file_ops::metadata_timestamp(&metadata)?;
    let file_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("backup")
        .to_string();

    let is_folder = source_path.is_dir();
    let manifest = if is_folder {
        match read_manifest(source_path) {
            Ok(manifest) => manifest,
            Err(error) => {
                log::warn!(
                    "backup folder {} has an unreadable manifest: {error}",
                    source_path.display()
                );
                None
            }
        }
    } else {
        None
    };

    let workbook_previews = manifest
        .as_ref()
        .map(|manifest| {
            workbooks::resolve_manifest_workbooks(source_path, manifest)
                .into_iter()
                .map(|(relative_path, absolute, x_count)| BackupWorkbookPreview {
                    file_name: absolute
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(&relative_path)
                        .to_string(),
                    relative_path,
                    bytes: fs::metadata(&absolute).map(|meta| meta.len()).unwrap_or(0),
                    x_count,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let size_bytes = if is_folder {
        super::file_ops::directory_size(source_path)?
    } else {
        metadata.len()
    };

    let mut warnings = Vec::new();
    warnings.extend(workbook_absence_notes(
        source_path,
        is_folder,
        &workbook_previews,
    ));

    let database_path = resolve_backup_database(source_path)?;
    let conn = Connection::open_with_flags(&database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open backup {}", database_path.display()))?;
    run_integrity_check(&conn)?;

    let current_version = crate::infrastructure::database::CURRENT_SCHEMA_VERSION;
    let schema_version = read_schema_version(&conn)?;
    if schema_version > current_version {
        if !allow_ahead {
            bail!("Backup schema version {schema_version} is newer than this app supports ({current_version})");
        }
        log::warn!(
            "snapshot schema version {schema_version} is newer than this app supports \
             ({current_version}); keeping the copy anyway"
        );
        warnings.push(format!(
            "This backup is schema version {schema_version}, newer than this app supports \
             ({current_version}). Restoring it needs the newer version of the app."
        ));
    }

    require_core_tables(&conn)?;

    if schema_version < current_version {
        warnings.push(format!(
            "Backup will be migrated from schema version {schema_version} to {current_version} during restore."
        ));
    }

    let absent_count = count_absent_events(&conn)?;
    warnings.extend(workbook_absence_warning(&workbook_previews, absent_count));

    Ok(BackupPreview {
        source_path: source_path.to_string_lossy().to_string(),
        database_path: database_path.to_string_lossy().to_string(),
        file_name,
        modified_at,
        size_bytes,
        schema_version,
        student_count: count_table_rows(&conn, "students")?,
        class_count: count_table_rows(&conn, "classes")?,
        event_count: count_table_rows(&conn, "events")?,
        absent_count,
        settings_count: count_table_rows(&conn, "settings")?,
        sf2_template_count: count_table_rows(&conn, "sf2_templates")?,
        includes_database: true,
        workbooks: workbook_previews,
        warnings,
    })
}

/// The phrase every "this backup carries no workbooks" note shares, so
/// [`restore_service`] can tell whether the user has already been told without
/// duplicating or re-deriving the message.
pub(crate) const NO_WORKBOOKS_MARKER: &str = "no SF2 workbooks";

/// Everything the user should be told about a backup's workbook subtree,
/// before any restore runs.
///
/// A backup that carries no workbooks is not an error and is not a warning about
/// a mismatch — it is simply an older backup, and the user deserves to know
/// their live workbooks will be left alone.
fn workbook_absence_notes(
    source_path: &Path,
    is_folder: bool,
    workbook_previews: &[BackupWorkbookPreview],
) -> Vec<String> {
    if !is_folder {
        return vec![format!(
            "This backup predates workbook backup: it holds only the database and \
             {NO_WORKBOOKS_MARKER}. Restoring it leaves the current SF2 workbooks \
             untouched."
        )];
    }

    let workbooks_dir = workbooks_path(source_path);
    if !workbooks_dir.is_dir() {
        return vec![format!(
            "This backup holds {NO_WORKBOOKS_MARKER}. Restoring it leaves the current \
             SF2 workbooks untouched."
        )];
    }

    let snapshot_file_count = workbooks::list_files(&workbooks_dir).map_or(0, |files| files.len());
    if snapshot_file_count == 0 {
        return vec![format!(
            "The workbooks folder in this backup is empty, so it holds \
             {NO_WORKBOOKS_MARKER}. Restoring it leaves the current SF2 workbooks \
             untouched."
        )];
    }

    if workbook_previews.is_empty() {
        return vec![
            "This backup has a workbooks folder but no manifest listing. \
             The workbooks will be restored, but their X-mark counts are unknown."
                .to_string(),
        ];
    }

    Vec::new()
}

/// The critical pre-restore warning: a workbook in this backup holds more
/// absences than the database it is paired with.
///
/// Restoring does not merge the two — it makes the database match the backup
/// exactly — so after a restore the workbook is ahead of the database and the
/// app will re-import the difference on the next SF2 open. The user has to know
/// that before they click, not after.
pub fn workbook_absence_warning(
    workbooks: &[BackupWorkbookPreview],
    absent_count: i64,
) -> Option<String> {
    let expected = workbooks
        .iter()
        .map(|workbook| workbook.x_count)
        .max()
        .unwrap_or(0);
    if expected <= 0 || absent_count >= expected {
        return None;
    }
    let names = workbooks
        .iter()
        .filter(|workbook| workbook.x_count == expected)
        .map(|workbook| workbook.file_name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "This backup's workbooks record {expected} X mark(s) but its database holds only \
         {absent_count} absence(s). Restoring pairs a database that is behind its own \
         workbooks ({names}). The app re-imports the missing marks the next time you \
         open SF2 — restore only if that is what you want."
    ))
}

fn count_absent_events(conn: &Connection) -> Result<i64> {
    if !open_table_exists(conn, "events")? {
        return Ok(0);
    }
    conn.query_row(COUNT_ABSENT_EVENTS_SQL, [], |row| row.get(0))
        .map_err(Into::into)
}

pub fn enforce_retention(app_dir: &Path) -> Result<()> {
    let backups = list_backups(app_dir)?;
    for backup in backups.into_iter().skip(RETENTION_LIMIT) {
        let path = PathBuf::from(&backup.path);
        remove_if_exists(&path)
            .with_context(|| format!("failed to remove old backup {}", backup.path))?;
    }
    Ok(())
}

// ── Core Backup ───────────────────────────────────────────────────────

/// Write a backup folder: `attendance.db`, the `workbooks/` subtree, then
/// `manifest.json`.
///
/// The folder is assembled under a `.tmp` sibling and renamed into place only
/// once every part is written, so an interrupted backup never appears in
/// `list_backups` as a half-complete folder.
pub(crate) fn create_backup_at(
    pool: &DbPool,
    app_dir: &Path,
    kind: BackupKind,
    now: DateTime<Local>,
) -> Result<BackupSummary> {
    let backup_root = backup_dir(app_dir);
    fs::create_dir_all(&backup_root).with_context(|| {
        format!(
            "failed to create backup directory {}",
            backup_root.display()
        )
    })?;

    let final_path = unique_backup_path(&backup_root, kind, now);
    let temp_path = temp_sibling(&final_path);
    remove_if_exists(&temp_path)?;
    fs::create_dir_all(&temp_path)
        .with_context(|| format!("failed to create {}", temp_path.display()))?;

    let database_path = backup_db_path(&temp_path);
    let source = pool.get().context("failed to get database connection")?;
    source
        .backup(
            DatabaseName::Main,
            &database_path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .with_context(|| format!("failed to create backup {}", database_path.display()))?;

    // Validate the exported copy before anything else is written into the
    // folder: a backup that cannot be opened is not a backup. A schema ahead of
    // this build is allowed — see [`inspect_backup`].
    let preview =
        inspect_backup(&database_path, true).context("created backup failed validation")?;

    // Copying the workbooks is allowed to fail the whole backup. A folder that
    // silently lacks the X marks is the failure mode this task exists to stop.
    let snapshot = workbooks::snapshot_workbooks_into(app_dir, &temp_path)?;

    let conn = Connection::open_with_flags(&database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to reopen backup {}", database_path.display()))?;
    let counts = read_manifest_counts(&conn)?;

    write_manifest(
        &temp_path,
        &BackupManifest {
            schema_version: crate::infrastructure::database::CURRENT_SCHEMA_VERSION,
            created_at: now.to_rfc3339(),
            kind: manifest_kind(kind).to_string(),
            counts,
            workbooks: snapshot.entries.clone(),
        },
    )?;

    drop(conn);

    fs::rename(&temp_path, &final_path).with_context(|| {
        format!(
            "failed to finalize backup {} -> {}",
            temp_path.display(),
            final_path.display()
        )
    })?;

    // Summarise before pruning: `enforce_retention` deletes the oldest entries,
    // and a caller that asked for a backup must never be handed a path that
    // retention has just removed.
    let summary = summary_from_path(&final_path)?;
    enforce_retention(app_dir)?;

    let mut state = load_state(app_dir).unwrap_or_default();
    state.last_backup_at = Some(summary.created_at);
    state.last_backup_path = Some(summary.path.clone());
    state.last_error = None;
    state.last_sync_error = copy_to_sync_folder(&state, &final_path).err().map(|error| {
        log::warn!("backup sync failed: {error}");
        error.to_string()
    });
    if let Err(error) = super::google_drive::upload_backup_to_google_drive(&mut state, &final_path)
    {
        if let Some(google_drive) = state.google_drive.as_mut() {
            google_drive.last_error = Some(error.to_string());
        }
        log::warn!("Google Drive backup upload failed: {error}");
    }
    save_state(app_dir, &state)?;

    log::info!(
        "created {} backup at {} ({} workbook(s), {} x-mark(s), {} absent event(s))",
        manifest_kind(kind),
        summary.path,
        snapshot.entries.len(),
        snapshot.total_x_count(),
        preview.absent_count,
    );

    Ok(summary)
}

// ── Sync Folder ───────────────────────────────────────────────────────

/// Mirror a backup into the sync folder.
///
/// Copies the whole folder for a folder-shaped backup — the database, the
/// manifest and the workbooks together, since a `.db` without its workbooks is
/// no longer a complete backup. A legacy flat `.db` is still copied as a file.
fn copy_to_sync_folder(state: &BackupState, source_path: &Path) -> Result<()> {
    let Some(sync_folder_path) = &state.sync_folder_path else {
        return Ok(());
    };

    let sync_folder = PathBuf::from(sync_folder_path);
    if !sync_folder.is_dir() {
        bail!("sync folder is unavailable: {}", sync_folder.display());
    }

    let file_name = source_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("backup file name is missing"))?;
    let destination = sync_folder.join(file_name);
    copy_path_recursive(source_path, &destination).with_context(|| {
        format!(
            "failed to copy backup to sync folder {}",
            destination.display()
        )
    })
}

fn prepare_sync_folder(selected_folder: &Path) -> Result<PathBuf> {
    let sync_folder = if selected_folder
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(SYNC_BACKUP_DIR_NAME))
    {
        selected_folder.to_path_buf()
    } else {
        selected_folder.join(SYNC_BACKUP_DIR_NAME)
    };

    fs::create_dir_all(&sync_folder)
        .with_context(|| format!("failed to create sync folder {}", sync_folder.display()))?;

    Ok(sync_folder)
}

// ── Helpers ───────────────────────────────────────────────────────────

fn read_manifest_counts(conn: &Connection) -> Result<ManifestCounts> {
    let fingerprint = read_db_fingerprint(conn)?;
    Ok(ManifestCounts {
        students: count_table_rows(conn, "students")?,
        events: fingerprint.events,
        absent: fingerprint.absent,
        // Guarded by `count_table_rows`, which returns 0 for a missing table
        // rather than failing. That keeps a backup readable from a database
        // older than the `sf2_month_templates` migration (v19), which is
        // exactly what restoring an old backup does.
        sf2_month_templates: count_table_rows(conn, "sf2_month_templates")?,
    })
}

fn read_manifest_counts_from_live(app_dir: &Path) -> ManifestCounts {
    let live_db = app_dir.join("attendance.db");
    let Ok(conn) = Connection::open_with_flags(
        &live_db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return ManifestCounts::default();
    };
    read_manifest_counts(&conn).unwrap_or_default()
}

/// A `.tmp` sibling of a backup folder, so the finished folder can be renamed
/// into place atomically.
fn temp_sibling(final_path: &Path) -> PathBuf {
    let name = final_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("attendance-backup");
    final_path.with_file_name(format!("{name}.tmp"))
}

fn remove_if_exists(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove stale {}", path.display()))?;
    } else if path.exists() {
        fs::remove_file(path)
            .with_context(|| format!("failed to remove stale {}", path.display()))?;
    }
    Ok(())
}

fn google_drive_client_id() -> Result<String> {
    option_env!("EES_AMS_GOOGLE_CLIENT_ID")
        .map(str::to_string)
        .or_else(|| std::env::var("EES_AMS_GOOGLE_CLIENT_ID").ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Google Drive is not configured. Set EES_AMS_GOOGLE_CLIENT_ID before building the app."
            )
        })
}

const COUNT_ABSENT_EVENTS_SQL: &str = include_str!("sql/count_absent_events.sql");

/// Retention counts backup *entries* — a folder counts as one, the same as the
/// flat `.db` it replaced. A folder costs more disk than a file, so a
/// disk-budget cap would be worth adding; the count-based cap is what the
/// previous build shipped and is what the UI reports.
pub(crate) const RETENTION_LIMIT: usize = 30;

// brief-B2: §9.4 item 4 also asks for pre-*migration* snapshot retention to go
// 3 -> 5, because the destructive migrations are v11 and v17. That constant
// (`SNAPSHOT_HISTORY`) lives in `infrastructure/database/migrations.rs`, which
// another agent owns; it is handed off in `.superpowers/sdd/sf2-month-workbooks/report-B2.md`
// rather than edited here. Those pre-migration snapshots are bare `.db` files, so
// they are exactly the legacy shape `file_ops::is_app_backup_file` and
// `preview_backup` must keep accepting.

/// The D12 snapshot the update installer, the pre-wipe path and the
/// pre-install path all call: copy the live SF2 workbooks into a backup folder
/// that already has its database and manifest.
///
/// # Signature for Brief A (pre-wipe wiring)
///
/// ```ignore
/// pub(crate) fn snapshot_workbooks_for(
///     app_dir: &std::path::Path,
///     backup_folder: &std::path::Path,
/// ) -> anyhow::Result<WorkbookSnapshot>
/// ```
///
/// `backup_folder` is an existing backup directory — the same value
/// `create_backup_at` returns as `BackupSummary::path`. Call it *before* the
/// wipe deletes anything. It is idempotent (the copy overwrites) and it fails
/// loudly rather than writing an empty `workbooks/`.
// The only in-tree caller arrives with the pre-wipe wiring (Brief A), so this
// reads as dead code until then.
#[allow(dead_code)]
pub(crate) fn snapshot_workbooks_for(
    app_dir: &Path,
    backup_folder: &Path,
) -> Result<WorkbookSnapshot> {
    let snapshot = workbooks::snapshot_workbooks_into(app_dir, backup_folder)?;
    log::info!(
        "snapshotted {} SF2 workbook(s) from {} into {}",
        snapshot.entries.len(),
        snapshot.source_dir.display(),
        backup_folder.display(),
    );
    Ok(snapshot)
}
