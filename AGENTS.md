# PROJECT KNOWLEDGE BASE

**Generated:** 2026-07-05
**Commit:** a373fed
**Branch:** main

## OVERVIEW

EES-AMS: cross-platform Tauri v2 desktop app for elementary school attendance management with ID card reader support. SvelteKit 5 + TypeScript + TailwindCSS 4 frontend, Rust with rusqlite + r2d2 backend, SQLite database. Offline-first, Windows-primary.

## STRUCTURE

```
ees_ams/
├── src/              # SvelteKit 5 frontend
├── src-tauri/        # Rust/Tauri v2 backend (crate)
├── docs/             # Documentation, DESIGN.md tokens
├── test/             # Vitest frontend tests
├── static/           # Static assets (images, icons)
├── build/            # Build artifacts (generated)
├── output/           # Exports (JSON, CSV, DB backups)
└── .agents/          # AI agent skills
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Frontend pages | `src/routes/` | SvelteKit file-based routing per feature |
| Shared UI primitives | `src/lib/components/ui/` | Dialog, Toast, Pagination, etc. |
| Tauri command wrappers | `src/lib/db-rust/` | Frontend→Rust invoke bridge |
| Feature business logic | `src/lib/features/` | Per-feature TS workflows |
| Types | `src/lib/types.ts` | Shared TS interfaces |
| Rust Tauri commands | `src-tauri/src/commands/` | `#[tauri::command]` handlers |
| Domain models + schema | `src-tauri/src/domain/` | Business logic, DB schema, migrations |
| Database repos | `src-tauri/src/infrastructure/database/` | Pool + repository pattern |
| SF2 Excel automation | `src-tauri/src/sf2/` | COM-based Excel workbook I/O |
| Backup system | `src-tauri/src/backup/` | Local + Google Drive sync |
| Shared Rust utilities | `src-tauri/src/shared/` | Cross-cutting helpers |
| Docs | `docs/` | DESIGN.md, AGENTS.md, README.md |

## CODE MAP

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

## CONVENTIONS (PROJECT-SPECIFIC)

- **Svelte 5 Runes only**: `$state`, `$derived`, `$effect` — no Svelte 4 stores
- **Dual data layer**: Frontend calls Rust via `invoke()` in `src/lib/db-rust/`; no direct IndexedDB in new code
- **Tauri commands**: All in `src-tauri/src/commands/`, async if I/O, return `Result<T, String>`
- **Rust error handling**: `thiserror` for domain errors, `anyhow` for app errors
- **SF2 Excel**: COM automation on Windows only; uses `interoptoke` + Excel object model
- **Backup**: r2d2 connection pool export + optional Google Drive sync via `tauri-plugin-oauth`

## ANTI-PATTERNS (THIS PROJECT)

- No direct `@tauri-apps/api` imports from route code — use feature native adapters
- No blocking ops in async Tauri commands — use `tokio::spawn` for background work
- No `as any` or `@ts-ignore` — strict typing required
- No dual IndexedDB+Rust paths for same data — migrate to Rust when adding features
- No manual SQLite connection handling — always use `r2d2::Pool`

## COMMANDS

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

## NOTES

- Card reader = USB HID keyboard wedge mode (emulated keystrokes)
- No auth — single-teacher desktop use case
- Windows NSIS installer via Tauri bundler
- Tauri updater plugin for in-app update notifications
- CI: `.github/workflows/release.yml` requires `TAURI_SIGNING_PRIVATE_KEY`
- Google Drive backup uses `tauri-plugin-oauth` for token exchange
