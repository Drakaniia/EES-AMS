# Rust Backend - Tauri v2

## Package Identity

Tauri v2 Rust backend providing SQLite database, card reader support, and desktop integration for attendance management system.

## Setup & Run

```bash
# Install dependencies
cargo fetch

# Development mode (from project root)
bun run tauri dev

# Build for production
bun run tauri build

# Rust-specific checks
cargo check
cargo clippy
cargo fmt --check
cargo test
```

## Patterns & Conventions

- **Domain-Driven Design**: `domain/` for business logic, `infrastructure/` for external concerns
- **Error Handling**: Use `thiserror` for custom errors, `anyhow` for application errors
- **Database**: SQLite with `rusqlite` and connection pooling via `r2d2`
- **Async**: Use `tokio` runtime, `async/await` for I/O operations
- **Commands**: All Tauri commands in `commands.rs` with `#[tauri::command]` macro

Examples with actual file paths:

- DO: Follow command pattern from `src-tauri/src/commands.rs`
- DO: Database models in `src-tauri/src/domain/models/`
- DO: HTTP routes in `src-tauri/src/infrastructure/http/`
- DON'T: Direct database access without connection pooling
- DON'T: Blocking operations in async functions

## Key Files

- **Main Entry**: `src-tauri/src/main.rs`
- **Tauri Commands**: `src-tauri/src/commands.rs`
- **Domain Logic**: `src-tauri/src/domain/`
- **Infrastructure**: `src-tauri/src/infrastructure/`
- **Database Schema**: `src-tauri/src/domain/schema.rs`

## JIT Index Hints

- Find Tauri command: `rg -n "#\[tauri::command\]" src-tauri/src`
- Find database model: `rg -n "struct.*\{" src-tauri/src/domain`
- Find HTTP route: `rg -n "router\." src-tauri/src/infrastructure`
- Find error types: `rg -n "enum.*Error" src-tauri/src`
- Find tests: `find src-tauri/ -name "_test_.rs" -o -name "tests/"

## Database Patterns

- **Connection Pooling**: Use `r2d2::Pool` for SQLite connections
- **Migrations**: Schema changes in `src-tauri/src/domain/migrations/`
- **Queries**: Implement in domain layer, return Result<T, DomainError>
- **Transactions**: Use connection pool transactions for data consistency

## Common Gotchas

- Always handle SQLite connection errors gracefully
- Use `tokio::spawn` for background tasks, don't block main thread
- Tauri commands must be `async` if they perform I/O
- HTTP server runs on separate port from Tauri's built-in server

## Pre-PR Checks

```bash
cargo check && cargo clippy && cargo fmt --check && cargo test
```
