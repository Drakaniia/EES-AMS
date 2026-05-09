## Goal

A single-teacher app that records **student** attendance by tapping NFC ID cards. Teacher is the sole user/admin. Data stored in SQLite on the laptop. Works offline via local network (hotspot or LAN).

## Architecture

### Desktop App (Tauri + Rust)

- Native `.exe` (Windows), `.dmg` (Mac), or `.apk` (Android)
- Runs a local HTTP API server on port 3030 bound to `0.0.0.0`
- SQLite database for persistent storage
- Displays local IP address for phone connection

### Phone Access (Android Chrome)

- No app install needed — open `http://192.168.x.x:3030` in Chrome
- Full NFC scanning capability via Web NFC API
- All data syncs instantly to laptop's SQLite database
- Works over hotspot or local network

### Data Flow

```
Phone (Android Chrome)
  → NFC tap reads card serial
  → HTTP POST to laptop's API server
  → Rust backend writes to SQLite
  → Laptop UI reads from same SQLite
  → Data appears instantly on both devices
```

## User flow

1. **Students** page — teacher adds/edits students and registers each student's NFC card (tap once to capture the card's serial number).
2. **Attendance (Tap)** page — large kiosk-style screen. Teacher hits "Start scanning"; each tap toggles check-in / check-out for that student and shows a big confirmation (name, time, status). Manual fallback: pick student from a list.
3. **Records** page — filter by date range / student, see all check-in/out events, export CSV.
4. **Settings** — class name, work hours (for late flag), data export/import (JSON backup), wipe data.

## Setup Instructions

### Laptop Setup

1. Run the Tauri app (`.exe` on Windows)
2. The app will display the local server URL (e.g., `http://192.168.1.100:3030`)
3. Keep the app running while using the system

### Phone Setup (for NFC scanning)

1. Connect your Android phone to the same network as the laptop
   - Option A: Connect both to the same WiFi
   - Option B: Create a hotspot on the laptop, connect phone to it
2. Open Chrome on Android
3. Navigate to the URL shown in the laptop app
4. Start scanning NFC cards

### Manual Entry (Laptop Only)

- If you don't have an Android phone or NFC cards
- Use the "Manual Log" button to select students from a list
- Works on any device, any browser

## Pages (SvelteKit routes)

- `/` — Dashboard: today's count, currently checked-in list, quick link to Tap mode.
- `/students` — list, add, edit, delete, register card.
- `/attendance` — full-screen NFC tap mode.
- `/records` — table + filters + CSV export.
- `/settings` — preferences and backup.

## NFC integration (Web NFC)

- Uses `NDEFReader` (`navigator.nfc`) — Android Chrome only. On unsupported browsers (iOS, desktop), show a clear banner and fall back to manual student selection or typing a card serial.
- Card identity = `serialNumber` from the NFC reading event. Stored against each student.
- Registration: "Tap card to register" → wait for one read → save serial.
- Tap mode: continuous scan; on read, look up student by serial, log event, show toast + audible beep.

## Data model (SQLite via Rust)

- `students` { id, name, studentNumber, cardSerial, createdAt }
- `events` { id, studentId, type: 'in' | 'out', timestamp, note }
- `settings` { className, dayStart, dayEnd, lateAfter }

Auto-derive next event type per student (last event was "in" → next is "out").

## CSV export

Columns: Date, Student Number, Name, Check-in, Check-out, Duration, Late. One row per student per day. Range selectable.

## Tech / structure

### Backend (Rust)

- Tauri v2 for native desktop app
- Axum for HTTP API server
- SQLite with r2d2 connection pooling
- Tokio async runtime
- `thiserror` for error handling

### Frontend (Svelte)

- SvelteKit with static adapter
- Svelte 5 runes (`$state`, `$derived`, `$effect`)
- Tailwind CSS for styling
- Fetch API for backend communication
- Web NFC API for card scanning

### Project Structure

- `src-tauri/` — Rust backend
  - `src/domain/` — models and error types
  - `src/infrastructure/` — database and HTTP server
  - `src/commands.rs` — Tauri commands
- `src/` — Svelte frontend
  - `src/lib/api.ts` — API client
  - `src/lib/types.ts` — TypeScript types
  - `src/routes/` — SvelteKit pages

## Out of scope (v1)

Schedules/shifts, leave requests, multi-teacher logins, cloud sync.

## Notes / caveats

- Web NFC only works on Android Chrome over HTTPS or localhost
- The laptop must keep the app running for the phone to connect
- All data lives in the laptop's SQLite database
- Offline-first: no internet required, just local network
- Regular JSON backups recommended via Settings page
