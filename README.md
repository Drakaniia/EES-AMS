# EES-AMS - AttendEase Attendance Management System

A modern, cross-platform attendance management system built with Tauri (Rust backend) and React (TypeScript frontend).

## Project Structure

```
EES-AMS/
├── src/              # Frontend React application
│   ├── components/   # Reusable UI components
│   ├── pages/        # Route pages
│   ├── services/     # Frontend API services
│   └── types/        # TypeScript types
├── src-tauri/        # Backend Rust/Tauri application
│   ├── src/
│   │   ├── domain/           # Domain layer (business logic)
│   │   │   ├── entities/     # Domain entities
│   │   │   ├── services/     # Domain services
│   │   │   ├── repositories/ # Repository traits
│   │   │   └── errors/       # Domain errors
│   │   ├── infrastructure/   # Infrastructure layer
│   │   │   ├── database/     # Data persistence
│   │   │   ├── external/     # External services (Google API)
│   │   │   └── config/       # Configuration
│   │   └── application/      # Application layer
│   │       ├── commands/     # Tauri IPC commands
│   │       └── handlers/     # Request handlers
│   ├── Cargo.toml
│   └── tauri.conf.json
└── package.json
```

## Features

- **Class Management**: Create and manage classes with sections and school years
- **Student Management**: Add students with unique IDs and assign them to classes
- **Attendance Recording**: Record daily attendance with multiple status options (present, absent, late, excused)
- **Dashboard Statistics**: View real-time attendance statistics and trends
- **Google Sheets Sync**: Automatic synchronization with Google Sheets for backup and reporting
- **Offline-First**: Works offline and syncs when connection is available
- **Cross-Platform**: Runs on Windows, macOS, and Linux

## Tech Stack

### Frontend (src/)
- React 18 with TypeScript
- Tailwind CSS for styling
- Vite for build tooling

### Backend (src-tauri/)
- Rust with Tauri 2.0 framework
- Clean Architecture (Domain/Infrastructure/Application layers)
- JSON file storage for data persistence
- OAuth2 for Google authentication
- Async/await with Tokio runtime
- ts-rs for TypeScript type generation

## Prerequisites

- **Bun** (package manager and runtime)
- **Rust 1.70+** (install from https://rustup.rs/)

## Installation

```bash
bun install
```

## Development

Run the full application (recommended):
```bash
bun run dev
```

Frontend-only development (faster UI iterations):
```bash
bun run dev:frontend
```

Backend-only development (requires `cargo-watch`):
```bash
bun run dev:backend
```

## Build

```bash
bun run build
```

## License

MIT License