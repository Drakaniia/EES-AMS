# EES-AMS - AttendEase Attendance Management System

## Project Overview

EES-AMS (AttendEase) is a modern, cross-platform attendance management system built with Tauri (Rust backend) and React (TypeScript frontend). Designed for educational institutions, it provides features for class management, student management, attendance recording, data statistics, and Google Sheets synchronization.

### Core Tech Stack

**Frontend (src/):**
- React 18 + TypeScript
- Tailwind CSS 4.x (via Vite plugin)
- Vite 7.x as build tool
- Tauri API 2.x for backend communication

**Backend (src-tauri/):**
- Rust 1.70+
- Tauri 2.0 framework
- Clean Architecture layered design
- JSON file storage (no database server required)
- Tokio async runtime
- OAuth2 for Google authentication
- ts-rs for TypeScript type generation

### Architecture Design

The project uses Clean Architecture layered design:

```
src-tauri/src/
├── domain/              # Domain layer (business logic core)
│   ├── entities/        # Domain entities (Class, Student, Attendance, SyncStatus)
│   ├── services/        # Domain service interfaces and implementations
│   ├── repositories/    # Repository interfaces (abstract data access)
│   └── errors/          # Domain error types
├── infrastructure/      # Infrastructure layer (technical implementation)
│   ├── database/        # Data persistence (JSON files)
│   ├── external/        # External services (Google API)
│   └── config/          # Configuration management
└── application/         # Application layer (coordination layer)
    ├── commands/        # Tauri IPC commands (frontend entry points)
    └── handlers/        # Request handlers (coordinate domain services and infrastructure)
```

**Frontend page structure:**
```
src/src/
├── components/          # Reusable UI components (Sidebar)
├── pages/              # Page components
│   ├── Dashboard.tsx   # Dashboard (statistics)
│   ├── Attendance.tsx  # Attendance recording
│   ├── Classes.tsx     # Class management
│   ├── Students.tsx    # Student management
│   └── Settings.tsx    # Settings (Google sync configuration)
└── lib/                # Frontend utilities (Tauri bindings)
```

## Building and Running

### Prerequisites

- **Bun** (package manager and runtime, recommended)
- **Rust 1.70+** (install from https://rustup.rs/)
- **Firebase project** (for hybrid storage - see docs/FIREBASE_INTEGRATION.md)

### Install Dependencies

```bash
bun install
```

### Environment Configuration

Copy the example environment file and configure your settings:

```bash
cp .env.example .env
# Edit .env with your Firebase and Google Drive credentials
```

#### Required Environment Variables

```bash
# Firebase (required for hybrid storage)
FIREBASE_PROJECT_ID=your-firebase-project-id
FIREBASE_SERVICE_ACCOUNT_KEY_PATH=./firebase-service-account.json

# Google Drive (optional)
GOOGLE_DRIVE_CLIENT_ID=your-google-drive-client-id
GOOGLE_DRIVE_CLIENT_SECRET=your-google-drive-client-secret

# Application settings
DATABASE_PATH=./data
SYNC_INTERVAL_MINUTES=30
```

### Development Commands

**Full application development (recommended):**
```bash
bun run dev
```
This starts both the frontend development server and Tauri development environment.

**Frontend-only development (fast UI iteration):**
```bash
bun run dev:frontend
```
Only starts the Vite development server, suitable for frontend styling and interaction development.

**Backend-only development (requires cargo-watch):**
```bash
bun run dev:backend
```
Uses cargo-watch to monitor Rust code changes and automatically recompile.

### Build Production Version

```bash
bun run build
```
This builds the frontend and packages the Tauri application, generating platform-specific installers.

### Frontend-specific Commands

In the `src/` directory:
```bash
bun run dev          # Start Vite development server
bun run build        # TypeScript type check + Vite build
bun run lint         # ESLint code check
bun run preview      # Preview production build
```

## Development Conventions

### Code Style

**Rust:**
- Follow Rust official code style (rustfmt)
- Use `cargo clippy` for additional checks
- All domain entities implement `ts-rs::TS` trait to generate TypeScript types
- Use `async/await` for async operations
- Use `Arc` and `Mutex` for shared state

**TypeScript:**
- Enable strict mode (`strict: true`)
- Enable all check options (`noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`)
- Use TypeScript strict type checking
- React components use functional components and Hooks

### Data Persistence

- Use JSON file storage in app data directory
- Data file path: `{app_data_dir}/attendease/`
- Abstract data access through Repository Pattern

### Frontend-Backend Communication

- Communicate via Tauri IPC commands
- All commands defined in `src-tauri/src/application/commands.rs`
- Command naming follows `feature_action` format (e.g., `class_create`, `student_get_all`)
- Use TypeScript types to ensure type safety

### Google Sheets Integration

- OAuth2 authentication flow
- Credentials stored in settings (JSON format)
- Supports offline mode, sync happens when connection is restored
- Sync status tracking (SyncStatus)

### Update Mechanism

- Use Tauri plugin `tauri_plugin_updater`
- Automatic update checking
- Supports silent update downloads
- Update endpoint configured in `tauri.conf.json`

## Tauri Configuration

- Application identifier: `com.attendease.app`
- Window size: 1400x900 (minimum 1024x700)
- Development server: `http://localhost:5173`
- CSP policy: Restricted to same-origin and inline styles
- Build targets: MSI, NSIS, DEB, AppImage, DMG, APP

## Type System

**Backend entity types:**
- `Class`: Class (ID, name, section, school year)
- `Student`: Student (ID, name, class ID)
- `Attendance`: Attendance record (ID, student ID, date, status)
- `AttendanceStatus`: Attendance status (Present, Absent, Late, Excused)
- `SyncStatus`: Sync status (Synced, Pending, Failed)

All entity types are automatically generated as corresponding TypeScript types via `ts-rs`, ensuring frontend-backend type consistency.

## Development Workflow

1. Modify domain entities or business logic → Update `src-tauri/src/domain/`
2. Add new features → Define commands in `src-tauri/src/application/commands.rs`
3. Implement frontend UI → In `src/src/pages/` or `src/src/components/`
4. Call backend commands via Tauri API → `import { invoke } from '@tauri-apps/api/core'`

## Debugging

**Backend debugging:**
- View Tauri dev tools console
- Use `println!` or `eprintln!` for debug output

**Frontend debugging:**
- Browser developer tools (dev mode)
- React DevTools extension

## License

MIT License