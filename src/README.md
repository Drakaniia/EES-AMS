# AttendEase - Attendance Management System

A modern, cross-platform attendance management system built with Tauri (Rust backend) and React (TypeScript frontend).

## Features

- **Class Management**: Create and manage classes with sections and school years
- **Student Management**: Add students with unique IDs and assign them to classes
- **Attendance Recording**: Record daily attendance with multiple status options (present, absent, late, excused)
- **Dashboard Statistics**: View real-time attendance statistics and trends
- **Google Sheets Sync**: Automatic synchronization with Google Sheets for backup and reporting
- **Offline-First**: Works offline and syncs when connection is available
- **Cross-Platform**: Runs on Windows, macOS, and Linux

## Tech Stack

### Frontend
- React 18 with TypeScript
- Tailwind CSS for styling
- Vite for build tooling

### Backend
- Rust with Tauri framework
- JSON file storage for data persistence
- OAuth2 for Google authentication
- Async/await with Tokio runtime

## Getting Started

### Prerequisites

- Node.js 18+ and npm
- Rust 1.70+ (install from https://rustup.rs/)
- For development: Tauri CLI

### Installation

1. Clone the repository
2. Install dependencies:
   ```bash
   npm install
   ```

3. Run in development mode:
   ```bash
   npm run dev
   ```

4. Build for production:
   ```bash
   npm run build
   ```


## Migration from Electron

This project was migrated from Electron to Tauri for:
- **Better Performance**: Rust backend is faster and more memory-efficient
- **Smaller Bundle Size**: ~10MB vs ~150MB for Electron
- **Enhanced Security**: Tauri's security model is more restrictive by default
- **Native Feel**: Better integration with OS features

### Key Changes
- Replaced Node.js backend with Rust
- Replaced Electron IPC with Tauri commands
- Removed `electron/` directory and related files
- Updated build configuration to use Tauri CLI
- Created Tauri API bridge for backward compatibility

## Google Sheets Integration

To enable Google Sheets sync:

1. Create a Google Cloud Project
2. Enable Google Sheets API and Google Drive API
3. Create OAuth 2.0 credentials (Desktop app)
4. In the app Settings, enter your:
   - Client ID
   - Client Secret
   - Redirect URI (usually `http://localhost`)

## Development

### Running Tests
```bash
npm run lint
```

### Building for Specific Platforms
```bash
# Windows
npm run tauri build -- --target x86_64-pc-windows-msvc

# macOS
npm run tauri build -- --target x86_64-apple-darwin

# Linux
npm run tauri build -- --target x86_64-unknown-linux-gnu
```

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## Support

For issues and questions, please open a GitHub issue.
