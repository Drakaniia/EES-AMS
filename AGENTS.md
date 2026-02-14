# EES-AMS - AttendEase Attendance Management System

## Project Overview

EES-AMS (AttendEase) is a modern, cross-platform attendance management system built with Tauri 2.0 (Rust backend) and React 18 (TypeScript frontend). Designed for educational institutions, it provides comprehensive features for class management, student management, attendance recording, data statistics, real-time synchronization, and multi-platform data storage.

### Core Features

- **Class Management**: Create and manage classes with sections and school years
- **Student Management**: Add students with unique IDs, Excel import support, assign to classes
- **Attendance Recording**: Record daily attendance with multiple status options (present, absent, late, excused)
- **Dashboard Statistics**: Real-time attendance statistics and trend analysis
- **Hybrid Storage**: Local JSON + Google Drive + Firebase Firestore synchronization
- **User Authentication**: JWT-based authentication with OAuth2 (Google) support
- **Auto-Update**: Automatic application updates with Tauri Updater Plugin
- **Offline-First**: Works offline and syncs when connection is available
- **Cross-Platform**: Runs on Windows, macOS, and Linux

## Tech Stack

### Frontend (src/)
- **React 18** with TypeScript
- **Tailwind CSS 4.x** (using @tailwindcss/vite)
- **Vite 7.3.1** for build tooling
- **Firebase 12.9.0** for authentication and realtime data
- **ESLint 10.0.0** + **Prettier** for code quality

### Backend (src-tauri/)
- **Rust** with Tauri 2.0 framework
- **Clean Architecture** (Domain/Infrastructure/Application layers)
- **JSON file storage** for local data persistence
- **Firebase Firestore** for cloud synchronization
- **OAuth2** for Google authentication
- **JWT** (jsonwebtoken) for token-based authentication
- **Argon2** for secure password hashing
- **calamine** for Excel import functionality
- **ts-rs** for automatic TypeScript type generation
- **Async/await** with Tokio runtime

### Development Tools
- **Bun** (package manager and runtime)
- **Husky** for Git hooks
- **Commitlint** for conventional commits
- **Lint-staged** for pre-commit checks
- **Rust Clippy** for Rust linting

## Build, Lint, and Test Commands

### Prerequisites
- **Bun** (package manager and runtime, recommended)
- **Rust 1.70+** (install from https://rustup.rs/)
- **Node.js** (for Vite development)

### Installation
```bash
# Install all dependencies
bun install

# Install Rust dependencies (done automatically on first build)
cd src-tauri && cargo fetch
```

### Development Commands (from project root)
```bash
# Full development (frontend + backend)
bun run dev

# Frontend-only development (fast UI iteration)
bun run dev:frontend

# Backend-only development (requires cargo-watch)
bun run dev:backend

# Production build
bun run build

# Install dependencies
bun run install
```

### Backend Commands (from src-tauri directory)
```bash
# Check compilation without building
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

# Build release version (optimized)
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

# Preview production build
bun run preview
```

## Code Style Guidelines

### Rust Backend Guidelines

#### Imports and Dependencies
- Group imports in order: std → external crates → internal modules
- Use `crate::` prefix for internal modules
- Prefer explicit imports over glob imports
```rust
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use crate::domain::entities::Student;
```

#### Naming Conventions
- Functions, variables, and files: `snake_case`
- Types, structs, enums, traits: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Module names: `snake_case`
- Tauri commands: `feature_action` pattern
- Services: `FeatureService` or `FeatureServiceImpl`

#### Error Handling
- Use `Result<T, DomainError>` for repository returns
- Use `?` operator for error propagation
- Domain errors in `domain/errors/mod.rs`
- Never use `panic!` in production code
- Use `anyhow` for flexible error context when needed
```rust
pub async fn create_student(&self, input: CreateStudentInput) -> DomainResult<i64> {
    let student = Student::new(/*...*/)?;
    Ok(self.student_repo.create(student).await?)
}
```

#### Async and State Management
- Use `async/await` for all async operations
- Shared state via `Arc<Mutex<T>>`
- Drop MutexGuard before await points to avoid deadlock
```rust
{
    let data = self.db.get_data().lock().unwrap();
    // Modify data
}
// Now we can await safely
some_async_fn().await
```

#### Domain Entities
- All entities must implement `ts-rs::TS` trait for TypeScript type generation
- Use `#[derive(Debug, Clone, Serialize, Deserialize, TS)]`
- Include serde attributes for proper JSON handling
- Add `#[ts(export)]` for types that need to be exported to frontend

#### Repository Pattern
- Interfaces (traits) in `domain/repositories/`
- Implementations in `infrastructure/database/`
- All methods are async
- Return `DomainResult<T>` type
- Use Arc for shared ownership across services

#### Service Layer
- Business logic in `domain/services/`
- Services depend on repository traits (not implementations)
- Use dependency injection for better testability
```rust
pub struct StudentServiceImpl {
    student_repo: Arc<dyn StudentRepository>,
}
```

### TypeScript Frontend Guidelines

#### Code Organization
- Components in `src/components/`
- Pages in `src/pages/`
- Utilities in `src/lib/`
- Types in `src/types/`
- Contexts in `src/contexts/`
- Hooks in `src/hooks/`

#### Naming Conventions
- Components: `PascalCase`
- Functions, variables, files: `camelCase`
- Constants: `UPPER_SNAKE_CASE`
- Interfaces: `PascalCase` with `I` prefix optional
- Custom hooks: `use*` prefix

#### TypeScript Standards
- Strict mode enabled
- No implicit `any` types
- Explicit return types for functions
- Use interfaces for object shapes
- Use `type` for unions and aliases
```typescript
interface Student {
  id: number;
  name: string;
  classId: number;
}

const createStudent = async (input: CreateStudentInput): Promise<Student> => {
  // Implementation
};
```

#### React Patterns
- Functional components with hooks
- TypeScript props interfaces
- No class components
- Avoid `React.FC` (use explicit function signatures)
```typescript
interface StudentListProps {
  students: Student[];
  onStudentSelect: (id: number) => void;
}

const StudentList = ({ students, onStudentSelect }: StudentListProps) => {
  return (
    <div>
      {students.map(student => (
        <StudentCard key={student.id} student={student} onClick={onStudentSelect} />
      ))}
    </div>
  );
};
```

#### State Management
- Local state with `useState`
- Global state through React Context
- Async state with custom hooks (e.g., `useUpdateService`)
- Avoid prop drilling when appropriate
- Use `useMemo` for expensive computations
- Use `useCallback` for stable function references

#### Tauri Integration
- Import from `@tauri-apps/api/core`
- Use `invoke` for backend calls
- Handle errors with try-catch
- Type-safe invocations using generics
```typescript
import { invoke } from '@tauri-apps/api/core';

const loadStudents = async (): Promise<Student[]> => {
  try {
    const response = await invoke<Student[]>('student_get_all');
    return response;
  } catch (error) {
    console.error('Failed to load students:', error);
    return [];
  }
};
```

#### Firebase Integration
- Initialize Firebase in `src/lib/firebase.ts`
- Use Firebase Auth for authentication
- Use Firestore for real-time data sync
- Handle Firebase errors gracefully
```typescript
import { initializeApp } from 'firebase/app';
import { getAuth } from 'firebase/auth';
import { getFirestore } from 'firebase/firestore';

const app = initializeApp(firebaseConfig);
const auth = getAuth(app);
const db = getFirestore(app);
```

### Git Workflow

#### Commit Messages
- Follow conventional commits: `type(scope): description`
- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`
- Scopes: `student`, `class`, `attendance`, `auth`, `sync`, `firebase`, `update`
- Example: `feat(student): add Excel import functionality`
- Example: `fix(auth): resolve JWT token expiration issue`

#### Pre-commit Hooks
- Husky runs before commit
- Lint-staged checks only staged files
- ESLint and Prettier run automatically
- Commitlint validates commit message format

#### Branch Organization
- `main` - production-ready code
- `develop` - integration branch
- `feature/*` - feature branches
- `hotfix/*` - emergency fixes
- `release/*` - release preparation

## Architecture

### Clean Architecture Layers

```
src-tauri/src/
├── domain/              # Domain Layer (Business Logic)
│   ├── entities/        # Domain entities (Student, Class, Attendance, User)
│   ├── repositories/    # Repository interfaces (traits)
│   ├── services/        # Domain services (StudentService, ClassService, AuthService)
│   └── errors/          # Domain errors (DomainError, ValidationError)
├── infrastructure/      # Infrastructure Layer (External Concerns)
│   ├── database/        # Data persistence (JsonDatabase, Repository implementations)
│   ├── external/        # External services (Firebase, Google Sync, OAuth2)
│   ├── config/          # Configuration management
│   └── importer/        # Data import (Excel import)
└── application/         # Application Layer (Orchestration)
    ├── commands/        # Tauri IPC commands
    └── handlers/        # Request handlers (coordinate between layers)
```

### Data Flow

```
Frontend (React)
    ↓ invoke()
Tauri Commands
    ↓
Application Handlers
    ↓
Domain Services
    ↓
Repositories (Infrastructure)
    ↓
Database / External Services
```

### Hybrid Storage Architecture

```
┌─────────────────────────────────────────┐
│         Application Layer               │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│     Hybrid Sync Service                 │
│  (coordinates between storage layers)   │
└─────────────────────────────────────────┘
         ↓              ↓              ↓
┌──────────────┐ ┌──────────┐ ┌────────────┐
│ Local JSON   │ │Google    │ │Firebase    │
│ Storage      │ │Drive     │ │Firestore   │
│ (Offline)    │ │(Backup)  │ │(Real-time) │
└──────────────┘ └──────────┘ └────────────┘
```

## Database Patterns

### Local JSON Storage
- Schema definition in `infrastructure/database/schema.rs`
- Counter IDs for entity relationships
- Atomic operations using Mutex locks
- Backup before major changes
- Automatic backup on startup

### File Structure
```
attendease-data/
├── attendance-data.json    # Main database (classes, students, attendance)
├── users.json              # User authentication data
├── settings.json           # Application settings
└── backups/                # Automatic backups
    ├── attendance-data-YYYYMMDD-HHMMSS.json
    └── ...
```

### Firebase Firestore Structure
```
collections/
├── users/                  # User accounts
├── classes/                # Class information
├── students/               # Student records
├── attendance/             # Attendance records
└── sync_metadata/          # Sync conflict resolution
```

## Authentication & Security

### User Authentication
- JWT-based authentication using `jsonwebtoken`
- Password hashing with Argon2
- OAuth2 flow for Google authentication
- Token validation and refresh
- Session management

### Firebase Security Rules
- Defined in `docs/FIREBASE_INTEGRATION.md`
- User-scoped data access
- Authentication required for writes
- Public read for certain collections

### Security Guidelines
- Never commit secrets or API keys
- Use environment variables for configuration
- Validate all user inputs on both client and server
- Sanitize data before storage
- Use prepared statements for database operations (when applicable)
- Implement rate limiting for sensitive operations
- Enable HTTPS in production
- Regular security audits and dependency updates

## Features & Integrations

### Excel Import
- Uses `calamine` crate for Excel file parsing
- Supports `.xls` and `.xlsx` formats
- Validates student data before import
- Bulk import with progress tracking
- Error reporting for invalid records
- Tests in `infrastructure/importer/student_importer_tests.rs`

### Google Drive Integration
- OAuth2 authentication flow
- File upload and download
- Backup and restore functionality
- Conflict resolution
- Sync status tracking

### Firebase Integration
- Real-time data synchronization
- Firestore for cloud storage
- Firebase Auth for authentication
- Offline support with local cache
- Conflict resolution strategies
- See `docs/FIREBASE_INTEGRATION.md` for details

### Auto-Update System
- Tauri Updater Plugin integration
- Automatic update checking (every 4 hours)
- User notification for available updates
- Download progress tracking
- Signature verification (production)
- Update server configuration in `tauri.conf.json`
- See `docs/AUTO_UPDATE_SETUP.md` for details

## Performance Considerations

### Backend
- Batch database operations when possible
- Use pagination for large datasets
- Lazy load non-critical data
- Cache frequently accessed data
- Optimize SQL/JSON queries
- Use `Arc` for shared state to avoid cloning
- Release Mutex locks before async operations

### Frontend
- Optimize React render cycles with `useMemo`/`useCallback`
- Lazy load components with React.lazy()
- Code splitting with dynamic imports
- Virtualize long lists
- Debounce search inputs
- Use Web Workers for heavy computations

### Release Build Optimization
```toml
[profile.release]
codegen-units = 1      # Compile slower but better optimized
lto = true            # Link time optimization
strip = true          # Strip symbols from binary
panic = "abort"       # Abort on panic (reduces binary size)
```

## Testing Strategy

### Backend Tests
- Unit tests for business logic in `domain/`
- Integration tests for repository operations
- Mock external dependencies (Firebase, Google API)
- Test error paths and edge cases
- Excel import tests in `infrastructure/importer/`
- Run with: `cargo test`

### Frontend Tests
- Component tests with React Testing Library
- Integration tests for user flows
- Mock Tauri API calls in tests
- Test loading and error states
- (To be implemented with React Testing Library)

### Test Commands
```bash
# Run all Rust tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test student_importer

# Run tests for specific function
cargo test test_excel_import_valid_data
```

## Development Workflow

1. Create feature branch from `develop`
2. Implement backend (domain → repository → handler → command)
3. Add TypeScript type generation with `ts-rs` attributes
4. Implement frontend components and pages
5. Test integration between frontend and backend
6. Run formatters and linters
7. Update documentation if needed
8. Submit PR for code review

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

### Tauri Configuration
- App configuration in `src-tauri/tauri.conf.json`
- Window settings (size, minimum size, decorations)
- Security policies (CSP)
- Updater endpoints and public key
- Bundle targets (MSI, NSIS, DEB, AppImage, DMG, APP)

## Debugging

### Backend
- Use `println!` or `eprintln!` for debug output
- Tauri dev tools console
- Rust debugger with `cargo-debug` or VS Code
- Enable logging with `env_logger::init()`

### Frontend
- Browser developer tools
- React DevTools
- Tauri console in development mode
- Network tab for API calls
- Application tab for local storage

### Common Issues

#### Build Errors
- Check Rust version: `rustc --version`
- Update dependencies: `cargo update`
- Clean build: `cargo clean && cargo build`

#### Runtime Errors
- Check console for error messages
- Verify Firebase configuration
- Check file permissions
- Ensure all environment variables are set

#### Sync Issues
- Verify Firebase security rules
- Check network connectivity
- Review sync logs for conflicts
- Ensure authentication tokens are valid

## Documentation

- **FIREBASE_INTEGRATION.md**: Firebase setup and configuration guide
- **AUTO_UPDATE_SETUP.md**: Auto-update system configuration
- **README.md**: Project overview and getting started
- **AGENTS.md**: This file - development guidelines
- **GITHUB_CODE_REVIEW_WORKFLOW.md**: GitHub code review workflow and best practices

## Support and Maintenance

### Regular Maintenance Tasks
- Update dependencies monthly
- Review and address security vulnerabilities
- Test auto-update flow
- Verify Firebase sync functionality
- Check database integrity
- Update documentation

### Version Management
- Follow semantic versioning (MAJOR.MINOR.PATCH)
- Update version in `src-tauri/Cargo.toml`
- Update version in `tauri.conf.json`
- Update CHANGELOG.md for each release
- Tag releases in Git

### Backup and Recovery
- Automatic backups created on startup
- Manual export functionality
- Restore from Google Drive
- Restore from Firebase Firestore
- Import from Excel backup files