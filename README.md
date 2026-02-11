# EES-AMS - AttendEase Attendance Management System

A modern, cross-platform attendance management system built with Tauri (Rust backend) and React (TypeScript frontend).

## Project Structure

```
EES-AMS/
├── client/          # Frontend React application
├── server/          # Backend Rust/Tauri application
└── package.json     # Workspace scripts
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

### Frontend (client/)
- React 18 with TypeScript
- Tailwind CSS v4 for styling
- Vite for build tooling
- Bun as package manager

### Backend (server/)
- Rust with Tauri 2.0 framework
- JSON file storage for data persistence
- OAuth2 for Google authentication
- Async/await with Tokio runtime

## Prerequisites

- **Bun** (for client package management)
- **Rust 1.70+** (install from https://rustup.rs/)
- **Node.js 18+** (optional, if using npm instead of Bun)

## Installation

1. Install workspace dependencies:
```bash
bun run install:all
```

## Development

Run the application in development mode:
```bash
bun run dev
```

## Build

Build for production:
```bash
bun run build
```

## Linting

Run ESLint on the client code:
```bash
bun run lint
```

## Cleaning

Remove build artifacts and dependencies:
```bash
bun run clean
```

## License

MIT License

## See Also

- [Client README](client/README.md) for detailed frontend information