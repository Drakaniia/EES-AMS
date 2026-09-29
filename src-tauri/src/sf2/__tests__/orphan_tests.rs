//! The project convention forbids orphan code, and the SF2 module is where it
//! collects: four phases of extraction left behind whole functions whose last
//! caller was replaced by the guard, the per-month model, or the differential
//! clear.
//!
//! A `pub fn` with no caller is not a warning in Rust. It compiles, it costs
//! nothing at runtime, and it is the single most dangerous thing left in a
//! codebase whose subject is not destroying data - because the next person to
//! reach for it finds a working, documented, tempting function. `clear_attendance_grid`
//! is the example that matters: it blanks the whole 33-column attendance range,
//! which is precisely the primitive spec §9.2 replaced with a differential
//! clear. Anything that calls it again re-arms the original bug.
//!
//! So this is a test, not a review note. A dead function is exactly the kind of
//! thing an agent under time pressure adds back, and the cost of it landing is
//! a term of X marks.

use std::path::{Path, PathBuf};

/// Symbols that must not exist anywhere in the SF2 module, and why each one had
/// to go.
const DELETED: [(&str, &str); 4] = [
    (
        "clear_attendance_grid",
        "blanks the whole 33-column attendance range; superseded by \
         `attendance_marks::differential_clear_marks` (spec §9.2), which can only ever blank an \
         `X` the database no longer proves",
    ),
    (
        "clear_attendance_marks_for_records",
        "the pre-guard whole-block clear; superseded by the differential clear",
    ),
    (
        "latest_event_timestamp",
        "existed only to feed the deleted 'has an event landed since the last sync?' shortcut, a \
         row count standing in for evidence. The §9.1 guard reads the workbook instead",
    ),
    (
        "set_report_month",
        "the Excel month switch, deleted with the overlay it existed to cover (spec §7.2). A \
         month switch is now one SQL read, and this method would have moved a month in the \
         database without touching the file - two months disagreeing about which one is current",
    ),
];

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

/// This file has to name the deleted symbols in order to assert they are gone.
fn this_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("sf2")
        .join("__tests__")
        .join("orphan_tests.rs")
}

#[test]
fn the_deleted_sf2_symbols_are_gone_from_the_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let this_file = this_file();
    let sources: Vec<PathBuf> = rust_sources(&root)
        .into_iter()
        .filter(|path| path != &this_file)
        .collect();
    assert!(
        !sources.is_empty(),
        "the source walk found nothing; the test is not testing anything"
    );

    for (symbol, why) in DELETED {
        // A definition or a call, not a mention. Comments and string literals
        // that say "this is gone" are the documentation of the deletion and have
        // to be allowed to name it - several of them do, on purpose.
        let needles = [format!("fn {symbol}"), format!("{symbol}(")];
        let mut sites = Vec::new();
        for path in &sources {
            let Ok(code) = std::fs::read_to_string(path) else {
                continue;
            };
            for (number, line) in code.lines().enumerate() {
                let code_line = line.split("//").next().unwrap_or(line);
                if needles.iter().any(|needle| code_line.contains(needle)) {
                    sites.push(format!(
                        "{}:{}: {}",
                        path.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
        assert!(
            sites.is_empty(),
            "`{symbol}` is deleted because {why}, but it is still defined or called at:\n{}\n\
             The project convention forbids orphan code, and a reachable copy of it is worse than \
             a dead one.",
            sites.join("\n")
        );
    }
}
