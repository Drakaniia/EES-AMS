use std::path::{Path, PathBuf};

/// Every `.rs` file under the crate's `src`, so the assertion below is over the
/// whole tree rather than over the files that used to name the deleted shortcut.
fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn crate_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

// ── acceptance #19 ───────────────────────────────────────────────────

/// Spec §7.2 / acceptance #19: the "has anything changed since the last sync?"
/// helper and its `None`/`None` heuristic are gone.
///
/// The heuristic reported "in sync" for a database with no attendance events and
/// no `last_synced_at`, and "clear it back to agreement" for a database that had
/// events but nothing newer than the last sync. Both readings are row counts, and
/// a row count is not evidence: they are what let spec §4's chain reach a total
/// clear. The §9.1 `SyncPermit` guard replaced them - `Proven` means the database
/// holds every `X` the workbook shows, and anything less is `Stale` or
/// `Unmeasured`, neither of which writes. There is no count-based shortcut left
/// to reintroduce, so the name must not come back.
///
/// This is a grep, deliberately, and it is a grep over the *whole tree including
/// comments* - the acceptance criterion is `grep -rn ... src-tauri/` returning
/// nothing, and leaving the name in a doc comment would be a way of keeping the
/// greppable form alive next to a re-introduction. A function this easy to
/// re-add is one an agent optimising for speed will reach for, and a
/// compile-time check cannot see it: the function compiles perfectly well, it is
/// just wrong.
///
/// This test does not name the symbol either. A test that has to grep for it
/// would be the one place still naming it.
#[test]
fn the_deleted_sync_shortcut_is_gone_from_the_whole_crate() {
    let needle = ["attendance", "changed", "since"].concat();
    let sources = rust_sources(&crate_src());
    assert!(
        !sources.is_empty(),
        "the source walk found nothing; the test is not testing anything"
    );

    let mut offenders = Vec::new();
    for path in &sources {
        let Ok(code) = std::fs::read_to_string(path) else {
            continue;
        };
        for (number, line) in code.lines().enumerate() {
            if line.contains(&needle) {
                offenders.push(format!(
                    "{}:{}: {}",
                    path.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "the last-sync shortcut is deleted (spec acceptance #19), but its name is still in:\n{}",
        offenders.join("\n")
    );
}
