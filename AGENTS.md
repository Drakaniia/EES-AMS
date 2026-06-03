!always follow @DESIGN.md and /reference when generating UI
!we start by converting typescript (/reference) into svelte
!always run `bun run check && bun run lint && bun run typecheck` / `cargo check` and `clippy fmt --check` after implementation to ensure quality
read @rust-skills before implementing rust code
read @rust-skills before implementing rust code
read @rust-skills before implementing rust code
read @rust-skills before implementing rust code
always read @rust-skills when implimenting code
always use @tauri-v2
always use @svelte-best-practices
always use @svelte-code-writer

# EES-AMS - Student Attendance Management System

## Project Snapshot

Tauri v2 desktop application for student attendance management with ID card reader support. SvelteKit 5 frontend with Rust backend, SQLite database, and modular feature architecture.

## Root Setup Commands

```bash
# Install dependencies
bun install
cd src-tauri && cargo fetch

# Development (both frontend and backend)
bun run tauri dev

# Build for production
bun run tauri build

# Quality checks
bun run check && bun run lint && bun run typecheck
cd src-tauri && cargo check && cargo clippy
```

## Universal Conventions

- **Code Style**: Prettier for frontend, rustfmt for backend
- **Commit Format**: Conventional commits (feat:, fix:, docs:, etc.)
- **Branch Strategy**: main for production, develop for integration
- **PR Requirements**: All checks must pass, include tests for new features
- **File Naming**: kebab-case for files, PascalCase for components/types

## Security & Secrets

- Never commit API keys, tokens, or sensitive data
- Use .env files for environment variables (gitignored)
- Handle PII data carefully in SQLite database

## JIT Index

### Package Structure

- Frontend: `src/` -> [see src/AGENTS.md](src/AGENTS.md)
- Rust Backend: `src-tauri/` -> [see src-tauri/AGENTS.md](src-tauri/AGENTS.md)
- Feature Routes: `src/routes/` -> [see src/routes/AGENTS.md](src/routes/AGENTS.md)

### Quick Find Commands

- Search function: `rg -n "functionName" src/ src-tauri/`
- Find component: `rg -n "export.*ComponentName" src/lib/components`
- Find Tauri command: `rg -n "#\[tauri::command\]" src-tauri/`
- Find API route: `rg -n "export const (GET|POST)" src/routes`
- Find database schema: `rg -n "CREATE TABLE" src-tauri/src/domain/`

## Definition of Done

- All TypeScript checks pass (`bun run check`)
- All Rust checks pass (`cargo check && cargo clippy`)
- Code formatted (`bun run format` and `cargo fmt`)
- Tests pass for new functionality
- Documentation updated for new features
- Card reader input tested (if applicable)
