## Goal

A single-teacher app that records **student** attendance by tapping NFC ID cards. Teacher is the sole user/admin. All data stored locally in the browser (IndexedDB). Works offline.

## User flow

1. **Students** page — teacher adds/edits students and registers each student's NFC card (tap once to capture the card's serial number).
2. **Attendance (Tap)** page — large kiosk-style screen. Teacher hits "Start scanning"; each tap toggles check-in / check-out for that student and shows a big confirmation (name, time, status). Manual fallback: pick student from a list.
3. **Records** page — filter by date range / student, see all check-in/out events, export CSV.
4. **Settings** — class name, work hours (for late flag), data export/import (JSON backup), wipe data.

## Pages (TanStack routes)

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

## Data model (IndexedDB via `idb`)

- `students` { id, name, studentNumber, cardSerial, createdAt }
- `events` { id, studentId, type: 'in' | 'out', timestamp, note }
- `settings` { className, dayStart, dayEnd, lateAfter }

Auto-derive next event type per student (last event was "in" → next is "out").

## CSV export

Columns: Date, Student Number, Name, Check-in, Check-out, Duration, Late. One row per student per day. Range selectable.

## Tech / structure

- TanStack Start route files under `src/routes/`.
- `src/lib/db.ts` — idb wrapper.
- `src/lib/nfc.ts` — Web NFC helpers + capability detection.
- `src/lib/csv.ts` — CSV builder.
- `src/components/` — StudentForm, StudentList, TapScreen, RecordsTable, NfcUnsupportedBanner.
- shadcn/ui for tables, dialogs, inputs, toasts (sonner).
- Design tokens in `src/styles.css` (clean light theme; large readable kiosk type on Tap screen).

## Out of scope (v1)

Schedules/shifts, leave requests, multi-teacher logins, cloud sync. Easy to add later by enabling Lovable Cloud.

## Notes / caveats

- Web NFC only works on Android Chrome over HTTPS — the published Lovable URL satisfies this; the in-editor preview may not. We'll show a help note.
- All data lives in this browser only. Encourage regular JSON backups via Settings.
