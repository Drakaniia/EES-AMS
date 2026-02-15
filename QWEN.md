# EES-AMS - AttendEase Attendance Management System

## Project Overview

EES-AMS is a modern, cross-platform attendance management system with a dual frontend approach. The project consists of:
1. A Tauri/Rust backend providing the core API services
2. Two frontend implementations: a React frontend and a Flutter desktop frontend

The system provides features for class management, student management, attendance recording, Google Sheets synchronization, and offline-first capabilities. The Flutter frontend was recently added as a migration from the original React/Tauri implementation to leverage Flutter's performance advantages.

## Architecture

### Backend (Tauri/Rust)
Located in `src-tauri/`, the backend uses:
- **Rust** with **Tauri 2.0** framework
- **Clean Architecture** with Domain/Infrastructure/Application layers
- **JSON file storage** for data persistence
- **OAuth2** for Google authentication
- **Tokio** runtime for async operations
- **Axum** web framework for HTTP API
- **Firestore** integration for cloud storage

### Frontend Options

#### Option 1: React Frontend (Legacy)
Located in `src/`, built with:
- **React 18** with TypeScript
- **Tailwind CSS** for styling
- **Vite** for build tooling

#### Option 2: Flutter Desktop Frontend (Current)
Located in `flutter/desktop/`, built with:
- **Flutter** framework for cross-platform desktop applications
- **Dart** programming language
- **Riverpod** for state management
- **Go Router** for navigation
- **Dio** for HTTP client
- **Material Design** widgets

## Key Features

- **Class Management**: Create and manage classes with sections and school years
- **Student Management**: Add students with unique IDs and assign them to classes
- **Attendance Recording**: Record daily attendance with multiple status options (present, absent, late, excused)
- **Dashboard Statistics**: View real-time attendance statistics and trends
- **Google Sheets Sync**: Automatic synchronization with Google Sheets for backup and reporting
- **Offline-First**: Works offline and syncs when connection is available
- **Cross-Platform**: Runs on Windows, macOS, and Linux

## Project Structure

```
EES-AMS/
├── flutter/                    # Flutter frontend implementations
│   ├── desktop/               # Flutter desktop app (primary)
│   └── mobile/                # Flutter mobile app (future)
├── src-tauri/                 # Backend Rust/Tauri application
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
├── package.json               # Workspace configuration
└── README.md
```

## Prerequisites

- **Bun** (package manager and runtime)
- **Rust 1.70+** (install from https://rustup.rs/)
- **Flutter 3.5.0+** (for Flutter frontend)
- **Dart 3.5.0+** (included with Flutter)

## Development Setup

### Installation
```bash
bun install
```

### Running the Applications

#### Full Application (Recommended)
```bash
bun run dev
```

#### Flutter Desktop Development
```bash
# Navigate to Flutter directory
cd flutter/desktop

# Install dependencies
flutter pub get

# Generate code (JSON serialization, etc.)
dart run build_runner build --delete-conflicting-outputs

# Run in development mode
flutter run

# Build for release
flutter build windows    # For Windows
flutter build macos      # For macOS
flutter build linux      # For Linux
```

#### Backend-only Development
```bash
bun run dev:backend
```

### Building for Production
```bash
# Tauri build
bun run build

# Flutter build
bun run build:flutter

# Build both
bun run build:all
```

## Development Conventions

### Flutter Frontend
- Use Riverpod for state management
- Follow clean architecture principles with separation of concerns
- Use code generation for JSON serialization with build_runner
- Implement responsive UI with Flutter's layout widgets
- Follow Material Design guidelines

### Rust Backend
- Maintain clean architecture with Domain/Infrastructure/Application layers
- Use async/await patterns consistently
- Implement proper error handling with Result types
- Use Serde for JSON serialization
- Follow Rust idioms and best practices

## Testing

### Backend Tests
```bash
bun run test
```

### Flutter Tests
```bash
bun run test:flutter
```

### All Tests
```bash
bun run test:all
```

## Code Quality

### Linting and Formatting
```bash
# Rust linting
bun run lint

# Rust formatting
bun run format

# Check formatting
bun run format:check
```

## Key Technologies

### Backend
- **Rust**: Systems programming language for backend logic
- **Tauri**: Framework for building desktop apps with web technologies
- **Axum**: Web framework for Rust
- **Firestore**: Cloud database integration
- **OAuth2**: Authentication protocol for Google services

### Flutter Frontend
- **Flutter**: Cross-platform UI toolkit
- **Dart**: Programming language for Flutter
- **Riverpod**: State management solution
- **Go Router**: Navigation and routing
- **Dio**: HTTP client
- **Material Design**: UI component library

## Migration Status

The project is transitioning from a React/Tauri frontend to a Flutter desktop frontend. The Flutter implementation is now the primary interface while maintaining compatibility with the existing Tauri/Rust backend.

### Completed Features
- Authentication system (Login, Register, Google OAuth)
- Dashboard with statistics
- Attendance tracking functionality
- Class management
- Student management
- Excel import functionality
- Google Sheets integration
- Settings and preferences
- Profile management

### Ongoing Development
- Real-time sync
- Auto-update system
- Push notifications
- Advanced reporting

## Performance Benefits

Migration to Flutter provides:
- Reduced app size (~80MB to ~10MB)
- Lower memory usage (~200MB to ~50MB)
- Faster startup times
- Improved CPU efficiency
- Better native UI consistency