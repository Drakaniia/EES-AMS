# EES-AMS - Espiritu Elementary School Attendance Management System

[![EES Logo](https://via.placeholder.com/100x100/4A90E2/FFFFFF?text=EES)](https://www.espiritu-elementary.edu.ph)

[![Rust](https://img.shields.io/badge/rust-rust-orange?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Svelte](https://img.shields.io/badge/svelte-svelte-orange?style=flat-square&logo=svelte&logoColor=white)](https://svelte.dev)
[![Tauri](https://img.shields.io/badge/tauri-tauri-orange?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app)
[![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)

A cross-platform desktop application for student attendance management at Espiritu Elementary School with NFC/USB card reader support. Built with Tauri v2, SvelteKit 5, and Rust.

> **🏫 Elementary School Focused**: This system is specifically designed and optimized for elementary school environments (EES - Elementary Education System), with features tailored to the unique needs of primary education institutions.

## Features

- Cross-platform desktop app - Windows, macOS, Linux
- NFC card support - USB and mobile NFC readers
- Real-time attendance tracking - Instant data synchronization
- Local network architecture - Phone connects via WiFi/hotspot
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
│  │   Svelte UI  │───▶│  HTTP Server │───▶│   SQLite DB  │ │
│  │  (Frontend)  │    │  (Axum/Rust) │    │  (Persistent)│ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                             │
│         │              Bound to 0.0.0.0:3030               │
│         │              (accessible on LAN)                 │
└─────────┼────────────────────┼─────────────────────────────┘
          │                    │
          │                    │ HTTP API
          │                    │
┌─────────┼────────────────────┼─────────────────────────────┐
│         │                    │                             │
│  ┌──────▼──────┐    ┌────────▼────────┐                   │
│  │   Svelte UI │───▶│   HTTP Requests │                   │
│  │  (Browser)  │    │  to Laptop API  │                   │
│  └─────────────┘    └─────────────────┘                   │
│         │                                                  │
│    ┌────▼────┐                                             │
│    │ Web NFC │  Tap card → POST /api/events               │
│    └─────────┘                                             │
│                                                            │
│              Phone (Android Chrome)                        │
└────────────────────────────────────────────────────────────┘
```

## Prerequisites

- **Node.js** 18+ (for frontend development)
- **Rust** 1.77+ (for backend development)
- **Bun** (recommended package manager)
- **Android device** with Chrome for NFC scanning (optional)

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

### Mobile Device Setup

1. **Connect to same network** as the laptop (WiFi or hotspot)
2. **Open Chrome** on Android device
3. **Navigate to URL** shown in laptop app (e.g., `http://192.168.1.100:3030`)
4. **Start NFC scanning** on the Attendance page

### NFC Card Workflow

1. **Register student cards** in the Students section
2. **Tap NFC card** on phone device
3. **System records** attendance automatically
4. **Data syncs** instantly to laptop database
5. **View reports** in real-time on both devices

## Project Structure

```
espíritu-ams/
├── src/                    # SvelteKit frontend
│   ├── lib/
│   │   ├── components/     # Reusable UI components
│   │   ├── entities/       # TypeScript types
│   │   └── api.ts         # API client
│   ├── routes/            # Page routes
│   └── app.css           # Global styles
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── domain/        # Business logic
│   │   ├── infrastructure/ # Database & server
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

## API Endpoints

The HTTP server exposes the following REST API:

### Students

- `GET /api/students` - List all students
- `POST /api/students` - Create new student
- `GET /api/students/:id` - Get student by ID
- `PUT /api/students/:id` - Update student
- `DELETE /api/students/:id` - Delete student
- `GET /api/students/card/:serial` - Find student by NFC card

### Attendance Events

- `GET /api/events` - List all events
- `POST /api/events` - Create attendance event
- `GET /api/events/student/:id/last` - Get last event for student

### Settings & Data

- `GET /api/settings` - Get application settings
- `PUT /api/settings` - Update settings
- `GET /api/export` - Export all data as JSON
- `POST /api/import` - Import data from JSON
- `POST /api/wipe` - Clear all data

## Security

- **Local network only** - No internet connectivity required
- **No authentication** - Designed for single-teacher use case
- **Data encryption** - NFC card data encrypted at rest
- **CORS enabled** - Cross-origin requests for mobile access

> **Note**: For production deployment, consider adding API key authentication or JWT tokens.

## Troubleshooting

### Server Issues

**Problem**: Server won't start on port 3030

```bash
# Check if port is in use
netstat -an | grep :3030

# Kill process using port (Windows)
taskkill /PID <PID> /F
```

**Problem**: Firewall blocking connections

- Allow incoming connections on port 3030
- Add exception for Tauri app in Windows Defender

### Mobile Connection Issues

**Problem**: Phone can't connect to laptop

1. Verify both devices on same network
2. Check laptop's IP address in app
3. Try accessing `http://<laptop-ip>:3030/api/health` from phone browser
4. Disable VPN on phone if enabled

### NFC Issues

**Problem**: NFC not working on phone

1. Use Android Chrome (not Firefox/Samsung Internet)
2. Enable NFC in phone settings
3. Ensure cards are NFC-compatible
4. Try different tapping positions

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
- **Axum** - Web framework for Rust

## Support

For support and questions:

- **Documentation**: Check the `/docs` folder
- **Issues**: Open an issue on GitHub
- **Discussions**: Use GitHub Discussions for questions

---

**EES-AMS** - Modern attendance management system designed exclusively for elementary schools (EES - Elementary Education System).
