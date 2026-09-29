//! The process-wide Excel serialisation spec §8.2 assumes and the code did not
//! have.
//!
//! §8.2 requires the startup self-heal to *"not run while another Excel task is in
//! flight — it takes the same `run_excel_task` serialisation as every other COM
//! path."* `run_excel_task` was spawn-and-join with no mutual exclusion anywhere
//! in `excel_com/`, so the serialisation the spec relies on did not exist: the
//! heal, running on a background thread at launch, could open a second Excel
//! session against the same `.xls` while the user was opening or exporting it.
//!
//! In an app whose entire subject is *not losing attendance marks*, two writers
//! on one `.xls` is a corruption class, not a nuisance.
//!
//! ## Policy: exclusive
//!
//! One COM operation at a time, process-wide. Not a reader-writer lock: Excel's
//! automation apartment is single-threaded by construction (`COINIT_APARTMENTTHREADED`),
//! a second automation session against the same file is refused or silently
//! shared at best, and the read paths here (`measure_workbook_marks`,
//! `analyze_workbook`) are already the slow part of a month switch - letting two
//! of them interleave buys nothing and risks a torn read.
//!
//! ## Where it is acquired
//!
//! Inside [`run_excel_task`](crate::sf2::excel::excel_com::com_session::run_excel_task)'s
//! spawned thread, around the task closure, *after* the COM apartment is
//! initialised. That placement is deliberate:
//!
//! * Every COM path in the crate goes through `run_excel_task` - including the
//!   ones that construct a session directly rather than going through
//!   `with_workbook`. A gate on the wrapper is only worth anything if nothing
//!   bypasses the wrapper, so `__tests__/excel_lock_tests.rs` asserts that
//!   exhaustively over the source rather than trusting it.
//! * The gate is *not* held across the `join`. The waiting happens on the spawned
//!   thread, so a caller that loses the race blocks there and not in its own
//!   thread - which is what keeps §8.2's *"startup is not blocked"* true. The
//!   user is never the one waiting; the heal is.
//!
//! ## Deadlock and poisoning
//!
//! * **Nesting** is detected by a thread-local depth counter and re-entrant by
//!   construction: a second acquisition on a thread that is already inside the
//!   critical section runs the section inline instead of blocking, so a task that
//!   calls `run_excel_task` cannot wedge the process against itself. Exclusivity
//!   still holds, because the outer acquisition is holding the gate throughout.
//!   The check is made on the *calling* thread, before the spawn - a spawned
//!   thread does not inherit a thread-local, so checking inside the closure would
//!   always read zero and the nested task would deadlock.
//! * **Poisoning** is recovered from rather than propagated. A task that panics
//!   leaves the data it was holding, not the gate: the gate guards a `()`, and
//!   every workbook is closed by the `Drop` impls on `ExcelSession` and
//!   `WorkbookSession` on the way out. Refusing every later Excel operation
//!   because one task panicked would turn a single failure into a wedged app.

use std::cell::Cell;
use std::sync::{Mutex, MutexGuard};

/// The process-wide gate. One per process, for the reason in the module docs.
static PROCESS_GATE: ExcelGate = ExcelGate::new();

/// An exclusive gate around every Excel COM operation.
///
/// A type rather than a bare `static Mutex<()>` so the *policy* is testable: a
/// process-wide static can only be poisoned or wedged once per test binary, which
/// makes it useless as a subject for both the poisoning test and the concurrency
/// test. Production uses [`PROCESS_GATE`]; the tests build their own.
pub(crate) struct ExcelGate {
    held: Mutex<()>,
}

impl ExcelGate {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            held: Mutex::new(()),
        }
    }

    /// Run `critical_section` with this gate held, and return its result.
    ///
    /// Re-entrant: if the calling thread is already inside a critical section -
    /// of this gate or of the process-wide one, since the depth counter is a
    /// thread-local rather than a per-gate field - the section runs inline and
    /// the gate is not re-acquired. That is what makes nesting safe.
    pub(crate) fn serialise<T>(&self, critical_section: impl FnOnce() -> T) -> T {
        if excel_section_depth() > 0 {
            return enter_section(critical_section);
        }
        let _guard = self.lock();
        enter_section(critical_section)
    }

    /// The guard itself, recovered rather than propagated if a task panicked.
    fn lock(&self) -> MutexGuard<'_, ()> {
        self.held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Run `critical_section` inside the process-wide Excel gate.
///
/// This is the one function every COM path is expected to pass through. It is
/// deliberately free of any Excel or COM reference so that the *serialisation*
/// can be tested on any platform, with no Excel installed.
pub(crate) fn with_excel_serialisation<T>(critical_section: impl FnOnce() -> T) -> T {
    PROCESS_GATE.serialise(critical_section)
}

/// Is the calling thread already inside an Excel critical section?
///
/// Asked by `run_excel_task` *before* it spawns, because that is the only point
/// at which the answer can be true for a nested call.
pub(crate) fn in_excel_section() -> bool {
    excel_section_depth() > 0
}

/// How many critical sections the calling thread is already inside.
fn excel_section_depth() -> u32 {
    SECTION_DEPTH.with(Cell::get)
}

thread_local! {
    /// Set for the duration of a critical section, on the thread running it.
    ///
    /// Thread-local rather than per-gate on purpose: the nesting question is
    /// "is this thread already inside *an* Excel section", and there is exactly
    /// one gate in production. Tests that build their own gate therefore inherit
    /// the same rule, which is the rule they are testing.
    static SECTION_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Mark the calling thread as being inside a section, and undo that on the way
/// out - including on the unwind path, so a panicking task cannot leave a thread
/// permanently exempt from the gate.
fn enter_section<T>(critical_section: impl FnOnce() -> T) -> T {
    SECTION_DEPTH.with(|depth| depth.set(depth.get() + 1));
    let _section = SectionMarker;
    critical_section()
}

struct SectionMarker;

impl Drop for SectionMarker {
    fn drop(&mut self) {
        SECTION_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

#[cfg(test)]
#[path = "__tests__/excel_lock_tests.rs"]
mod tests;
