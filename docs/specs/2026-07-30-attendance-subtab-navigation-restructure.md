# Restructure Overview & Attendance Logs as Subtab Navigation Under Attendance

**Date:** 2026-07-30  
**Status:** Draft  
**Author:** Buffy (AI Agent)

---

## 1. Motivation

Currently the sidebar contains six main navigation items: SF2 Reports, Attendance, Class List, **Attendance Logs**, **Overview**, and Configuration. The Overview page (`/overview`) and Attendance Logs page (`/records`) are conceptually subordinate to the Attendance flow — they display the same attendance data in different formats. Making them subtabs under the Attendance page reduces sidebar clutter and creates a clearer navigation hierarchy.

---

## 2. Route Changes

### 2.1 New Routes

| Old Path                | New Path                   | Page Title           |
| ----------------------- | -------------------------- | -------------------- |
| `/overview`             | `/attendance/overview`     | Daily Overview       |
| `/records`              | `/attendance/logs`         | Attendance Logs      |
| `/records/day-overview` | `/attendance/day-overview` | Daily Overview (day) |

### 2.2 Removed Routes

| Path                    | Action                                | Reason                                  |
| ----------------------- | ------------------------------------- | --------------------------------------- |
| `/overview`             | Redirect → `/attendance/overview`     | Route moved.                            |
| `/records`              | Redirect → `/attendance/logs`         | Route moved.                            |
| `/records/day-overview` | Redirect → `/attendance/day-overview` | Route moved.                            |
| `/dashboard`            | Delete entirely                       | Already dead (redirects to `/reports`). |

### 2.3 Root Route (`/`)

No change — continues to redirect to `/reports` as-is.

---

## 3. Sidebar Changes

### 3.1 Before

```
SF2 Reports      → /reports
Attendance       → /attendance
─────────────────
Class List       → /students
Attendance Logs  → /records     ← REMOVE
─────────────────
Overview         → /overview    ← REMOVE
Configuration    → /settings
```

### 3.2 After

```
SF2 Reports      → /reports
Attendance       → /attendance
─────────────────
Class List       → /students
─────────────────
Configuration    → /settings
```

**Order:** Reports → Attendance → divider → Class List → divider → Settings.

The `fullPreviewStore` hide-sidebar logic (for SF2 full-review mode) is unaffected.

---

## 4. Attendance Page Tab Bar

### 4.1 Design

A horizontal tab bar rendered at the top of every page under `/attendance/*`. It contains three tabs:

| Tab Label       | Target Route           |
| --------------- | ---------------------- |
| Attendance      | `/attendance`          |
| Daily Overview  | `/attendance/overview` |
| Attendance Logs | `/attendance/logs`     |

- **Active state:** The tab corresponding to the current URL path is visually highlighted (via `aria-current="page"` and CSS classes).
- **Default tab:** Visiting `/attendance` defaults to the "Attendance" tab.
- **Tab bar location:** Above the page content, below the page header area.
- **Responsive:** On narrow screens the tab bar should scroll horizontally if needed.

### 4.2 Component

Create a reusable tab-bar component at `src/lib/components/layout/AttendanceTabBar.svelte` (or co-located in `src/routes/attendance/`). It is rendered in the `+layout.svelte` file of the `src/routes/attendance/` route group (or directly in each sub-page if a layout file doesn't fit the existing structure).

---

## 5. Page Content per Route

### 5.1 `/attendance` — Attendance Grid (No Change)

The current attendance page (manual grid + card reader mode) stays exactly as-is. The only addition is the tab bar at the top.

The floating `AttendanceLog` (toast-like inline log within the attendance page) also stays unchanged — it is **not** the same as the full "Attendance Logs" page.

### 5.2 `/attendance/overview` — Daily Overview

This is the **existing content of `/overview`** moved to the new path:

- Today's attendance summary (stat cards: Students, Present, Absent, Rate)
- Today completion bar
- Absent students list
- Recent activity feed
- "Take Attendance" / "Start Live Session" CTA button
- Midnight auto-refresh
- Session-end banner notification

**Changes from the current `/overview/+page.svelte`:**

- Update all internal references to `/students`, `/attendance` to use `resolve()` with the correct paths.
- The `attendanceHref()` helper already points to `/attendance`, so no change needed for that.
- The "View all records" link in the recent activity panel: change from `/records` to `/attendance/logs`.
- Update breadcrumb / page header category to reflect it's under Attendance.

### 5.3 `/attendance/logs` — Attendance Logs

This is the **existing content of `/records`** moved to the new path:

- Filter controls (date range, class, student)
- Paginated attendance records table
- Export CSV functionality (via `exportCsvWithFolder`)
- Edit / audit / delete record actions
- Link to Daily Overview (now under the same tab bar)

**Changes from current `/records/+page.svelte`:**

- Update all internal references from `/records` to `/attendance/logs`.
- Update "Daily Overview" link from `/records/day-overview` to `/attendance/day-overview`.
- Update "SF2 Workbook" link — no change needed (`/reports`).
- Page title: "Attendance Logs".
- Breadcrumb category: "Attendance" or remove in favor of the tab bar.

### 5.4 `/attendance/day-overview` — Daily Day-Level Overview

This is the **existing content of `/records/day-overview`** moved to the new path:

- Date picker to select a day
- Roster, Present, Absent stat cards
- Per-student mark present / mark absent buttons
- Audit reason dialog for marking absent
- Links back to log view

**Changes from current `/records/day-overview/+page.svelte`:**

- Update "Back to logs" link from `/records` to `/attendance/logs`.
- Page description / category updated.

---

## 6. Redirect Handling

### 6.1 Old Route Redirects

Three old routes should redirect to their new locations with a brief toast notification:

| Old URL                 | New URL                    |
| ----------------------- | -------------------------- |
| `/overview`             | `/attendance/overview`     |
| `/records`              | `/attendance/logs`         |
| `/records/day-overview` | `/attendance/day-overview` |

**Implementation:** Keep old route files but replace their content with a redirect component that:

1. Calls `goto(resolve('/attendance/...'), { replaceState: true })`
2. Shows a toast: "This page has moved. You're being redirected…"
3. The toast auto-dismisses after 2 seconds.

**Alternative approach:** If SvelteKit route params allow, use a single catch-all pattern. However, the redirect + toast approach is cleaner for this limited set of routes.

### 6.2 `/dashboard` Route

Delete `src/routes/dashboard/+page.svelte` and its parent directory. The root page (`/`) already handles the redirect to `/reports`.

---

## 7. Cross-Reference Updates

All internal links referencing the old paths must be updated:

| File & Location                          | Old Reference                      | New Reference                         |
| ---------------------------------------- | ---------------------------------- | ------------------------------------- |
| `AppShell.svelte` navItems               | `/overview`                        | Remove entirely                       |
| `AppShell.svelte` navItems               | `/records`                         | Remove entirely                       |
| `overview/+page.svelte` L50              | `resolve('/overview')`             | `resolve('/attendance/overview')`     |
| `overview/+page.svelte` L357             | `resolve('/records')`              | `resolve('/attendance/logs')`         |
| `students/+page.svelte` L35              | `resolve('/records')`              | `resolve('/attendance/logs')`         |
| `students/student-list.svelte` L244      | `resolve(\`/records?...\`)`        | `resolve(\`/attendance/logs?...\`)`   |
| `records/+page.svelte` L201              | `resolve('/records/day-overview')` | `resolve('/attendance/day-overview')` |
| `records/day-overview/+page.svelte` L184 | `resolve('/records')`              | `resolve('/attendance/logs')`         |

---

## 8. File Structure Changes

### 8.1 New/Moved Files

```
src/routes/
├── attendance/
│   ├── +page.svelte                   (unchanged, but now has tab bar)
│   ├── +layout.svelte                 (NEW — wraps all /attendance/* pages with tab bar)
│   ├── overview/
│   │   └── +page.svelte               (MOVED from src/routes/overview/+page.svelte)
│   ├── logs/
│   │   ├── +page.svelte               (MOVED from src/routes/records/+page.svelte)
│   │   ├── records-filters.svelte      (MOVED from src/routes/records/)
│   │   ├── records-table.svelte        (MOVED from src/routes/records/)
│   │   ├── records-export-dialog.svelte (MOVED from src/routes/records/)
│   │   └── records-state.svelte.ts     (MOVED from src/routes/records/)
│   └── day-overview/
│       └── +page.svelte               (MOVED from src/routes/records/day-overview/+page.svelte)
```

### 8.2 Removed Directories

```
src/routes/dashboard/        ← Delete entirely
src/routes/overview/         ← Delete (content moved to /attendance/overview/)
```

### 8.3 Redirect-Only Files

```
src/routes/overview/+page.svelte        ← Replace with redirect to /attendance/overview
src/routes/records/+page.svelte         ← Replace with redirect to /attendance/logs
src/routes/records/day-overview/+page.svelte ← Replace with redirect to /attendance/day-overview
```

---

## 9. Implementation Steps

1. **Create `src/routes/attendance/+layout.svelte`** with the tab bar component that wraps all sub-pages.
2. **Create tab bar component** (`AttendanceTabBar`) with the three tabs and active-state logic.
3. **Move** `src/routes/overview/+page.svelte` → `src/routes/attendance/overview/+page.svelte` and update all internal links.
4. **Move** all files from `src/routes/records/` → `src/routes/attendance/logs/` and update internal links.
5. **Move** `src/routes/records/day-overview/+page.svelte` → `src/routes/attendance/day-overview/+page.svelte` and update internal links.
6. **Update** `AppShell.svelte` to remove `/overview` and `/records` from `navItems`.
7. **Replace** old route files (`/overview`, `/records`, `/records/day-overview`) with redirect components.
8. **Delete** `src/routes/dashboard/` entirely.
9. **Update all cross-references** listed in Section 7 across the codebase.
10. **Test** — navigate between all tabs, verify redirects work, verify no broken links.

---

## 10. Open Questions / Edge Cases

- **Tab bar visibility on `/attendance` card-reader mode:** The current attendance page in card-reader mode has a two-column layout. The tab bar should appear above this layout as well.
- **Import paths for moved `records-state.svelte.ts`:** The `records-state` module exports functions used by `records-export-dialog.svelte`. Both are being moved together, so relative imports should work fine with the new directory structure.
- **`attendance-log.svelte` vs `Attendance Logs` tab:** The attendance page already contains an inline `AttendanceLog` component (toast-like undo notifications). This is distinct from the full "Attendance Logs" page and should not be confused. The tab bar "Attendance Logs" tab links to the full-page log view.
