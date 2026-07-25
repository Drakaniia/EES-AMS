# AGENTS.md — Project Knowledge Base

**Generated:** 2026-07-25
**Branch:** main

This is the single authoritative AGENTS.md for the EES-AMS project. It consolidates all knowledge previously scattered across 6 per-directory files.

---

## 1. PROJECT OVERVIEW

EES-AMS: cross-platform Tauri v2 desktop app for elementary school attendance management with ID card reader support. SvelteKit 5 + TypeScript + TailwindCSS 4 frontend, Rust with rusqlite + r2d2 backend, SQLite database. Offline-first, Windows-primary.

### Key Facts
- Card reader = USB HID keyboard wedge mode (emulated keystrokes)
- No auth — single-teacher desktop use case
- Windows NSIS installer via Tauri bundler
- Tauri updater plugin for in-app update notifications
- Google Drive backup uses `tauri-plugin-oauth` for token exchange

---

## 2. PROJECT STRUCTURE

```
ees_ams/
├── src/                  # SvelteKit 5 frontend
├── src-tauri/            # Rust/Tauri v2 backend (crate)
├── docs/                 # Documentation, DESIGN.md
├── static/               # Self-hosted fonts + robots.txt
├── build/                # Build artifacts (generated)
├── output/               # Exports (JSON, CSV, DB backups)
└── .agents/              # AI agent skills
```

---

## 3. WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Frontend pages | `src/routes/` | SvelteKit file-based routing per feature |
| Shared UI primitives | `src/lib/components/ui/` | Dialog, Toast, Pagination, etc. |
| Tauri command wrappers | `src/lib/db-rust/` | Frontend→Rust invoke bridge |
| Feature business logic | `src/lib/features/` | Per-feature TS workflows (only settings currently) |
| Types | `src/lib/types.ts` | Shared TS interfaces |
| Rust Tauri commands | `src-tauri/src/commands/` | `#[tauri::command]` handlers |
| Domain models + schema | `src-tauri/src/domain/` | Business logic, DB schema, migrations |
| Database repos | `src-tauri/src/infrastructure/database/` | Pool + repository pattern |
| SF2 Excel automation | `src-tauri/src/sf2/` | COM-based Excel workbook I/O |
| Backup system | `src-tauri/src/backup/` | Local + Google Drive sync |
| Shared Rust utilities | `src-tauri/src/shared/` | Cross-cutting helpers |

---

## 4. CODE MAP

| Symbol | Type | Location | Role |
|--------|------|----------|------|
| `app_lib::run` | fn | `src-tauri/src/lib.rs` | Tauri app bootstrap |
| `commands::*` | fns | `src-tauri/src/commands/` | All Tauri command handlers |
| `domain::models` | types | `src-tauri/src/domain/models.rs` | Core domain types (StudentId, EventId, models) |
| `infrastructure::database` | module | `src-tauri/src/infrastructure/database/` | Pool managers + repos |
| `sf2::*` | module | `src-tauri/src/sf2/` | Excel import/export |
| `backup::*` | module | `src-tauri/src/backup/` | Backup + restore logic |
| `db-rust/*` | TS modules | `src/lib/db-rust/` | Frontend→Rust API layer |
| `features/settings/*` | TS modules | `src/lib/features/settings/` | Settings workflow logic |

---

## 5. COMMANDS

```bash
# Frontend
bun install              # Install JS deps
bun run dev              # Vite dev server
bun run check            # svelte-check + type checking
bun run lint             # Prettier + ESLint
bun run format           # Prettier write
bun test                 # Vitest

# Tauri
bun run tauri dev        # Full dev (frontend + backend)
bun run tauri build      # Production build (NSIS on Windows)

# Rust (from src-tauri/)
cargo check              # Type check
cargo clippy             # Lint
cargo fmt --check        # Format check
cargo test               # Backend tests
cargo build --release    # Release build
```

---

## 6. CONVENTIONS

### 6.1 Project-Wide

- **Svelte 5 Runes only**: `$state`, `$derived`, `$effect` — no Svelte 4 stores
- **Dual data layer**: Frontend calls Rust via `invoke()` in `src/lib/db-rust/`; no direct IndexedDB in new code
- **Tauri commands**: All in `src-tauri/src/commands/`
- **Rust error handling**: `thiserror` for domain errors, `anyhow` for app errors
- **Backup**: r2d2 connection pool export + optional Google Drive sync via `tauri-plugin-oauth`
- **No `as any` or `@ts-ignore`** — strict typing required
- **No dual IndexedDB+Rust paths** for same data — migrate to Rust when adding features

### 6.2 Frontend / SvelteKit 5

#### File Conventions
- `+page.svelte` = single file per route, but extract heavy logic to `*-state.svelte.ts` when >400 lines
- `$lib/` aliases to `src/lib/` (do not use relative `../../` for shared code)
- Route state lives in `src/routes/<route>/<route>-state.svelte.ts`
- Feature logic lives in `src/lib/features/<feature>/`

#### Data Loading
- Route pages use `<script lang="ts">` with explicit `$state` for reactive data
- No `+page.server.ts` or `+page.ts` load functions — all data fetched via `invoke()` inside `$effect` or `onMount`
- `onMount` runs once; `$effect` tracks dependencies for re-fetch

#### TypeScript
- Prefer `$derived` over manual recomputation
- Use `$derived.by(() => { ... })` for multi-step derivations
- No `any` types — use `unknown` + type guards
- Event handlers get `(e: Event)` not `(e: any)`

#### UI Components
- Scoped styles in `<style>` blocks (no global leakage)
- No CSS preprocessors — raw CSS + TailwindCSS 4 (v4, not v3 — `@import 'tailwindcss'`, no `@tailwind` directives)
- All shared UI in `src/lib/components/ui/`
- Components receive props via `interface Props` + `$props()` destructuring

#### State Pattern
- `*-state.svelte.ts` exports a class with `$state`/`$derived`/`$effect` fields
- Methods are arrow function properties or regular functions called with the instance
- Constructor runs `$effect` blocks for reactive side effects
- Usage in route: `let state = new XxxPageState();` then `state.method()` in template

#### Key Files
| File | Purpose |
|------|---------|
| `src/lib/types.ts` | All shared TS interfaces |
| `src/lib/db-rust/students.ts` | Student CRUD wrappers |
| `src/lib/db-rust/classes.ts` | Class CRUD wrappers |
| `src/lib/db-rust/attendance.ts` | Event CRUD wrappers |
| `src/lib/db-rust/settings.ts` | Settings + SF2 wrappers |
| `src/lib/stores/settings.svelte.ts` | Global settings singleton |
| `src/lib/features/settings/native.ts` | Native (Rust) API for settings |
| `src/lib/features/settings/sf2-workbook.ts` | SF2 workbook pure helper functions |
| `src/lib/student-analytics.ts` | Attendance analytics |
| `src/lib/components/ui/Dialog.svelte` | Reusable dialog |
| `src/lib/components/ui/Toast.svelte` | Toast notifications |

#### Route Key Files
| File | Purpose |
|------|---------|
| `src/routes/+layout.svelte` | Root layout (AppShell, favicon) |
| `src/routes/+layout.ts` | CSR-only (no SSR) |
| `src/routes/layout.css` | Imports `../app.css` |
| `src/routes/dashboard/` | Home/dashboard page |
| `src/routes/students/` | Student list + CRUD |
| `src/routes/attendance/` | Daily attendance |
| `src/routes/reports/` | Reports and analytics |
| `src/routes/settings/` | Settings + SF2 workbook |
| `src/routes/records/` | Audit records |

#### Pre-PR Checks (Frontend)
1. `bun run check` — type checks (0 errors)
2. `bun run lint` — lint clean
3. Verify `invoke()` error paths handled (try/catch, user-visible message)
4. Confirm no `as any` or `@ts-ignore` in diff
5. Check route files stay under 400 lines or delegate to `*-state.svelte.ts`

### 6.3 Backend / Rust

#### Tauri Commands
- All commands in `src-tauri/src/commands/`
- NEW commands → new `.rs` file in `commands/`
- Register in `commands/mod.rs` — add `pub mod XxxCommand;` and `handlers! { XxxCommand::execute }`
- Sync function + `Result<T, String>` return (do NOT use `#[tauri::command(async)]` — commands run on a threadpool, sync is fine)
- Dialog-based commands (open/save file pickers): use `std::sync::mpsc` to bridge Tauri's async dialog API to sync handlers
- `execute()` is the entry point — parse args, call service layer, map errors to strings

#### Module Organization
- `src-tauri/src/domain/` — models, schema, SQL migrations
- `src-tauri/src/infrastructure/database/` — connection pool, repos
- `src-tauri/src/commands/` — Tauri command handlers
- `src-tauri/src/services/` — external boundaries (currently not used; command files handle service logic directly)
- `src-tauri/src/utils/` — pure logic (no IO)
- `src-tauri/src/models/` — domain types (one type per file)
- `src-tauri/src/shared/` — cross-cutting helpers

#### Database / SQLite
- Always use `r2d2::Pool` for connections — never manual SQLite handles
- Repos in `infrastructure/database/repos/` — each repo is a struct with a `new(pool)` constructor
- Queries: use SQL files in `src-tauri/src/sf2/sql/` with `include_str!()` for SF2 module; for non-SF2 repos, inline SQL in repo methods is acceptable
- Schema: `DomainNameSchema` trait in `src-tauri/src/domain/` implementing `create_table()` + `drop_table()` + `migrate()`
- Migrations: idempotent ALTER TABLE IF NOT EXISTS in `migrate()` per schema

#### Testing
- Colocated `__tests__/` dirs under the module being tested
- Tests use `#[ctor]` for one-time setup (create temp DB, init pool)
- DB-dependent tests: create temp file DB via `Database`, test via repo, clean up after

#### Gotchas
- `rusqlite::Connection` is !Send — wrap in `Mutex` or use `r2d2` pool
- Tauri's `app_handle` is not available at startup for dialog commands — use `mpsc` channels
- Schema trait is NOT automatically called — `app_lib::run` must call `schema.create_table()` explicitly

### 6.4 SF2 Excel COM Automation Module

#### Architecture
- Location: `src-tauri/src/sf2/`
- COM automation via `interoptoke` crate — Windows-only
- Entry point: `sf2.rs` — re-exports, `get_sf2_root_path()`
- `sf2/mod.rs` — module declarations
- Workbook operations: `excel_com/workbook.rs` + `excel_com/com_session.rs` (extracted COM infra)
- Tests: colocated `__tests__/` per submodule

#### Key Files
| File | Purpose |
|------|---------|
| `excel_com/workbook.rs` | Workbook analysis, writing, batch operations |
| `excel_com/worksheet.rs` | Individual worksheet I/O |
| `excel_com/learners.rs` | Learner roster operations |
| `excel_com/calendar.rs` | Calendar/day mapping operations |
| `excel_com/com_session.rs` | COM object wrappers (ComObject, ComVariant, ExcelSession) |
| `excel_dialog.rs` | File dialog + template path logic |
| `roster/roster_parser.rs` | Parse SF2 roster exports |
| `roster/excel_service.rs` | Excel roster service |
| `roster/template_ops.rs` | Template operations |
| `roster/data_transfer.rs` | Data transfer operations |
| `roster/roster_sync.rs` | Roster sync logic |
| `backup/attendance_service.rs` | Attendance backup/restore |
| `sql/` | SQL query files (separated from Rust code) |

#### Test Conventions
- Each submodule has `__tests__/` with integration tests
- `sf2_integration_test.rs` in `src-tauri/src/sf2/__tests__/`
- Tests use `#[ctor]` for setup, run sequentially to avoid COM apartment conflicts
- COM tests need Windows + Excel installed — skipped on non-Windows CI

#### Anti-Patterns (SF2)
- No blocking main thread during COM calls — background thread via `run_excel_task`
- No inline SQL in `.rs` files — use `include_str!()` with `.sql` files
- No manual `unsafe` for COM — use `interoptoke` bindings
- No hardcoded Excel constants — define in `sf2_constants.rs`
- Tests MUST NOT have dangling COM objects — always call `release()` or drop in scope

---

## 7. ANTI-PATTERNS

- No direct `@tauri-apps/api` imports from route code — use feature native adapters (`src/lib/features/`)
- No blocking ops in async Tauri commands — `#[tauri::command]` handlers are sync, Tauri runs them on a threadpool
- No `as any` or `@ts-ignore` — strict typing required
- No dual IndexedDB+Rust paths for same data — migrate to Rust when adding features
- No manual SQLite connection handling — always use `r2d2::Pool`
- No inline SQL strings in `.rs` files — use `include_str!()` with dedicated SQL files
- No inline LLM/AI prompts in `.rs` files — use `include_str!()` with dedicated prompt files
- No Svelte 4 stores — Svelte 5 runes only (`$state`, `$derived`, `$effect`)
- No `helpers.rs` or `common.rs` dumping grounds — use `utils/` or `services/`

---

## 8. NOTES

- Tests are colocated: frontend `*.test.ts` next to source files, backend `__tests__/` under `src-tauri/src/sf2/`
- `src/app.css` is the main global stylesheet (design tokens, @font-face, utility classes, base resets)
- CI: `.github/workflows/release.yml` requires `TAURI_SIGNING_PRIVATE_KEY`
- The `state-context.ts` pattern provides parent→child state sharing in settings page
- All `*-state.svelte.ts` files follow: class with $state fields, $derived computed properties, arrow method properties for callbacks
- `bun run check` runs svelte-check, NOT tsc — it's the frontend type checker

---

## 9. KNOWN CONTRADICTIONS

The root `/AGENTS.md` previously said "Tauri commands: async if I/O." However, the actual pattern in the codebase uses sync functions + `std::sync::mpsc` for dialog-based commands. The `sync + mpsc` pattern is the correct description of the current code — commands are sync, Tauri runs them on a threadpool automatically.

---

## 10. PER-DIRECTORY FILES REMOVED IN CONSOLIDATION

This single file replaces the following per-directory AGENTS.md files (all deleted in the consolidation):
- `/AGENTS.md` (root — now a short cross-reference)
- `src/AGENTS.md`
- `src/routes/AGENTS.md`
- `src-tauri/AGENTS.md`
- `src-tauri/src/sf2/AGENTS.md`
