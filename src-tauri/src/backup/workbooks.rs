use super::manifest::{BackupManifest, ManifestWorkbook, WORKBOOK_DIR_NAME};
use anyhow::{Context, Result};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

// ── Constants ─────────────────────────────────────────────────────────

/// Name of the live SF2 working-copy directory, under the app data directory.
///
/// Mirrors the literal in `src-tauri/src/sf2/workbook_files.rs:42`
/// (`sf2_workbook_dir`). That helper takes a `tauri::AppHandle` and is
/// `pub(super)`, so it is not callable from the backup module and could not be
/// called from a plain path-based test. Both sides derive the location from the
/// same `app.path().app_data_dir()`, so the two definitions agree.
pub const SF2_WORKBOOK_DIR_NAME: &str = "sf2-workbooks";

// ── Types ─────────────────────────────────────────────────────────────

/// What a workbook snapshot copied and what it could prove about it.
#[derive(Debug, Clone, Default)]
pub struct WorkbookSnapshot {
    /// One entry per copied workbook, in sorted path order.
    pub entries: Vec<ManifestWorkbook>,
    /// The directory the workbooks were copied from.
    pub source_dir: PathBuf,
    /// True when the source tree was absent (no workbooks have been made yet).
    ///
    /// Distinguishes "the user has no SF2 templates yet" from "the copy silently
    /// found nothing", which is the distinction a restore guard needs.
    pub source_missing: bool,
}

impl WorkbookSnapshot {
    pub fn total_x_count(&self) -> i64 {
        self.entries.iter().map(|entry| entry.x_count).sum()
    }
}

// ── Paths ─────────────────────────────────────────────────────────────

/// The live SF2 working-copy directory for a given app data directory.
pub fn workbook_source_dir(app_dir: &Path) -> PathBuf {
    app_dir.join(SF2_WORKBOOK_DIR_NAME)
}

// ── Snapshot ──────────────────────────────────────────────────────────

/// Copy the whole live `sf2-workbooks` tree into `<backup_folder>/workbooks/`.
///
/// Copies the tree *recursively* and by whole-directory recursion rather than
/// by a hardcoded list of the 12 month files, so it keeps working unchanged
/// after a later phase moves the layout to per-month files plus a `_legacy/`
/// subfolder.
///
/// The copy is all-or-nothing for the folder contents: a failure to copy any
/// file aborts the whole snapshot, so a backup folder never silently claims to
/// hold workbooks it does not. Reading `xCount` is *not* allowed to fail the
/// snapshot — that needs Excel, and Excel may be missing or the file locked.
pub fn snapshot_workbooks_into(app_dir: &Path, backup_folder: &Path) -> Result<WorkbookSnapshot> {
    let source_dir = workbook_source_dir(app_dir);
    let destination_dir = backup_folder.join(WORKBOOK_DIR_NAME);

    if !source_dir.is_dir() {
        log::info!(
            "no SF2 workbook directory at {} — snapshotting an empty workbooks folder",
            source_dir.display()
        );
        fs::create_dir_all(&destination_dir).with_context(|| {
            format!(
                "failed to create workbooks folder {}",
                destination_dir.display()
            )
        })?;
        return Ok(WorkbookSnapshot {
            entries: Vec::new(),
            source_dir,
            source_missing: true,
        });
    }

    let copied = copy_tree(&source_dir, &destination_dir)?;
    let x_counts = read_x_counts(&copied);

    let entries = copied
        .iter()
        .map(|(relative, absolute)| ManifestWorkbook {
            path: format!("{WORKBOOK_DIR_NAME}/{relative}"),
            bytes: fs::metadata(absolute).map(|meta| meta.len()).unwrap_or(0),
            x_count: x_counts.get(relative).copied().unwrap_or(0),
        })
        .collect();

    Ok(WorkbookSnapshot {
        entries,
        source_dir,
        source_missing: false,
    })
}

/// Copy a directory tree, returning every copied file as
/// `(slash-separated path relative to the source root, absolute path)`.
fn copy_tree(source_dir: &Path, destination_dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    fs::create_dir_all(destination_dir).with_context(|| {
        format!(
            "failed to create workbook destination {}",
            destination_dir.display()
        )
    })?;

    let mut copied = Vec::new();
    let mut pending = vec![(
        source_dir.to_path_buf(),
        destination_dir.to_path_buf(),
        String::new(),
    )];

    while let Some((source, destination, prefix)) = pending.pop() {
        let entries = fs::read_dir(&source)
            .with_context(|| format!("failed to read {}", source.display()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("failed to read entries in {}", source.display()))?;

        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            let target = destination.join(&name);

            if entry.file_type()?.is_dir() {
                fs::create_dir_all(&target)
                    .with_context(|| format!("failed to create {}", target.display()))?;
                pending.push((entry.path(), target, relative));
            } else if entry.file_type()?.is_file() {
                fs::copy(entry.path(), &target).with_context(|| {
                    format!(
                        "failed to copy workbook {} -> {}",
                        entry.path().display(),
                        target.display()
                    )
                })?;
                copied.push((relative, target));
            }
        }
    }

    copied.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(copied)
}

// ── xCount ────────────────────────────────────────────────────────────

/// Count the `"X"` cells in each copied workbook.
///
/// Excel does the counting: one `COUNTIF` per day column over the whole learner
/// block, so the work happens inside Excel rather than as one COM round-trip
/// per cell. A workbook that cannot be read (no Excel, file locked, unexpected
/// layout) reports 0 and logs — the backup is still valid, it just cannot
/// prove anything about that file.
fn read_x_counts(copied: &[(String, PathBuf)]) -> BTreeMap<String, i64> {
    let mut counts = BTreeMap::new();
    for (relative, path) in copied {
        match super::x_count::count_x_marks(path) {
            Ok(count) => {
                counts.insert(relative.clone(), count);
            }
            Err(error) => {
                log::warn!(
                    "could not count X marks in {}: {error} (recorded as 0)",
                    path.display()
                );
            }
        }
    }
    counts
}

// ── Restore ───────────────────────────────────────────────────────────

/// Write a backup's `workbooks/` subtree back over the live SF2 workbooks.
///
/// Returns the number of files written, or 0 when the backup carries no
/// workbooks — a legacy flat `*.db` backup, or a folder whose `workbooks/`
/// subtree is empty. The caller must not treat that as a failure: a restore of
/// an old backup simply leaves the live workbooks alone.
pub fn restore_workbooks_from(app_dir: &Path, backup_folder: &Path) -> Result<usize> {
    let source_dir = backup_folder.join(WORKBOOK_DIR_NAME);
    if !source_dir.is_dir() {
        return Ok(0);
    }

    let destination_dir = workbook_source_dir(app_dir);
    let existing = list_files(&source_dir)?;
    if existing.is_empty() {
        return Ok(0);
    }

    let copied = copy_tree(&source_dir, &destination_dir)?;
    Ok(copied.len())
}

pub(crate) fn list_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current)
            .with_context(|| format!("failed to read {}", current.display()))?
        {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                pending.push(entry.path());
            } else if entry.file_type()?.is_file() {
                files.push(entry.path());
            }
        }
    }
    Ok(files)
}

// ── Manifest Helpers ──────────────────────────────────────────────────

/// The manifest entries a backup folder advertises, resolved against the
/// folder. Entries whose file is missing are dropped.
pub fn resolve_manifest_workbooks(
    backup_folder: &Path,
    manifest: &BackupManifest,
) -> Vec<(String, PathBuf, i64)> {
    manifest
        .workbooks
        .iter()
        .filter_map(|entry: &ManifestWorkbook| {
            let absolute = backup_folder.join(&entry.path);
            absolute
                .is_file()
                .then(|| (entry.path.clone(), absolute, entry.x_count))
        })
        .collect()
}
