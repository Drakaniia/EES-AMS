# EES-AMS Project Context & Instructions

This project is the **Espiritu Elementary School Attendance Management System (EES-AMS)**, a cross-platform desktop application for student attendance tracking.

## 🚀 Project Overview

- **Core Purpose**: Manage student attendance using NFC/USB card readers in a local school environment.
- **Target Platform**: Desktop (Windows, macOS, Linux) with mobile connectivity for card scanning.
- **Key Technologies**:
  - **Frontend**: SvelteKit 5 (Runes), TypeScript, Tailwind CSS 4, Vite 8.
  - **Backend**: Rust (Tauri v2), SQLite (rusqlite + r2d2).
  - **Hardware**: USB NFC card readers (PCSC/USB).
- **Architecture**: Local-first with a laptop acting as the central hub. Mobile devices connect via the local network to a REST API (planned/partial) to scan cards.

## 🛠 Building and Running

### Prerequisites

- Node.js 18+ & Bun (Package Manager)
- Rust 1.77+
- Tauri dependencies (platform-specific)

### Commands

- **Development**: `bun run tauri dev` (Starts frontend and backend)
- **Production Build**: `bun run tauri build`
- **Frontend Checks**: `bun run check` / `bun run lint` / `bun run typecheck`
- **Backend Checks**: `cd src-tauri && cargo check && cargo clippy`
- **Database**: Initialized automatically in the app data directory (`attendance.db`).

## 📂 Project Structure

- `src/`: SvelteKit frontend.
  - `lib/db-rust.ts`: Tauri command wrappers (Primary frontend-to-backend interface).
  - `lib/api.ts`: REST API client for mobile/external connectivity (Port 3030).
  - `routes/`: Application pages (Dashboard, Students, Attendance, etc.).
- `src-tauri/`: Rust backend.
  - `src/commands.rs`: Implementation of all Tauri `#[tauri::command]` functions.
  - `src/domain/`: Business logic and data models.
  - `src/infrastructure/`: Database access and hardware integration.
- `static/`: Static assets (Logos, icons).

## 📝 Development Conventions

### Backend (Rust)

- **Domain-Driven Design**: Keep business logic in `domain/` and implementation details in `infrastructure/`.
- **Error Handling**: Use `AppError` (defined in `domain/error.rs`) with `thiserror`.
- **Type Safety**: Ensure Rust structs match TypeScript interfaces in `src/lib/types.ts`.
- **Concurrency**: Use `tokio` for async operations and `r2d2` for database connection pooling.

### Frontend (Svelte)

- **Svelte 5 Runes**: Strictly use `$state`, `$derived`, and `$effect` for reactivity.
- **Component Design**: Modular components in `src/lib/components/`.
- **API Communication**: Prefer `db-rust.ts` for desktop features. `api.ts` is intended for mobile/LAN access.

## ⚠️ Known Discrepancies & TODOs

- **HTTP Server**: Documentation (README.md, AGENTS.md) mentions an Axum-based HTTP server on port 3030 for mobile connectivity. However, the current implementation in `src-tauri` does not yet include the `axum` dependency or the server logic.
- **NFC Drivers**: The current NFC implementation in `commands.rs` is partially simulated; hardware integration using `rusb`/`pcsc` is in progress.
- **Authentication**: The system currently operates without authentication, designed for single-teacher local use.
