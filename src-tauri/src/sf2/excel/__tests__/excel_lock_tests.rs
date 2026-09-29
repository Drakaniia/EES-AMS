//! Tests for the process-wide Excel serialisation (spec §8.2).
//!
//! None of them needs Excel, and none of them needs Windows. That is the point of
//! factoring the critical section into a closure over an [`ExcelGate`]: a
//! concurrency fix whose only test needs a licensed copy of Excel is a concurrency
//! fix nobody runs.
//!
//! * **Behavioural.** Exclusivity, poison recovery, and the nesting rule, driven
//!   through real threads against a locally built gate so the process-wide one is
//!   never poisoned or wedged by a test.
//! * **Structural.** The property a runtime test *cannot* reach: that no code in
//!   the crate opens a workbook without going through `run_excel_task`. A gate on
//!   a wrapper is worth exactly as much as the number of callers that skip it, so
//!   this is asserted over the source of every `.rs` file in the crate.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

/// This file, which names the forbidden constructs as string literals and would
/// otherwise be its own worst finding.
const THIS_FILE: &str = "excel_lock_tests.rs";

// ── Behavioural: the gate ───────────────────────────────────────────────────

#[test]
fn two_sections_never_overlap() {
    // The property §8.2 asks for, stated as a counter: `inside` is incremented on
    // entry and decremented on exit, and every increment must find it at zero. A
    // second COM operation arriving during the first finds it at one, and the
    // thread that is supposed to be waiting fails instead of proceeding.
    const THREADS: usize = 6;
    const ROUNDS: usize = 40;

    let gate = ExcelGate::new();
    let inside = AtomicUsize::new(0);
    let entries = AtomicUsize::new(0);

    std::thread::scope(|scope| {
        for _ in 0..THREADS {
            scope.spawn(|| {
                for _ in 0..ROUNDS {
                    gate.serialise(|| {
                        assert_eq!(
                            inside.fetch_add(1, Ordering::SeqCst),
                            0,
                            "two Excel sections were inside the gate at once"
                        );
                        entries.fetch_add(1, Ordering::SeqCst);
                        // Long enough that an unsynchronised implementation has a
                        // real chance to collide: without the gate, six threads
                        // doing 40 short rounds overlap almost immediately.
                        std::thread::sleep(Duration::from_micros(200));
                        inside.fetch_sub(1, Ordering::SeqCst);
                    });
                }
            });
        }
    });

    assert_eq!(
        entries.load(Ordering::SeqCst),
        THREADS * ROUNDS,
        "every section must still run: serialisation defers work, it does not drop it"
    );
    assert_eq!(inside.load(Ordering::SeqCst), 0);
}

#[test]
fn a_section_runs_start_to_finish_before_the_next_one_begins() {
    // Exclusivity as an ordering rather than as a counter, because a counter can
    // be satisfied by a lock that is taken and dropped in the wrong places. Every
    // entry must be immediately followed by that same section's exit.
    const THREADS: usize = 4;
    const ROUNDS: usize = 15;

    let gate = ExcelGate::new();
    let log: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

    let gate = &gate;
    let log = &log;
    std::thread::scope(|scope| {
        for thread in 0..THREADS {
            scope.spawn(move || {
                for round in 0..ROUNDS {
                    let id = format!("{thread}-{round}");
                    gate.serialise(|| {
                        log.lock().expect("log").push(format!("enter {id}"));
                        std::thread::sleep(Duration::from_micros(100));
                        log.lock().expect("log").push(format!("exit {id}"));
                    });
                }
            });
        }
    });

    let log = log.lock().expect("log").clone();
    assert_eq!(log.len(), THREADS * ROUNDS * 2);
    for pair in log.chunks(2) {
        let (enter, exit) = (&pair[0], &pair[1]);
        assert_eq!(
            enter.strip_prefix("enter "),
            exit.strip_prefix("exit "),
            "sections interleaved: {enter} was followed by {exit}"
        );
    }
}

#[test]
fn the_process_gate_is_the_one_every_com_path_shares() {
    // The same exclusivity, through the exact function `run_excel_task` calls -
    // so the production entry point is exercised, not only the type behind it.
    const THREADS: usize = 4;

    let inside = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..THREADS {
            scope.spawn(|| {
                for _ in 0..25 {
                    with_excel_serialisation(|| {
                        assert_eq!(
                            inside.fetch_add(1, Ordering::SeqCst),
                            0,
                            "two sections were inside the process-wide gate at once"
                        );
                        std::thread::sleep(Duration::from_micros(150));
                        inside.fetch_sub(1, Ordering::SeqCst);
                    });
                }
            });
        }
    });
    assert_eq!(inside.load(Ordering::SeqCst), 0);
}

#[test]
fn a_poisoned_gate_does_not_wedge_the_process() {
    // A task that panics leaves the data it was holding, not the gate: the gate
    // guards a `()`, and both session types close their workbook in `Drop` on the
    // way out. Refusing every later Excel operation because one task panicked
    // would turn a single failure into an app that can no longer open a workbook.
    let gate = ExcelGate::new();

    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        gate.serialise(|| panic!("a COM task died mid-section"));
    }));
    assert!(panicked.is_err(), "the section was supposed to panic");

    assert_eq!(
        gate.serialise(|| 41 + 1),
        42,
        "a poisoned gate must still admit the next Excel operation"
    );
}

#[test]
fn a_nested_section_does_not_deadlock() {
    // The nesting rule, asserted by *finishing*: a task that reaches Excel again
    // from inside a section would block on a gate its own thread already holds.
    // The watchdog is what turns "this would have hung" into a test failure rather
    // than a hung test binary.
    let gate = ExcelGate::new();
    let (done, finished) = mpsc::channel();

    let worker = std::thread::spawn(move || {
        let value = gate.serialise(|| gate.serialise(|| "inner ran under the outer lock"));
        done.send(value).expect("the outer section completed");
    });

    let value = finished
        .recv_timeout(Duration::from_secs(10))
        .expect("a nested section must not deadlock against its own thread");
    assert_eq!(value, "inner ran under the outer lock");
    worker.join().expect("worker");
}

#[test]
fn a_nested_section_still_holds_the_gate_against_other_threads() {
    // Re-entrancy must not become "no exclusion": the inline nested section runs
    // on a thread that is already holding the gate, and a *different* thread
    // asking for the gate during that window is still made to wait. Both halves
    // are asserted by the same thread that holds the gate, so there is no race
    // in the test itself.
    let gate = ExcelGate::new();
    let barged_in = AtomicUsize::new(0);

    let gate_ref = &gate;
    let barged_ref = &barged_in;
    std::thread::scope(|scope| {
        gate_ref.serialise(|| {
            scope.spawn(|| gate_ref.serialise(|| barged_ref.fetch_add(1, Ordering::SeqCst)));
            // Long enough for the other thread to have got in if the gate were
            // not held. A false pass weakens the test; it cannot fail spuriously.
            std::thread::sleep(Duration::from_millis(150));
            assert_eq!(
                barged_ref.load(Ordering::SeqCst),
                0,
                "another thread got into the gate while this one held it"
            );
            // The nested section runs inline rather than blocking.
            gate_ref.serialise(|| ());
        });
    });
    assert_eq!(
        barged_in.load(Ordering::SeqCst),
        1,
        "the outside section runs once the holder is done"
    );
}

#[test]
fn a_section_depth_is_not_left_behind_by_a_panicking_task() {
    // The exemption is a thread-local, so a task that unwound out of a section
    // without clearing it would leave that thread permanently outside the gate -
    // a silent hole in the one property this module exists for.
    let gate = ExcelGate::new();
    let worker = std::thread::spawn(move || {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            gate.serialise(|| gate.serialise(|| panic!("inner section died")));
        }));
        in_excel_section()
    });

    assert!(
        !worker.join().expect("worker"),
        "the depth counter survived a panicking section"
    );
}

// ── Structural: nothing opens a workbook without going through the gate ──────

/// `com_session.rs`, where the gate is actually taken.
const COM_SESSION_SOURCE: &str = include_str!("../excel_com/com_session.rs");

/// Every way the crate gets a COM session or opens a file through it.
///
/// The audit this is derived from: `ExcelSession::new` and
/// `ComObject::excel_application` create the automation object;
/// `get_object("Workbooks")` with `method_object("Open"/"Add")` is the file open
/// itself. All of them are reached either from a `run_excel_task` closure or from
/// a method on a session type - which only ever runs inside one.
const WORKBOOK_CONSTRUCTION: [&str; 6] = [
    "ExcelSession::new(",
    "ComObject::excel_application(",
    "get_object(\"Workbooks\")",
    "method_object(\"Open\"",
    "method_object(\"Add\"",
    ".open_workbook(",
];

/// The free functions that build a session or open a workbook without going
/// through `run_excel_task`, because they *are* the inside of one.
///
/// Their call sites are audited separately, by
/// `every_in_lock_helper_is_only_called_inside_a_section` - a helper listed here
/// is only safe while nothing calls it from outside.
const IN_LOCK_HELPERS: [&str; 2] = ["with_workbook", "build_in_session"];

#[test]
fn the_gate_is_taken_by_the_one_wrapper_every_com_path_uses() {
    let code = code_only(COM_SESSION_SOURCE);
    assert!(
        code.contains("with_excel_serialisation"),
        "`run_excel_task` must pass its task through the process-wide gate: it is \
         the only place the spec's serialisation can be enforced for every path"
    );
    assert!(
        code.contains("in_excel_section()"),
        "`run_excel_task` must recognise a nested call before it spawns, or a \
         nested task deadlocks on a gate its own thread is holding"
    );
}

#[test]
fn no_code_opens_a_workbook_outside_a_serialised_section() {
    // The audit, as a test. "There is a mutex in excel_com" is worth exactly as
    // much as the number of callers that skip it, and the only way to know that
    // about a codebase is to walk it.
    let mut checked = 0_usize;
    for (path, source) in crate_sources() {
        if path.ends_with(THIS_FILE) {
            continue;
        }
        for (line_number, line) in code_lines(&source) {
            let Some(construction) = WORKBOOK_CONSTRUCTION
                .iter()
                .find(|token| line.contains(**token))
            else {
                continue;
            };
            let scope = enclosing_scope(&source, line_number).unwrap_or_else(|| {
                panic!("`{construction}` appears in {path}:{line_number} outside any item")
            });
            let inside_a_section = match &scope {
                Scope::Impl(_) => true,
                Scope::Function { name, .. } => {
                    IN_LOCK_HELPERS.contains(&name.as_str())
                        || enclosing_body(&source, line_number).contains("run_excel_task")
                }
                Scope::Other => false,
            };
            assert!(
                inside_a_section,
                "`{construction}` in {path}:{line_number} is in {scope}, which does \
                 not go through `run_excel_task` and therefore runs outside the \
                 process-wide Excel gate"
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "the audit found no workbook construction at all, so it is not looking at \
         the code it thinks it is"
    );
}

#[test]
fn every_in_lock_helper_is_only_called_inside_a_section() {
    // The other half of the allowlist above: a helper listed as "in-lock" is only
    // safe while nothing calls it from outside. Checked at the call sites, so
    // adding one is a test failure rather than a comment that has gone stale.
    for helper in IN_LOCK_HELPERS {
        let mut called = 0_usize;
        for (path, source) in crate_sources() {
            if path.ends_with(THIS_FILE) {
                continue;
            }
            for (line_number, line) in code_lines(&source) {
                if !is_call_of(&line, helper) {
                    continue;
                }
                let body = enclosing_body(&source, line_number);
                assert!(
                    body.contains("run_excel_task"),
                    "`{helper}` is called in {path}:{line_number} from code that \
                     does not go through `run_excel_task`, so it runs outside the gate"
                );
                called += 1;
            }
        }
        assert!(
            called > 0,
            "no call site for `{helper}` was found - has it moved?"
        );
    }
}

#[test]
fn the_self_heal_reaches_excel_only_through_a_path_that_holds_the_gate() {
    // The one caller the whole change exists for. `heal.rs` may not name
    // `run_excel_task` at all - it reaches Excel through the guard's single
    // scanner, which is itself a `run_excel_task` path. This asserts the scanner
    // is still that one, so §8.2's "the same serialisation as every other COM
    // path" holds for the heal by construction rather than by inspection.
    let heal = code_only(include_str!("../../heal.rs"));
    assert!(
        heal.contains("measure_workbook_marks"),
        "the self-heal must still measure through the guard's single scanner"
    );
    for forbidden in ["run_excel_task", "with_workbook", "ExcelSession"] {
        assert!(
            !heal.contains(forbidden),
            "the self-heal must not name `{forbidden}`: a second reader of the \
             workbook is how \"what we measured\" and \"what we may clear\" stop \
             being the same set"
        );
    }
}

// ── Source-walking helpers ──────────────────────────────────────────────────

enum Scope {
    /// An `impl` block. A method on a session type only runs when a caller
    /// already holds the gate, so anything in here is inside a section.
    Impl(String),
    Function {
        name: String,
        start: usize,
    },
    Other,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Impl(name) => write!(formatter, "impl {name}"),
            Self::Function { name, .. } => write!(formatter, "fn `{name}`"),
            Self::Other => write!(formatter, "no enclosing item"),
        }
    }
}

/// Drop `//` comments and blank lines, keeping the rest of each line.
///
/// The limitation is the codebase's established one: a `//` inside a string
/// literal reads as a comment. Nothing checked here contains one, and a false
/// negative is a weakened test rather than a wrong pass.
fn code_only(source: &str) -> String {
    code_lines(source)
        .into_iter()
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn code_lines(source: &str) -> Vec<(usize, String)> {
    source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let code = match line.find("//") {
                Some(at) => &line[..at],
                None => line,
            };
            if code.trim().is_empty() {
                None
            } else {
                Some((index + 1, code.to_string()))
            }
        })
        .collect()
}

/// Every `.rs` file in the crate, as `(path, source)`.
fn crate_sources() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    let mut pending = vec![root];
    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
        for entry in entries {
            let entry = entry.expect("directory entry");
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let source = std::fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
                found.push((path.display().to_string(), source));
            }
        }
    }
    assert!(
        found.len() > 50,
        "the audit walked only {} files - it is not looking at the crate",
        found.len()
    );
    found
}

/// Lines that continue a signature started on an earlier line. They sit at
/// column 0 and are not items, so the walk has to step over them to reach the
/// function header they belong to.
const SIGNATURE_CONTINUATION: [&str; 5] = [")", ",", "where", "->", "{"];

/// The item whose body contains `line_number`: the nearest line at column 0 that
/// is neither blank, nor an attribute, nor a continuation of an earlier
/// signature.
fn enclosing_scope(source: &str, line_number: usize) -> Option<Scope> {
    let lines: Vec<&str> = source.lines().collect();
    for index in (0..line_number.min(lines.len())).rev() {
        let line = lines[index];
        if !line.starts_with(|character: char| !character.is_whitespace()) {
            continue;
        }
        if line.starts_with('#') || line.trim_start().starts_with("//") {
            continue;
        }
        if SIGNATURE_CONTINUATION
            .iter()
            .any(|prefix| line.starts_with(prefix))
        {
            continue;
        }
        if let Some(name) = line.strip_prefix("impl") {
            return Some(Scope::Impl(name.trim_end_matches('{').trim().to_string()));
        }
        if let Some(name) = function_name(line) {
            return Some(Scope::Function {
                name,
                start: index + 1,
            });
        }
        return Some(Scope::Other);
    }
    None
}

/// The source of the enclosing function, so a call can be checked against the
/// function it is in.
fn enclosing_body(source: &str, line_number: usize) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let Some(Scope::Function { start, .. }) = enclosing_scope(source, line_number) else {
        return String::new();
    };
    let end = (start..lines.len())
        .find(|index| is_item_header(lines[*index]) && *index + 1 > start)
        .unwrap_or(lines.len());
    lines[start - 1..end].join("\n")
}

/// The function's own name, if this line starts one.
fn function_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("pub ")
        .or_else(|| trimmed.strip_prefix("pub(crate) "))
        .unwrap_or(trimmed);
    let rest = rest
        .strip_prefix("async ")
        .or_else(|| rest.strip_prefix("unsafe "))
        .unwrap_or(rest);
    let after = rest.strip_prefix("fn ")?;
    let name: String = after
        .chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

fn is_item_header(line: &str) -> bool {
    if !line.starts_with(|character: char| !character.is_whitespace()) {
        return false;
    }
    line.starts_with("#[") || line.starts_with('#') || function_name(line).is_some()
}

/// Is this line a *call* of `helper`, rather than its definition or an import of
/// it? Comments and prose are already gone by the time this sees a line.
fn is_call_of(line: &str, helper: &str) -> bool {
    let Some(column) = line.find(helper) else {
        return false;
    };
    let before = line[..column].trim_end();
    let after = line[column + helper.len()..].trim_start();
    let is_a_definition = before == "fn" || before.ends_with("fn ");
    !is_a_definition && after.starts_with('(')
}
