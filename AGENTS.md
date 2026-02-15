# EES-AMS - AttendEase Attendance Management System

## Development Commands

### Prerequisites
- **Bun** (package manager and runtime, recommended)
- **Rust 1.70+** (install from https://rustup.rs/)

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

# Frontend-only development (fast UI iteration)
bun run dev:frontend

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

# Run tests with output (from src-tauri directory)
cargo test -- --nocapture
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

### Backend Commands (from src-tauri directory)
```bash
# Check compilation
cargo check

# Run linter
cargo clippy -- -D warnings

# Format code
cargo fmt

# Run all tests
cargo test

# Run single test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run specific module tests
cargo test domain::entities::student

# Run importer tests
cargo test student_importer

# Build release version
cargo build --release

# Run development server
cargo run
```

### Frontend Commands (from src directory)
```bash
# Start development server
bun run dev

# TypeScript type checking
tsc --noEmit

# Run linter
bun run lint

# Fix lint issues
bun run lint:fix

# Format code
bun run format

# Check formatting
bun run format:check

# Build for production
bun run build
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

## TypeScript Frontend Guidelines

### Code Organization
- Components in `src/components/`
- Pages in `src/pages/`
- Utilities in `src/lib/`
- Types in `src/types/`
- Contexts in `src/contexts/`
- Hooks in `src/hooks/`

### Naming Conventions
- Components: `PascalCase`
- Functions, variables, files: `camelCase`
- Constants: `UPPER_SNAKE_CASE`
- Custom hooks: `use*` prefix

### TypeScript Standards
- Strict mode enabled
- No implicit `any` types
- Explicit return types for functions
- Use interfaces for object shapes

### React Patterns
- Functional components with hooks
- TypeScript props interfaces
- No class components
- Avoid `React.FC` (use explicit function signatures)

### State Management
- Local state with `useState`
- Global state through React Context
- Use `useMemo` for expensive computations
- Use `useCallback` for stable function references

### Tauri Integration
- Import from `@tauri-apps/api/core`
- Use `invoke` for backend calls
- Handle errors with try-catch
- Type-safe invocations using generics

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

## Lint-staged Configuration
```json
{
  "src/**/*.{ts,tsx,js,jsx}": ["eslint --fix", "prettier --write"],
  "src/**/*.{json,css,md}": ["prettier --write"],
  "*.{js,json,md}": ["prettier --write"]
}
```

## Testing
- Backend tests: `cargo test` from src-tauri directory
- Run single test: `cargo test test_name`
- Run with output: `cargo test -- --nocapture`
- Frontend tests: To be implemented with React Testing Library