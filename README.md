# EES-AMS - Espiritu Elementary School Attendance Management System

[![EES Logo](https://via.placeholder.com/100x100/4A90E2/FFFFFF?text=EES)](https://www.espiritu-elementary.edu.ph)

[![Rust](https://img.shields.io/badge/rust-rust-orange?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Svelte](https://img.shields.io/badge/svelte-svelte-orange?style=flat-square&logo=svelte&logoColor=white)](https://svelte.dev)
[![Tauri](https://img.shields.io/badge/tauri-tauri-orange?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app)
[![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)

A cross-platform desktop application for student attendance management at Espiritu Elementary School with ID card reader support. Built with Tauri v2, SvelteKit 5, and Rust.

> **🏫 Elementary School Focused**: This system is specifically designed and optimized for elementary school environments (EES - Elementary Education System), with features tailored to the unique needs of primary education institutions.

## Features

- Cross-platform desktop app - Windows, macOS, Linux
- ID card reader support - Works with USB card readers as keyboard input
- Real-time attendance tracking - Instant data synchronization
- Offline-first design - No internet required
- SQLite database - Reliable local data storage
- Modern UI - Responsive SvelteKit 5 frontend
- Type-safe backend - Rust with comprehensive error handling

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Laptop (Tauri App)                       │
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │   Svelte UI  │───▶│  Tauri Cmds  │───▶│   SQLite DB  │ │
│  │  (Frontend)  │    │    (Rust)    │    │  (Persistent)│ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                                                   │
│    ┌────▼────┐                                             │
│    │  Card   │  Tap card → auto-types serial → lookup     │
│    │ Reader  │                                             │
│    └─────────┘                                             │
│                                                            │
│              USB/Bluetooth Card Reader                      │
└─────────────────────────────────────────────────────────────┘
```## Prerequisites

- **Node.js** 18+ (for frontend development)
- **Rust** 1.77+ (for backend development)
- **Bun** (recommended package manager)
- **USB card reader** (keyboard wedge mode, optional)

## Installation

### 1. Clone Repository

```bash
git clone https://github.com/your-username/espíritu-ams.git
cd espíritu-ams
```

### 2. Install Dependencies

```bash
# Frontend dependencies
bun install

# Rust dependencies
cd src-tauri && cargo fetch
```

### 3. Development Setup

```bash
# Start development server (both frontend and backend)
bun run tauri dev
```

## Usage

### Running the Application

#### Development Mode

```bash
bun run tauri dev
```

#### Production Build

```bash
bun run tauri build
```

The built executable will be in `src-tauri/target/release/`.

### Card Reader Setup

1. **Connect card reader** to laptop via USB
2. **Open Attendance page** in the app
3. **Tap a card** on the reader — serial auto-fills into the input
4. **System records** attendance automatically

### Card Workflow

1. **Register student cards** in the Students section (enter serial manually)
2. **Tap ID card** on the card reader during attendance
3. **System matches** serial to student and records attendance
4. **View reports** in real-time

## Project Structure

```
espíritu-ams/
├── src/                    # SvelteKit frontend
│   ├── lib/
│   │   ├── components/     # Reusable UI components
│   │   ├── entities/       # TypeScript types
│   │   └── db-rust.ts     # Tauri command wrappers
│   ├── routes/            # Page routes
│   └── app.css           # Global styles
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── domain/        # Business logic
│   │   ├── infrastructure/ # Database & hardware
│   │   └── commands.rs    # Tauri commands
│   └── Cargo.toml        # Rust dependencies
├── static/               # Static assets
└── docs/                # Documentation
```

## Development

### Code Quality Checks

```bash
# Frontend checks
bun run check && bun run lint && bun run typecheck

# Backend checks
cd src-tauri && cargo check && cargo clippy
```

### Code Formatting

```bash
# Frontend
bun run format

# Backend
cd src-tauri && cargo fmt
```

### Testing

```bash
# Frontend tests
bun test

# Backend tests
cd src-tauri && cargo test
```

## Security

- **Local only** - No network connectivity required
- **No authentication** - Designed for single-teacher use case

## Troubleshooting

### Card Reader Issues

**Problem**: Card reader not typing into input

1. Ensure reader is in keyboard wedge/HID mode
2. Check that the input field is focused
3. Try typing the serial manually to verify lookup works

## Performance

- **Connection pooling** - r2d2 for SQLite connections
- **Async runtime** - Tokio for concurrent operations
- **Zero-copy** - Minimal data cloning in Rust
- **Optimized builds** - LTO and aggressive optimizations

## Contributing

1. **Fork** the repository
2. **Create feature branch** (`git checkout -b feature/amazing-feature`)
3. **Commit changes** (`git commit -m 'feat: add amazing feature'`)
4. **Push to branch** (`git push origin feature/amazing-feature`)
5. **Open Pull Request**

### Commit Convention

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation updates
- `style:` - Code formatting
- `refactor:` - Code refactoring
- `test:` - Test additions/updates

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **Tauri** - Cross-platform app framework
- **SvelteKit** - Modern web framework
- **Rust** - Systems programming language
- **SQLite** - Reliable database engine

## Support

For support and questions:

- **Documentation**: Check the `/docs` folder
- **Issues**: Open an issue on GitHub
- **Discussions**: Use GitHub Discussions for questions

---

**EES-AMS** - Modern attendance management system designed exclusively for elementary schools (EES - Elementary Education System).
