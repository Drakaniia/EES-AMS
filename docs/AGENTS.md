# AGENTS.md — Project Knowledge Base

**Generated:** 2026-07-25 · **Revised:** 2026-10-02
**Branch:** main

This is the single authoritative AGENTS.md for the EES-AMS project. It consolidates all knowledge previously scattered across 6 per-directory files.

**Rewrite note.** The backend is now entirely TypeScript. `src-tauri/src/` holds nothing but plugin registration; everything the Rust side used to own lives under `src/lib/`. The migration decisions that shaped it (D1–D18) are recorded in [`docs/ts-migration-spec.md`](ts-migration-spec.md) — read that for _why_, this file for _what is where_.

---

## 1. PROJECT OVERVIEW

EES-AMS: cross-platform Tauri v2 desktop app for elementary school attendance management with manual name-based attendance. SvelteKit 5 + TypeScript + TailwindCSS 4 frontend. The whole backend — SQLite, Excel workbooks, backups, SF2 business logic — is TypeScript running in the webview. Windows-primary.

### Key Facts

- No auth — single-teacher desktop use case
- Windows NSIS installer via Tauri bundler
- Tauri updater plugin for in-app update notifications
- The Rust surface is five official plugins (dialog, fs, log, opener, updater) and nothing else
- Storage: SQLite in OPFS, files under `Documents\EES-AMS\`

---

## 2. PROJECT STRUCTURE

```
ees_ams/
├── src/                  # SvelteKit 5 frontend AND the entire backend
│   ├── routes/           # File-based routing per feature
│   └── lib/
│       ├── api/          # UI-facing adapters (the only thing routes import)
│       ├── db/           # SQLite driver, worker, migrations, repos
│       ├── domain/       # Domain types + settings shapes
│       ├── features/     # Business logic: excel, sf2, backup, settings
│       ├── platform/     # FileSystem interface + Tauri implementation
│       ├── components/   # Shared UI (ui/, layout/, students/)
│       └── stores/       # Global reactive singletons
├── src-tauri/            # Plugin registration only (~25 lines of Rust)
│   ├── src/lib.rs        # The whole Rust surface
│   ├── src/main.rs       # The binary's entry point
│   └── resources/sf2/    # Bundled DepEd workbook template
├── docs/                 # This file, DESIGN.md, ts-migration-spec.md
├── static/               # Self-hosted fonts + robots.txt
├── build/                # Build artifacts (generated)
├── output/               # Exports (JSON, CSV, DB backups)
└── .agents/              # AI agent skills
```

---

## 3. WHERE TO LOOK

| Task                     | Location                           | Notes                                                       |
| ------------------------ | ---------------------------------- | ----------------------------------------------------------- |
| Frontend pages           | `src/routes/`                      | SvelteKit file-based routing per feature                    |
| Shared UI primitives     | `src/lib/components/ui/`           | Dialog, Toast, Pagination, DatePicker, etc.                 |
| Global reactive state    | `src/lib/stores/`                  | `*.svelte.ts` singletons (settings, command palette, …)     |
| Data layer (SQLite)      | `src/lib/db/`                      | Driver interface, Worker, migrations, repos                 |
| Repositories             | `src/lib/db/repos/`                | students, classes, events, audit, settings, transfer        |
| Migration SQL            | `src/lib/db/sql/`                  | v1→v25 chain plus every named query                         |
| UI-facing data adapters  | `src/lib/api/`                     | Thin wrappers the UI imports instead of `$lib/db/repos`     |
| Excel engine             | `src/lib/features/excel/`          | ExcelJS I/O, cell access, formulas, layout constants        |
| SF2 business logic       | `src/lib/features/sf2/`            | Naming, logic, validation, calendar, roster, template, …    |
| SF2 month workbooks      | `src/lib/features/sf2/month/`      | 12-month split, workbook build/merge, student mapping       |
| SF2 attendance           | `src/lib/features/sf2/attendance/` | Read/write/import marks and events                          |
| Backup + restore         | `src/lib/features/backup/`         | Zip snapshots, retention, scheduling, restore preview       |
| Settings workflows       | `src/lib/features/settings/`       | Global settings, CSV, SF2 month settings                    |
| File system port         | `src/lib/platform/fs.ts`           | `FileSystem` interface, `MemoryFileSystem`, `useFileSystem` |
| Shared TS types          | `src/lib/types.ts`                 | UI-facing interfaces                                        |
| Domain types             | `src/lib/domain/`                  | `models.ts`, `settings.ts` — what repos read and write      |
| Startup wiring           | `src/lib/bootstrap.ts`             | Binds the injection points, once                            |
| Rust plugin registration | `src-tauri/src/lib.rs`             | The entire Rust surface                                     |
| Bundled SF2 template     | `src-tauri/resources/sf2/`         | `TEMPLATE_AUTOMATED_SF2.xlsx`, imported via Vite `?url`     |

---

## 4. CODE MAP

| Symbol                                              | Type               | Location                                 | Role                                               |
| --------------------------------------------------- | ------------------ | ---------------------------------------- | -------------------------------------------------- |
| `bootstrapApp`                                      | fn                 | `src/lib/bootstrap.ts`                   | One-time wiring of FS, preview, backups, quit hook |
| `SqlDriver` / `SqlError`                            | iface / class      | `src/lib/db/driver.ts`                   | The whole data-access surface                      |
| `getDriver` / `useDriver`                           | fns                | `src/lib/db/index.ts`                    | The one accessor for the database; test seam       |
| `WorkerSqlDriver`                                   | class              | `src/lib/db/client.ts`                   | Main-thread RPC; exclusive gate for transactions   |
| `node:sqlite` worker body                           | module             | `src/lib/db/worker.ts`                   | The only place SQLite exists at runtime            |
| `NodeSqlDriver`                                     | class              | `src/lib/db/node-driver.ts`              | `node:sqlite`, tests only                          |
| `AppError` / `errorMessage`                         | type / fn          | `src/lib/db/error.ts`                    | Rust `Display` strings, byte-for-byte              |
| `migrate` / `CURRENT_SCHEMA_VERSION`                | fn / const         | `src/lib/db/migrations.ts`               | The v1→v25 chain, stamped in `PRAGMA user_version` |
| `FileSystem` / `MemoryFileSystem` / `useFileSystem` | iface / class / fn | `src/lib/platform/fs.ts`                 | Every file read and write above this line          |
| `TauriFileSystem`                                   | class              | `src/lib/platform/tauri-fs.ts`           | `tauri-plugin-fs` implementation                   |
| `openWorkbook` / `saveWorkbookAtomic`               | fns                | `src/lib/features/excel/workbook.ts`     | ExcelJS read/write; writes are temp+rename         |
| `writableDayColumns`                                | fn                 | `src/lib/features/excel/workbook.ts`     | Day columns safe to write (merged pairs)           |
| `Sf2Formula` + formula fns                          | type / fns         | `src/lib/features/excel/formulas.ts`     | Formula text **and** the value Excel caches        |
| `getSf2WorkbookDir`                                 | fn                 | `src/lib/features/sf2/workbook-files.ts` | `Documents\EES-AMS\workbooks`, one derivation      |
| `getEesAmsRootDir`                                  | fn                 | `src/lib/features/backup/paths.ts`       | The root every output path hangs off               |
| `ensureDailyBackup` / `onAppQuit`                   | fns                | `src/lib/features/backup/scheduling.ts`  | Hourly timer + backup on close                     |
| `$lib/api` barrel                                   | modules            | `src/lib/api/index.ts`                   | What route code imports for data                   |

---

## 5. COMMANDS

```bash
# Frontend
bun install                # Install JS deps
bun run dev                # Vite dev server
bun run check              # svelte-check + type checking
bun run lint               # Prettier --check + ESLint
bun run format             # Prettier write

# Tests — Vitest only
bun run test               # Vitest, single run (all tests)
bun run test:watch         # Vitest in watch mode
bunx vitest run <path>     # One file, one run
bun run test:perf          # The month-switch wall-clock budget, alone (see below)

# Tauri
bun run tauri:dev          # Full dev (frontend + window)
bun run tauri:build        # Production build (NSIS on Windows)
bun run tauri <args...>    # Raw Tauri CLI, loaded with .env
```

> **`bun test` is not the test runner here.** `bun test` invokes Bun's own runner, which cannot load `node:sqlite` — every database test needs it, so the whole DB suite dies with `error: No such built-in module: node:sqlite`. Use `bun run test` or `bunx vitest run`. `package.json`'s `test` script is `vitest run`.

> **`month-switch-perf.test.ts` is excluded from `bun run test`.** It asserts wall-clock timings (p50/p95/p99), and a wall-clock budget measured while 66 other files compete for the CPU is meaningless — p50 tracks machine load roughly 1:1. Run it alone with `bun run test:perf`. Its scaling assertion (that month switching is linear, not quadratic) is the part that actually guards the code.

CI: `.github/workflows/release.yml` has one job. It installs Bun, then the Rust toolchain — the toolchain is there only to compile the binary, not to lint or test it. There is no Rust lint or test job, and `cargo` is not something you need to reach for.

---

## 6. CONVENTIONS

### 6.1 Project-Wide

- **Svelte 5 Runes only**: `$state`, `$derived`, `$effect` — no Svelte 4 stores
- **The backend is TypeScript.** Business logic, SQL and file I/O live under `src/lib/`; `src-tauri/src/` holds plugin registration only. New backend work is a TS module, never a `#[tauri::command]`
- **Route code imports from `$lib/api`,** never from `$lib/db/repos` or `@tauri-apps/*`. `$lib/api/*` are thin adapters and exist so the UI's call sites did not change during the migration
- **No `as any` or `@ts-ignore`** — strict typing required
- **One store per data domain.** SQLite is the only store; there is no IndexedDB path to keep in sync
- **Error strings are a UI contract.** `errorMessage()` in `$lib/db/error` reproduces the old Rust `Display` text byte-for-byte because the frontend shows it to the teacher. Reword nothing

### 6.2 Frontend / SvelteKit 5

#### File Conventions

- `+page.svelte` = single file per route, but extract heavy logic to `*-state.svelte.ts` when >400 lines
- `$lib/` aliases to `src/lib/` (do not use relative `../../` for shared code)
- Route state lives in `src/routes/<route>/<route>-state.svelte.ts`
- Feature logic lives in `src/lib/features/<feature>/`

#### Data Loading

- No `+page.server.ts` or `+page.ts` load functions — data is fetched inside `$effect` or `onMount`
- `onMount` runs once; `$effect` tracks dependencies for re-fetch

#### TypeScript

- Prefer `$derived` over manual recomputation; use `$derived.by(() => { ... })` for multi-step derivations
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
- Usage in route: `let state = new XxxPageState();` then `state.method()` in template

#### Key Files

| File                                        | Purpose                            |
| ------------------------------------------- | ---------------------------------- |
| `src/lib/types.ts`                          | All shared TS interfaces           |
| `src/lib/domain/models.ts`                  | Repo-facing domain types           |
| `src/lib/api/index.ts`                      | Barrel every route imports from    |
| `src/lib/stores/settings.svelte.ts`         | Global settings singleton          |
| `src/lib/features/settings/sf2-workbook.ts` | SF2 workbook pure helper functions |
| `src/lib/student-analytics.ts`              | Attendance analytics               |
| `src/lib/components/ui/Dialog.svelte`       | Reusable dialog                    |
| `src/lib/components/ui/Toast.svelte`        | Toast notifications                |

#### Route Key Files

| File                              | Purpose                                      |
| --------------------------------- | -------------------------------------------- |
| `src/routes/+layout.svelte`       | Root layout (AppShell, favicon)              |
| `src/routes/+layout.ts`           | CSR-only (no SSR)                            |
| `src/routes/layout.css`           | Imports `../app.css`                         |
| `src/routes/+page.svelte`         | Dashboard (uses `dashboard-state.svelte.ts`) |
| `src/routes/attendance/overview/` | Daily attendance                             |
| `src/routes/students/`            | Student list + CRUD                          |
| `src/routes/reports/`             | Reports and analytics                        |
| `src/routes/settings/`            | Settings, SF2 workbook, backup               |
| `src/routes/records/`             | Audit records                                |

#### Pre-PR Checks (Frontend)

1. `bun run check` — type checks (0 errors). Note this runs svelte-check, not `tsc`
2. `bun run lint` — lint clean
3. `bun run test` — suite green
4. Confirm no `as any` or `@ts-ignore` in diff
5. Check route files stay under 400 lines or delegate to `*-state.svelte.ts`

### 6.3 Data Access (`src/lib/db/`)

#### The layers

- **`driver.ts`** declares `SqlDriver`: `query`, `queryOne`, `execute`, `script`, `transaction`, `close`. Every repo is written against this interface and nothing above it may touch SQLite
- **`worker.ts`** is the only place SQLite exists at runtime: WASM SQLite from `@sqlite.org/sqlite-wasm`, over an OPFS file (`ees-ams.sqlite3`). It is a dedicated Worker so a report or import never blocks the UI and so the OPFS sync access handle stays owned by one context
- **`client.ts`** is the main-thread half: a typed `postMessage` RPC. `transaction()` holds an exclusive gate so a stray query from another caller cannot land between `BEGIN` and `COMMIT`
- **`node-driver.ts`** implements the same interface over `node:sqlite` for tests only. It is never bundled into the app

#### Repo rules

- **Repos take no driver parameter.** They call `getDriver()` from `$lib/db`. A `driver` argument is a way to give a repo a second data source, and there is none
- **SQL uses bound `?` parameters only.** No string interpolation, ever — a learner name comes from a school, not from you
- **Row shapes are coerced in the repo.** `node:sqlite` returns native values (`bigint` for INTEGER, `Uint8Array` for BLOB); repos narrow those to the types in `$lib/types`
- **Transactions wrap multi-statement writes** via `getDriver().transaction(async () => { ... })`
- Named queries live as `.sql` files in `src/lib/db/sql/`, imported with Vite's `?raw` and executed through `script()`/`execute()`. The `*.sql?raw` module shape is declared in `src/lib/db/sql.d.ts`

#### Migrations

- `migrations.ts` owns the chain; `CURRENT_SCHEMA_VERSION` is **25**. It refuses to leave a database above that number
- One step per version, ascending, each stamped into `PRAGMA user_version` after it runs, so a crash mid-migration replays only the versions after the last stamp
- There is no `migrate_to_v23.sql` because v23 was code, not schema
- `sql/` is a compatibility contract: version numbers, order and SQL body are all load-bearing. A teacher's database is at _some_ version in the middle of this list

### 6.4 Excel & SF2 (`src/lib/features/excel/`, `src/lib/features/sf2/`)

#### No Excel is launched

Workbooks are written with **ExcelJS** and then handed to `@tauri-apps/plugin-opener` if the user wants to see one. Nothing automates Excel, nothing needs Excel installed, nothing holds a COM object.

- **Formula values are computed in TypeScript.** ExcelJS writes a formula but never evaluates it, and the app compares X counts programmatically without opening the file. So a cell that carries a formula must also carry the value Excel would have cached for it. Every function in `features/excel/formulas.ts` returns that pair — `{ formula, value }` — and a new formula shape in the template means a new function there plus a test
- **Workbook writes are atomic.** `saveWorkbookAtomic()` writes to a sibling `.tmp` and renames over the target, so a failure part-way through leaves the previous good workbook intact. Use it; a direct `writeFile` on a workbook is a bug
- **Use `writableDayColumns(sheet)`.** The SF2 form merges consecutive day columns into pairs (`F8:G8`, `R29:S29`, …) so a weekend or non-school day occupies as much width as a school one. Only the left column of each pair holds the day number, and writing the right one lands on its master and silently overwrites it. `writableDayColumns` is the set that can actually be addressed; it cost someone a day during the migration
- **Merged cells need `writableCell(cell)`.** Every SF2 field and total sits in a merge and only the top-left cell holds a value. Writing anywhere else is dropped by Excel without an error
- The bundled template's sheet names, merged ranges and cell addresses are ground truth — they come from Excel's own conversion of the original workbook, verified in `features/excel/constants.ts`

#### Where SF2 logic lives

- Pure logic in `features/sf2/*.ts`: `naming`, `logic`, `validation` + `validation-service`, `metadata`, `calendar`, `first-school-day`, `workbook-files`, `repository`
- Attendance read/write/import in `features/sf2/attendance/`
- The 12-month workbook split in `features/sf2/month/` (`workbook-builder`, `workbook-sheets`, `merge`, `templates`, `students`)
- `preview.ts` and `progress.ts` feed the UI; `preview` is registered at startup (see below)

### 6.5 Files & Backup (`src/lib/features/backup/`, `src/lib/platform/`)

#### File IO goes through the `FileSystem` interface

- Every read and write above `src/lib/platform/fs.ts` goes through `getFileSystem()`. Nothing reaches for `tauri-plugin-fs` directly — that is what makes the feature code testable against `MemoryFileSystem`
- `TauriFileSystem` is the runtime implementation; `bootstrapApp()` binds it with `useFileSystem(new TauriFileSystem())`. Until it runs, `getFileSystem()` **throws on purpose** — a missing binding should be a startup error, not a workbook write that silently goes nowhere
- `FileSystem.writeFileAtomic` is the only write path for a file a teacher would be upset to lose

#### Where files live

Everything the app writes is under `Documents\EES-AMS\`:

```
Documents\EES-AMS\
├── workbooks\   # SF2 workbooks (+ _legacy\ holding the pre-split original)
├── backups\     # zip snapshots
└── exports\     # CSV / JSON / workbook exports
```

This is a deliberate departure from the app data directory (spec D13): a teacher has to find their workbooks and their backups in Explorer without knowing an app id or where Windows hides per-user app data. `backup/paths.ts` derives the root from `getSf2WorkbookDir()` rather than recomputing it — one derivation of one directory, because the Rust version had two and they drifted.

#### Backups

- Backups are local **zips** built with `fflate`. Entries are sorted so identical inputs produce identical bytes
- Scheduled by an hourly `setInterval` while the app is open (`scheduleBackups`), with a daily guard; `bootstrapApp()` also binds `onAppQuit()` to Tauri's close-requested event so a backup runs as the window closes
- There is no cloud sync, no OAuth, no keyring, no background task-scheduler entry. The interval timer is the only producer
- An archive carries the **real database file image** (`db.sqlite`): the worker serialises the live database with one `sqlite3_serialize()` call (the `export` op), and a restore hands the bytes back through the `import` op, which writes them over the OPFS file and reopens it. The `.sql` export in Settings is the readable counterpart — `INSERT` per row, for reading and diffing, never for restoring
- `x-count.ts` counts the `X` marks in the workbooks so the restore preview can tell a teacher their archive pairs a database with its own future before they click restore

### 6.6 Testing

- **Vitest only.** Config in `vitest.config.ts`, jsdom environment, `src/test-setup.ts`. Tests are colocated as `*.test.ts` and in `__tests__/` dirs
- **A test binds its own world before it touches anything:**

```ts
import { NodeSqlDriver, useDriver } from '$lib/db';
import { MemoryFileSystem, useFileSystem } from '$lib/platform/fs';

useDriver(new NodeSqlDriver()); // real SQLite, in memory
useFileSystem(new MemoryFileSystem());
```

- **Real SQL, fake disk.** `NodeSqlDriver` runs the actual query text against in-memory SQLite, so a test failure means the SQL is wrong, not that the mock drifted
- **Real workbook bytes.** `loadTemplate()` from `$lib/features/excel/__tests__/template-fixture.ts` reads the actual bundled template off disk and mounts it on a `MemoryFileSystem`. Sheet names and merged ranges in a fixture would be invented; the real ones are ground truth
- **No test needs Excel or a real disk.** If a test reaches for either, the seam is missing
- One runnable check per non-trivial rule. Prefer a single test that fails loudly over a suite that covers branches nobody exercises

---

## 7. ANTI-PATTERNS

- No direct `@tauri-apps/*` imports from route code — go through `$lib/api`, `$lib/features/*`, or the injection seams in `$lib/platform`
- No `bun test` — Bun's runner cannot load `node:sqlite` and every DB test dies. Use `bun run test`
- No driver parameter on a repo — repos call `getDriver()`
- No string-interpolated SQL — bind `?` parameters
- No second data store: the database is OPFS SQLite, full stop
- No `writeFile` on a workbook — `saveWorkbookAtomic` / `writeFileAtomic`, temp + rename
- No addressable cell written without checking the merge — use `writableCell` and `writableDayColumns`
- No formula written without its computed value — `features/excel/formulas.ts` returns the pair
- No rewording of `errorMessage()` output — the strings are shown to the teacher and are a compatibility contract
- No `as any` or `@ts-ignore` — strict typing required
- No Svelte 4 stores — Svelte 5 runes only (`$state`, `$derived`, `$effect`)
- No `helpers.ts` or `common.rs`-style dumping grounds — name the module after what it is for

---

## 8. NOTES

- `src/app.css` is the main global stylesheet (design tokens, @font-face, utility classes, base resets)
- `bun run check` runs svelte-check, NOT `tsc` — it's the frontend type checker
- All `*-state.svelte.ts` files follow: class with `$state` fields, `$derived` computed properties, arrow method properties for callbacks
- The `state-context.ts` pattern in `src/routes/settings/` provides parent→child state sharing
- Prettier is configured with `useTabs: true` and covers `docs/`, so this file is format-checked by `bun run lint`
- `$lib/db` exposes `exportFile()` / `importFile()` on `SqlDriver`: the Worker driver answers them with `sqlite3_serialize()` plus `OpfsDb.importDb()` in `worker.ts`, the Node test driver with `VACUUM INTO` plus a reopen — one contract, so backup → wipe → restore is proven against real bytes without a browser

---

## 9. WORKBOOK FORMAT & UPGRADING

The app writes **`.xlsx`**. The pre-split per-class workbook is kept untouched at `Documents\EES-AMS\workbooks\_legacy\` — it stays the last authoritative copy of the original, marks and all, and a backup snapshot walks it along with everything else.

**A teacher coming from the old `.xls` build:**

- The old workbooks do not become `.xlsx` by being renamed. Open the original once in Excel, Save As `.xlsx`, and import from that. The bundled template is already Excel's own `.xls`→`.xlsx` conversion, so the layout matches
- The old app data directory's `.sqlite` file is **not** read. The database now lives in OPFS as `ees-ams.sqlite3`, and the migration chain expects it to have been created by a previous version of _this_ app. Treat the first launch of the new build as a fresh start, and keep the old backups as your archive of record
- Attendance history lives in two places after the upgrade: the database (new) and the workbooks (old). Workbooks remain readable in Excel, and marks entered from this build on are read from the database
- Nothing in the app launches Excel. Workbooks open in whatever the OS associates with `.xlsx` when you ask for it

---

## 10. HISTORICAL NOTE

Sections 6.3, 6.4 and 6.5 replace what were previously "Backend / Rust" and "SF2 Excel COM Automation". The backend was Rust with `rusqlite` + `r2d2`, driven through `src/lib/db-rust/`, and SF2 workbooks were written by automating the Excel desktop application over COM. Both are gone, along with Google Drive backup, OAuth token exchange and the keyring.

What survived the move is worth knowing: the v1→v25 migration chain, the `AppError` message strings, the SF2 formula text and the workbook layout. All four are compatibility contracts with a teacher's existing data, which is why they were ported rather than redesigned. `docs/ts-migration-spec.md` records the decisions (D1–D18) behind each of them.
