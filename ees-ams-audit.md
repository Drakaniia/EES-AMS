# Audit Report: ees-ams

**Generated:** 2026-07-25
**Language Stack:** TypeScript (SvelteKit 5) + Rust (Tauri v2) + TailwindCSS 4
**Source Files:** 173 source files (451 total including generated code)
**Backend Tests:** 8 test files (7 SF2 + 1 workbook COM)
**Frontend Tests:** 2 test files

## Executive Summary

The ees-ams codebase is a well-structured Tauri v2 desktop application with clear separation between frontend (SvelteKit 5) and backend (Rust) concerns. The `sf2/` module is the largest subsystem, containing 40% of all backend code, with several significantly oversized files (up to 1548 lines). The frontend side has oversized Svelte route pages (up to 1255 lines) that mix business logic, state management, and UI rendering. The project also has scattered AGENTS.md knowledge base files and an empty `.vibe/` directory. Overall, the architecture is sound — the main opportunities are splitting the largest files and cleaning up documentation placement.

## Severity Legend

- 🔴 **Blocker** — must fix
- 🟡 **Warning** — should fix
- 🟢 **Suggestion** — nice to improve

---

## Issues Found

### Phase 1: Quick Cleanup

_Low effort, safe changes — can be done without touching business logic._

- [ ] **Remove empty `.vibe/` directory** — empty directory at project root.

  ```
  ./
  ├── .vibe/    ← (empty, should be deleted)

  ```

- [ ] **Remove orphaned work-in-progress files** — `docs/working_copy_do_not_commit.xls` and `docs/test.md` appear to be stray files that don't belong in the docs directory.

  ```
  docs/
  ├── working_copy_do_not_commit.xls   ← (remove or move to output/)
  ├── test.md                          ← (remove or rename appropriately)
  ├── DESIGN.md
  ├── GEMINI.md
  ├── RELEASE.md
  ├── readme.md
  ├── TODO.md
  └── superpowers/specs/

  ```

- [ ] **Move sample Excel files into a dedicated `docs/samples/` subdirectory** — `docs/SAMPLE_AUTOMATED SF 2 2025-2026.xls` (note the spaces in the filename — also rename to kebab-case like `sf2-sample-2025-2026.xls`).

  **Before:**
  ```
  docs/
  ├── SAMPLE_AUTOMATED SF 2 2025-2026.xls   ← (spaces in name, inconsistent)
  ```

  **After:**
  ```
  docs/
  ├── samples/
  │   └── sf2-sample-2025-2026.xls
  ```

- [ ] **Standardize `docs/readme.md` naming** — using lowercase `readme.md` alongside `README.md`, `DESIGN.md`, `TODO.md` is inconsistent. Rename to `docs/ARCHITECTURE.md` or `docs/TECHNICAL.md` to clarify its purpose vs. the root `README.md`.

  **Before:** `docs/readme.md` (lowercase, ambiguous name)
  **After:** `docs/ARCHITECTURE.md` or `docs/DEVELOPER.md`

---

### Phase 2: File Refactoring — Backend Rust

_Medium effort — split oversized Rust files and reduce code duplication._

- [ ] **Split `src-tauri/src/sf2/excel_com/workbook.rs`** — 1548 lines (threshold: ≤400)

  This is the largest file in the entire project. It contains:
  - COM infrastructure (`ComObject`, `ComVariant`, `ExcelSession`, `ComApartment`, `WorkbookSession`)
  - Utility functions (`month_number`, `month_name`, `report_year`, `year_from_sheet_name`, etc.)
  - Workbook analysis and metadata operations
  - Roster expansion and row hiding
  - Mark/formula writing operations
  - Batch operations session management

  **Proposed split:**
  ```mermaid
  flowchart LR
    A[workbook.rs<br/>1548 lines] --> B[workbook.rs<br/>~400 lines: high-level API]
    A --> C[workbook_com.rs<br/>~400 lines: COM objects]
    A --> D[workbook_io.rs<br/>~350 lines: read/write marks, formulas]
    A --> E[workbook_ops.rs<br/>~300 lines: expand, hide, batch ops]
  ```

  **Target structure:**
  ```
  src-tauri/src/sf2/excel_com/
  ├── mod.rs              (re-exports from sub-modules)
  ├── workbook.rs         (high-level API: analyze, write_metadata, batch_operations)
  ├── workbook_session.rs (WorkbookSession struct + all session methods)
  ├── workbook_com.rs     (ComObject, ComVariant, ExcelSession, ComApartment)
  └── workbook_utils.rs   (month_number, month_name, report_year, etc.)
  ```

- [ ] **Split `src-tauri/src/sf2/attendance_service.rs`** — 966 lines (threshold: ≤400)

  Contains both the attendance syncing logic and the progress-emitting orchestration. Extract the progress emission and the actual mark-writing logic.

  **Proposed split:**
  ```
  src-tauri/src/sf2/
  ├── attendance_service.rs    (~400 lines: main sync + open logic)
  ├── attendance_sync.rs       (~300 lines: mark-writing for days, template mark generation)
  └── progress.rs              (~100 lines: progress emission)
  ```

- [ ] **Split `src-tauri/src/sf2/excel_service.rs`** — 700 lines (threshold: ≤400)

  Contains Excel-level orchestration. Likely extractable into service initialization + Excel I/O helpers.

- [ ] **Split `src-tauri/src/sf2/roster_parser.rs`** — 695 lines (threshold: ≤400)

  Contains roster parsing logic. Split into parser + mapping logic.

- [ ] **Split `src-tauri/src/sf2/template_ops.rs`** — 668 lines (threshold: ≤400)

  Template operations. Split into template CRUD + template sync.

- [ ] **Split `src-tauri/src/commands/data_transfer.rs`** — 639 lines (threshold: ≤400)

  Command handler for data transfer. Split data transfer commands into smaller command files or extract business logic into the domain/sf2 layer.

- [ ] **Split `src-tauri/src/infrastructure/database/events.rs`** — 604 lines (threshold: ≤400)

  Database event repository. Split into multiple repository files by concern.

- [ ] **Split `src-tauri/src/sf2/roster_sync.rs`** — 470 lines (threshold: ≤400)

  Roster synchronization logic.

- [ ] **Split `src-tauri/src/sf2/calendar.rs`** — 415 lines (threshold: ≤400)

  Calendar utility functions.

---

### Phase 2: File Refactoring — Frontend TypeScript/Svelte

- [ ] **Refactor `src/routes/reports/+page.svelte`** — 1255 lines (threshold: ≤400)

  This is a Svelte route page that's far too large. It should be split into:
  - A page component (`+page.svelte`) — just layout and composition
  - Child components extracted for each section
  - Business logic moved to the state module (`report-state.svelte.ts`)

  **Before:**
  ```mermaid
  flowchart LR
    A[+page.svelte<br/>1255 lines] --> B[UI markup]
    A --> C[State management]
    A --> D[Event handlers]
    A --> E[Data fetching]
  ```

  **After (proposed):**
  ```
  src/routes/reports/
  ├── +page.svelte           (~150 lines: page layout composition)
  ├── ReportTable.svelte     (~300 lines: table rendering)
  ├── ReportFilters.svelte   (~150 lines: filter controls)
  ├── ReportExport.svelte    (~150 lines: export dialog)
  └── report-state.svelte.ts (~400 lines: state + logic)
  ```

- [ ] **Refactor `src/routes/attendance/+page.svelte`** — 803 lines (threshold: ≤400)

  Extract into smaller components:
  ```
  src/routes/attendance/
  ├── +page.svelte           (~150 lines: page layout)
  ├── AttendanceControls.svelte
  ├── AttendanceGrid.svelte
  ├── AttendanceLog.svelte
  └── attendance-state.svelte.ts
  ```

  (Note: Some of these files already exist — good. Reduce the page to pure composition.)

- [ ] **Refactor `src/routes/students/+page.svelte`** — 587 lines (threshold: ≤400)

  Extract into smaller components.

- [ ] **Refactor `src/routes/settings/sf2-state.svelte.ts`** — 457 lines (threshold: ≤400)

  State files can naturally be longer as they contain all the reactive state for a settings page. Consider splitting into `sf2-mappings-state.ts`, `sf2-template-state.ts`, etc.

- [ ] **Refactor `src/lib/components/ui/DateRangePicker.svelte`** — 426 lines (threshold: ≤400)

  This component is just over the threshold. Consider extracting calendar rendering logic.

---

### Phase 2: Test File Note

Test files have high line counts but this is less of a concern:
- `src-tauri/src/sf2/__tests__/attendance_service_tests.rs` — 1175 lines 🟢
- `src-tauri/src/sf2/__tests__/roster_tests.rs` — 931 lines 🟢
- `src-tauri/src/sf2/excel_com/__tests__/workbook_tests.rs` — 550 lines 🟢

These can be left as-is or optionally split by test concern.

---

### Phase 3: Structural Refactoring

_Higher effort — reorganize folders, consolidate naming._

#### 3A: Consolidate AGENTS.md Files

- [ ] **Consolidate AGENTS.md files into `docs/`** — 5 AGENTS.md files scattered:
  ```
  ./
  ├── AGENTS.md                          ← (root - project overview)
  ├── src/AGENTS.md                      ← (frontend conventions)
  ├── src/routes/AGENTS.md               ← (routing conventions)
  ├── src-tauri/AGENTS.md                ← (backend conventions)
  └── src-tauri/src/sf2/AGENTS.md        ← (SF2 module conventions)
  ```

  These serve as AI agent knowledge base files. Consider consolidating into a single `docs/AGENTS.md` or adding a reference table that points to the relevant per-directory docs instead of duplicating metadata.

  **Alternative:** Keep the scattered approach but ensure they are referenced from a central `docs/AGENTS.md` index.

  **Current approach diagram:**
  ```mermaid
  flowchart LR
    A["Root AGENTS.md<br/>(project KB)"] --> B["src/AGENTS.md<br/>(frontend)"]
    A --> C["src/routes/AGENTS.md<br/>(routing)"]
    A --> D["src-tauri/AGENTS.md<br/>(backend)"]
    D --> E["src-tauri/src/sf2/AGENTS.md<br/>(SF2 module)"]
  ```

#### 3B: Directory Organization

- [ ] **Consider moving SQL files from `src-tauri/sql/sf2/` into `src-tauri/src/sf2/`** — 16 SQL files live outside the source tree in a `sql/` directory. Since these are embedded SQL queries (not database migrations), they should live closer to the Rust modules that use them.

  **Before:**
  ```
  src-tauri/
  ├── sql/
  │   └── sf2/           ← 16 SQL migration files
  └── src/
      └── sf2/           ← Rust source code
  ```

  **After (proposed):**
  ```
  src-tauri/src/sf2/
  ├── sql/               ← 16 SQL files co-located with the SF2 module
  │   ├── migrate_to_v9.sql
  │   ├── migrate_to_v12.sql
  │   └── ...
  ├── repository.rs
  ├── service.rs
  └── ...
  ```

  The `sql/sf2/` directory name is also ambiguous — some files are actual migrations (`migrate_to_v16.sql`) while others are query templates (`find_template.sql`, `date_mappings_for_template.sql`). These should be separated.

#### 3C: CSS File Size

- [ ] **Consider splitting `src/app.css`** — 619 lines. While CSS files tend to be larger, this is still worth reviewing. Consider extracting component-specific styles into per-component files and keeping only global tokens/variables in `app.css`.

---

### Phase 3D: Nesting Depth

- [ ] **Flatten `docs/superpowers/specs/`** — 5 levels deep from root (`./docs/superpowers/specs/`). This exceeds the recommended ≤4 levels. Consider:
  - Moving spec files to `docs/specs/` (removing `superpowers/`)
  - Or keeping as-is if `superpowers` has semantic meaning

- [ ] **Monitor `src-tauri/src/sf2/` nesting** — At 4 levels deep (`src-tauri/src/sf2/excel_com/__tests__`), this is at the threshold. Avoid adding deeper nesting.

---

## Refactoring Progress

Use this checklist to track completion. Mark `[x]` when a task is done:

**Phase 1: Quick Cleanup** — `[ ] / 4 completed`
- `[ ]` Remove empty `.vibe/` directory
- `[ ]` Remove/relocate orphaned work-in-progress files (`working_copy_do_not_commit.xls`, `test.md`)
- `[ ]` Move sample Excel files to `docs/samples/`
- `[ ]` Standardize `docs/readme.md` naming

**Phase 2: File Refactoring** — `[ ] / 14 completed`
- `[ ]` Split `workbook.rs` (1548 lines)
- `[ ]` Split `attendance_service.rs` (966 lines)
- `[ ]` Split `excel_service.rs` (700 lines)
- `[ ]` Split `roster_parser.rs` (695 lines)
- `[ ]` Split `template_ops.rs` (668 lines)
- `[ ]` Split `data_transfer.rs` (639 lines)
- `[ ]` Split `events.rs` (604 lines)
- `[ ]` Split `roster_sync.rs` (470 lines)
- `[ ]` Split `calendar.rs` (415 lines)
- `[ ]` Refactor `+page.svelte` in reports (1255 lines)
- `[ ]` Refactor `+page.svelte` in attendance (803 lines)
- `[ ]` Refactor `+page.svelte` in students (587 lines)
- `[ ]` Refactor `sf2-state.svelte.ts` (457 lines)
- `[ ]` Refactor `DateRangePicker.svelte` (426 lines)

**Phase 3: Structural Refactoring** — `[ ] / 4 completed`
- `[ ]` Consolidate AGENTS.md files
- `[ ]` Move SQL files into source tree
- `[ ]` Review/refactor `app.css` (619 lines)
- `[ ]` Address nesting depth in `docs/superpowers/specs/`

---

## Key Structural Statistics

| Metric | Measured | Threshold | Status |
|--------|----------|-----------|--------|
| Total source files | 173 | — | 🟢 |
| Dir with most files | `src-tauri/src/sf2` (21 files) | ≤30 | 🟢 OK |
| Largest file | `workbook.rs` (1548 lines) | ≤400 | 🔴 EXCEEDS |
| Files >400 lines (source) | 19 | — | 🔴 19 offenders |
| Max nesting depth (src/) | 3 levels | ≤4 | 🟢 OK |
| Max nesting depth (src-tauri/) | 4 levels | ≤4 | 🟡 At threshold |
| Max nesting depth (docs/) | 5 levels | ≤4 | 🟡 EXCEEDS |
| Empty directories | 1 (`.vibe/`) | 0 | 🟢 Suggestion |
| Test files (frontend) | 2 | — | 🟢 Low count |
| Test files (backend) | 8 | — | 🟢 |
| MD files in docs/ | 10 | — | 🟢 Well organized |
| AGENTS.md files | 5 (scattered) | — | 🟢 Suggestion |

---

## Recommendations Summary by Effort

### Low Effort (Phase 1)
1. Delete `.vibe/` empty directory
2. Remove/relocate `docs/working_copy_do_not_commit.xls` and `docs/test.md`
3. Move sample XLS to `docs/samples/` with proper naming
4. Rename `docs/readme.md` to clarify purpose

### Medium Effort (Phase 2)
5. Split 10 oversized Rust files (starting with the biggest: `workbook.rs` at 1548 lines)
6. Split 3 oversized Svelte route pages
7. Split the largest state file (`sf2-state.svelte.ts` at 457 lines)
8. Reduce `DateRangePicker.svelte` or `app.css`

### Higher Effort (Phase 3)
9. Consolidate AGENTS.md knowledge base
10. Move `sql/sf2/` into source tree
11. Address nesting depth in `docs/superpowers/specs/`

---

> **⚠️ Before implementing any refactoring that splits or moves code, write tests for existing behavior first, then refactor. This ensures refactored code preserves all functionality and catches regressions early.**

> **⚠️ The `sf2/` module is the most complex subsystem. Splitting these files should be done carefully, with particular attention to the COM automation and Excel I/O code paths, which are hard to test in CI.**

