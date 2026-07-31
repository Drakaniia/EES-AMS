# Audit Remaining Refactoring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete remaining 8 items from the ees-ams audit — split 6 oversized Rust files, reduce reports/+page.svelte, and refactor app.css.

**Architecture:** Pure refactoring — no functional changes. Each task extracts code from an oversized file into new/more focused files, updates imports in mod.rs, and verifies the build.

**Tech Stack:** Rust (Tauri v2), SvelteKit 5, TailwindCSS 4

## Global Constraints

- Zero functional changes — these are pure extractions/refactors
- Build must pass: `cargo build` and `npm run build` (or `bun run build`)
- All existing tests must pass: `cargo test` and `bun test`
- Follow existing codebase patterns (error handling with `Result`, imports via `crate::`, etc.)
- No new dependencies
- Each task is independent — fan out in parallel

---

### Task 1: Split `workbook.rs` (925 → ≤400 lines)

**Files:**

- Modify: `src-tauri/src/sf2/excel_com/workbook.rs`
- Create: `src-tauri/src/sf2/excel_com/workbook_analysis.rs` — analyze() method
- Create: `src-tauri/src/sf2/excel_com/workbook_io.rs` — write_marks, write_marks_force, write_formulas, write_metadata
- Create: `src-tauri/src/sf2/excel_com/workbook_ops.rs` — expand_roster_rows, hide_empty_learner_rows, batch_operations
- Modify: `src-tauri/src/sf2/excel_com/mod.rs` — add new modules

**Interfaces:**

- Consumes: `WorkbookSession` struct (stays in workbook.rs), `ComObject`, `ExcelSession`, helpers from `workbook_utils.rs`
- Produces: `WorkbookSession` methods moved to new files via `impl` blocks in the same crate

- [ ] **Read current workbook.rs** — understand all method implementations
- [ ] **Create workbook_analysis.rs** — extract analyze(), helper types
- [ ] **Create workbook_io.rs** — extract write_marks, write_marks_force, write_formulas, write_metadata
- [ ] **Create workbook_ops.rs** — extract expand_roster_rows, hide_empty_learner_rows, batch_operations
- [ ] **Update mod.rs** — add `pub mod workbook_analysis;`, `pub mod workbook_io;`, `pub mod workbook_ops;`
- [ ] **Trim workbook.rs** — keep WorkbookSession struct, open/close/save/calculate methods
- [ ] **Build check**: `cd src-tauri && cargo build`

---

### Task 2: Split `attendance_service.rs` (503 → ≤400 lines)

**Files:**

- Modify: `src-tauri/src/sf2/attendance_service.rs`
- Modify: `src-tauri/src/sf2/mod.rs`

**Interfaces:**

- Consumes: Sf2Repository, excel helpers, progress types
- Produces: smaller attendance_service.rs

- [ ] **Read attendance_service.rs** — understand current structure (503 lines)
- [ ] **Extract progress emission** (`emit_sf2_progress`) into dedicated file or keep inline if small
- [ ] **Extract sync helpers** — any non-core sync logic to `attendance_sync.rs` or `attendance_events.rs`/`attendance_marks.rs`
- [ ] **Update mod.rs** — ensure new modules registered
- [ ] **Build check**: `cd src-tauri && cargo build`

---

### Task 3: Split `excel_service.rs` (455 → ≤400 lines)

**Files:**

- Modify: `src-tauri/src/sf2/excel_service.rs`
- Modify: `src-tauri/src/sf2/mod.rs`

- [ ] **Read excel_service.rs** — understand current structure
- [ ] **Extract init/helper code** into existing helper files or new thin module
- [ ] **Update mod.rs**
- [ ] **Build check**: `cd src-tauri && cargo build`

---

### Task 4: Split `roster_parser.rs` (484 → ≤400 lines)

**Files:**

- Modify: `src-tauri/src/sf2/roster_parser.rs`
- Modify: `src-tauri/src/sf2/mod.rs`

- [ ] **Read roster_parser.rs** — identify extractable mapping/data logic
- [ ] **Extract utility/helper code** into new or existing module
- [ ] **Update mod.rs**
- [ ] **Build check**: `cd src-tauri && cargo build`

---

### Task 5: Split `data_transfer.rs` (436 → ≤400 lines)

**Files:**

- Modify: `src-tauri/src/commands/data_transfer.rs`
- Consider: extracting business logic into `src-tauri/src/sf2/` domain layer

- [ ] **Read data_transfer.rs** — understand structure
- [ ] **Extract thin helpers** into inline helpers or separate module
- [ ] **Build check**: `cd src-tauri && cargo build`

---

### Task 6: Split `roster_sync.rs` (470 → ≤400 lines)

**Files:**

- Modify: `src-tauri/src/sf2/roster_sync.rs`
- Modify: `src-tauri/src/sf2/mod.rs`

- [ ] **Read roster_sync.rs** — identify extractable logic (learner sync already in `roster_sync_learner.rs`)
- [ ] **Extract remaining helpers**
- [ ] **Update mod.rs**
- [ ] **Build check**: `cd src-tauri && cargo build`

---

### Task 7: Reduce `reports/+page.svelte` (534 → ≤400 lines)

**Files:**

- Modify: `src/routes/reports/+page.svelte`
- Potentially: create new child components

- [ ] **Read reports/+page.svelte** — identify remaining inline markup/logic (already has 14 component files)
- [ ] **Extract remaining sections** into existing or new components
- [ ] **Build check**: `cd src-tauri && bun run build` (or npm run build)

---

### Task 8: Refactor `app.css` (619 lines)

**Files:**

- Modify: `src/app.css`
- Potentially: create `src/lib/components/*/*.css` per-component files

- [ ] **Read app.css** — identify global tokens vs component-specific styles
- [ ] **Extract component styles** into per-component `<style>` blocks or CSS files
- [ ] **Keep in app.css** only global tokens/variables and base resets
- [ ] **Build check**: `cd src-tauri && bun run build`
