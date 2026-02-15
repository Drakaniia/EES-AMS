# EES-AMS - AttendEase Attendance Management System

## Development Commands

### Prerequisites
- **Bun** (package manager and runtime, recommended)
- **Rust 1.70+** (install from https://rustup.rs/)
- **Flutter** (install from https://flutter.dev/)

### Installation
```bash
# Install all dependencies
npm run install:deps

# Or install individually
bun install
cd src-tauri && cargo fetch
cd flutter/desktop && flutter pub get
```

### Development Commands
```bash
# Full development (Tauri frontend + backend)
bun run dev

# Flutter development
bun run dev:flutter

# Backend-only development (requires cargo-watch)
bun run dev:backend

# Production build (all platforms)
bun run build:all

# Production build (Tauri only)
bun run build

# Production build (Flutter only)
bun run build:flutter

# Web build
bun run build:web
```

### Testing Commands
```bash
# Run all tests (Rust + Flutter)
npm run test:all

# Run Rust tests only
npm run test

# Run Flutter tests only
npm run test:flutter

# Run specific test (from src-tauri directory)
cargo test test_name

# Run tests with output (from src-tauri directory)
cargo test -- --nocapture

# Run specific module tests (from src-tauri directory)
cargo test domain::entities::student

# Run importer tests (from src-tauri directory)
cargo test student_importer
```

### Code Quality Commands
```bash
# Run linter for all projects
npm run lint

# Fix lint issues
npm run lint:fix

# Format all code
npm run format

# Check formatting
npm run format:check

# Type checking (TypeScript only)
npm run typecheck

# Check Rust compilation
cd src-tauri && cargo check
```

### Cleanup Commands
```bash
# Clean all build artifacts
npm run clean

# Clean Flutter only
cd flutter/desktop && flutter clean

# Clean Rust only
cd src-tauri && cargo clean
```

## Rust Code Style Guidelines

### Imports and Dependencies
- Group imports in order: std → external crates → internal modules
- Use `crate::` prefix for internal modules
- Prefer explicit imports over glob imports
```rust
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use crate::domain::entities::Student;
```

### Naming Conventions
- Functions, variables, and files: `snake_case`
- Types, structs, enums, traits: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Module names: `snake_case`
- Tauri commands: `feature_action` pattern

### Error Handling
- Use `Result<T, DomainError>` for repository returns
- Use `?` operator for error propagation
- Domain errors in `domain/errors/mod.rs`
- Never use `panic!` in production code
- Use `anyhow` for flexible error context when needed

### Async and State Management
- Use `async/await` for all async operations
- Shared state via `Arc<Mutex<T>>`
- Drop MutexGuard before await points to avoid deadlock

### Domain Entities
- All entities must implement `ts-rs::TS` trait for TypeScript type generation
- Use `#[derive(Debug, Clone, Serialize, Deserialize, TS)]`
- Add `#[ts(export)]` for types that need to be exported to frontend

### Repository Pattern
- Interfaces (traits) in `domain/repositories/`
- Implementations in `infrastructure/database/`
- All methods are async
- Return `DomainResult<T>` type
- Use Arc for shared ownership across services

## Flutter/Dart Frontend Guidelines

### Code Organization
- Flutter code in `flutter/desktop/`
- Mobile code in `flutter/mobile/`
- Generated code managed by build_runner

### Naming Conventions
- Classes, typedefs, and type parameters: `PascalCase`
- Libraries, packages, directories, and source files: `snake_case`
- Other identifiers: `camelCase`
- Private identifiers: `_privateIdentifier`

### Dart Standards
- Use `dart format` for code formatting
- Follow Effective Dart guidelines
- Use meaningful variable names
- Prefer final for immutable objects

### State Management
- Use Provider or Riverpod for state management
- Follow BLoC pattern for complex business logic
- Use immutable data structures when possible

## Git Workflow

### Commit Messages
- Follow conventional commits: `type(scope): description`
- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`
- Scopes: `student`, `class`, `attendance`, `auth`, `sync`, `firebase`, `update`
- Example: `feat(student): add Excel import functionality`

### Pre-commit Hooks
- Husky runs before commit
- Lint-staged checks only staged files
- ESLint and Prettier run automatically
- Commitlint validates commit message format

### Branch Organization
- `main` - production-ready code
- `develop` - integration branch
- `feature/*` - feature branches
- `hotfix/*` - emergency fixes
- `release/*` - release preparation

## Configuration

### Environment Variables
Create `.env` file from `.env.example`:
```bash
# Firebase Configuration
FIREBASE_PROJECT_ID=your-firebase-project-id
FIREBASE_SERVICE_ACCOUNT_KEY_PATH=./firebase-service-account.json
FIREBASE_API_KEY=your-firebase-web-api-key
FIREBASE_AUTH_DOMAIN=your-project.firebaseapp.com
FIREBASE_DATABASE_URL=https://your-project.firebaseio.com
FIREBASE_STORAGE_BUCKET=your-project.appspot.com

# Google Drive Configuration
GOOGLE_DRIVE_CLIENT_ID=your-google-drive-client-id
GOOGLE_DRIVE_CLIENT_SECRET=your-google-drive-client-secret
GOOGLE_DRIVE_REDIRECT_URL=http://localhost:8080/callback

# JWT Configuration
JWT_SECRET=your-jwt-secret-key-change-in-production

# Database Configuration
DATABASE_PATH=./data
SYNC_INTERVAL_MINUTES=30

# Application Configuration
APP_ENV=development
LOG_LEVEL=debug
```

### Lint-staged Configuration
```json
{
  "src/**/*.{ts,tsx,js,jsx}": ["eslint --fix", "prettier --write"],
  "src/**/*.{json,css,md}": ["prettier --write"],
  "*.{js,json,md}": ["prettier --write"]
}
```

## Testing
- Backend tests: `cargo test` from src-tauri directory
- Run single test: `cargo test test_name` from src-tauri directory
- Run with output: `cargo test -- --nocapture` from src-tauri directory
- Frontend tests: `flutter test` from flutter/desktop directory
- All tests: `npm run test:all`

## Security Best Practices
- Never commit secrets or API keys
- Use environment variables for sensitive data
- Validate all user inputs
- Sanitize data before storing or transmitting
- Use HTTPS for all network communications
- Implement proper authentication and authorization