# Present-by-Default Attendance Marking on the /attendance Page

**Date:** 2026-07-31
**Status:** Draft
**Author:** Buffy (AI Agent, via user interview)

---

## 1. Motivation

The SF2 workbook uses an *opt-out* attendance model: every cell is blank by default, which means **present**, and the teacher only clicks a cell to put an **X** on a student who is **absent**. The `/attendance` manual grid currently works the opposite way — it is *opt-in*:

- Students start as **Pending / IN** and must be clicked **one-by-one** to be recorded present, which is time-consuming for a whole class.
- Clicking an already-recorded box actually **deletes** the present record (making the student absent), but nothing in the UI says so.
- There is **no clear indicator** that an untouched ("not active") student is still considered **present by default** for SF2 purposes.

This spec makes the `/attendance` page mirror the SF2 mental model:

1. **"Present all"** records the entire class in a single action.
2. Clicking an individual box **toggles that student to absent** (visually highlighted, red + X).
3. A **legend + per-box labels** make it obvious that *Pending = Present by default*.

---

## 2. Scope

### 2.1 In scope — frontend only

| Area | Change |
|------|--------|
| `/attendance` manual grid — **Boxes** view | New three-state boxes (Pending / Present / Absent), legend, toggle behavior |
| `/attendance` manual grid — **List** view | Same states and labels |
| **Manual log dialog** (search-and-record picker) | Status labels reflect the new model |
| `attendance-page-state.svelte.ts` | Session absent tracking, updated present-all / clear-all / stats logic |
| `/attendance/day-overview` | Reframe "Absent / no present record" → "Not recorded" |
| Stat cards | Add **Absent** count; keep **Pending** |

### 2.2 Out of scope — no backend changes

The user's explicit final decision: **"no backend changes needed, it's already good, just UI frontend."**

- **No schema changes**, no new event types (`AttendanceType` stays `'in'`).
- **No Rust changes** — `attendance_marks.rs`, `logic.rs`, SF2 export all stay as-is.
- **SF2 export semantics unchanged** — on a closed day, a student with no present record gets an `X` (absent); a student with a record gets a blank (present). *This already produces correct SF2 output when the teacher uses the "Present all → click-to-absent" workflow described below.*
- **Card reader mode** flow and UI are **untouched**.
- **Late flagging** is **dropped from this flow** for now (see §5.6).

---

## 3. Decisions from the user interview

| # | Question | Decision |
|---|----------|----------|
| 1 | Resting state on page load | **Pending**, but with a clear legend: *Pending = Present by default* (not all boxes pre-marked green). |
| 2 | How absence is represented in data | **Delete the present record.** Absence = no present event for that day. No new event type. |
| 3 | When records are written | **Immediately per action** (current behavior — already good). |
| 4 | What "Present all" saves | **A present record for every student in the roster** → the day is closed; everyone counts as present in SF2 (blank cells). |
| 5 | Click on a Pending box | **Records present** (current behavior). |
| 6 | Click on a recorded (Present) box | **Removes the record → Absent** (the existing hidden toggle, now made visible). |
| 7 | Click on an Absent box | **Restores present** (re-records). Full cycle: Pending → click → Present → click → Absent → click → Present. |
| 8 | When does a day become "closed" | **"Present all" closes the day.** |
| 9 | Indicator placement | **Legend/caption row above the grid + a status label on every box.** |
| 10 | Absent state persistence | **Session-only highlight** (in-memory `Set`). After reload, absent students revert to *Pending* (no record exists). No data model change. |
| 11 | Visual states | **Green / Red / Neutral**: Present = green tint + check; Absent = red tint + X (mirrors SF2 grid); Pending = neutral with *"Pending · Present by default"* caption. |
| 12 | "Present all" re-records previously absent students | **Yes — records everyone** (resets the day to all-present). |
| 13 | "Clear all" button | **Keep it** (deletes all records → everyone back to Pending). |
| 14 | Boxes vs List view | **Update both views.** |
| 15 | Manual log dialog | **Update it too** — show the new status model. |
| 16 | Stat cards | **Add Absent + keep Pending** → Names / Present / Pending / Absent. |
| 17 | Day Overview page (`/attendance/day-overview`) | **Update it too** — align messaging with the attendance page. |
| 18 | "Present all" confirmation | **No confirmation** — toast + "Clear all" is enough. |
| 19 | Undo for marking absent | **No Undo needed** — clicking the box again re-records present (toggle cycle covers it). |
| 20 | Search filter + "Present all" | **Always record all** — "Present all" ignores the active search filter and records the whole class roster. |
| 21 | Day Overview wording for no-record students | **Reframe "Absent / no present record" → "Not recorded"** (neutral). |
| 22 | SF2 export for unrecorded students on a closed day | **Reconsidered** — final decision is **no backend changes**; SF2 keeps current behavior (no record on closed day = X). Correctness comes from the present-all workflow. |
| 23 | "Present all" button label | **Plain "Present all"** (no live count). |
| 24 | Pending per-box label wording | **"Pending · Present by default"** |
| 25 | Late flag | **Drop for now** — no late concept in this flow (logs/CSV still show existing data). |
| 26 | Card reader mode | **Untouched.** |

---

## 4. Data model — unchanged

- `AttendanceType = 'in'` (only present events are ever stored).
- **Absent = no 'in' record** for that student on that day.
- Marking a student absent = `deleteEvent(...)` (the same operation `logForStudent` already performs when `forcedType` is `null` and a last event exists).
- Marking present = `addEvent({ type: 'in', ... })` or `addEvents([...])` for bulk.
- SF2 export already derives marks from these records; no changes required.

---

## 5. UI model

### 5.1 Three box states

| State | Visual | Meaning | Data |
|-------|--------|---------|------|
| **Pending** | Neutral box (current `bg-background`), caption **"Pending · Present by default"** | Not recorded yet; will count as present if the day is recorded | No 'in' record; not in session-absent set |
| **Present** | **Green tint** + check icon, caption **"Present"** | Recorded present | Has an 'in' record for this session |
| **Absent** | **Red tint** + X icon (like the SF2 grid's `border-red-500/35 bg-red-50 text-red-700`), caption **"Absent"** | Explicitly marked absent this session | No 'in' record; **in the session-absent set** |

- Absent style reference (from `report-state.svelte.ts` `cellClass`): `border-red-500/35 bg-red-50 text-red-700` with an `<X>` icon.
- Present style: green equivalent — e.g. `border-green-500/35 bg-green-50 text-green-700` with a `<Check>`/`<CheckCircle2>` icon.
- Pending stays close to today's look but with the explicit caption.

### 5.2 Legend

A **legend row** above the grid (replacing/augmenting the current description line *"One click per learner. Boxes show whether attendance has been recorded for {date}."*). It should explain, in plain language:

> **Present by default.** Like SF2, every learner starts as present. Click **Present all** to record the class, then click individual boxes to mark those learners **absent**.

Plus a small three-item key (colored swatch + label):
- 🟢 Present
- 🔴 Absent
- ⚪ Pending · Present by default

### 5.3 Click behavior / toggle cycle

```
Pending ──click──▶ Present ──click──▶ Absent ──click──▶ Present
  │                   │                   │
  no record          has 'in' record     no record + session-absent flag
```

| Current box state | Click does | Data operation |
|-------------------|-----------|----------------|
| Pending | Records present | `addEvent` (creates 'in' record) |
| Present | Marks absent | `deleteEvent` (removes 'in' record) + adds student to session-absent set |
| Absent | Restores present | `addEvent` (re-creates 'in' record) + removes student from session-absent set |

This is the same data toggle the grid already performs (via `getNextAttendanceType` → `'in'` or `null`, and `logForStudent`'s `type === null && last` branch) — the change is mostly **making it explicit and visible** plus **tracking the absent set**.

### 5.4 Stat cards

Change from **Names / Recorded / Pending** to:

| Card | Value |
|------|-------|
| Names | `manualStudents.length` |
| Present | count of students with a recorded 'in' event |
| Pending | count of students with no record AND not in session-absent set |
| Absent | count of students in the session-absent set |

Formula: `manualStudents.length = Present + Pending + Absent`.

### 5.5 Buttons

- **"Present all"** — plain label, no count. Records **every** student in the class roster **regardless of the active search filter** (`rosterQuery`), creates an 'in' record for anyone who doesn't have one, **clears the session-absent set**, and shows the success toast. No confirmation dialog.
- **"Clear all"** — kept as-is. Deletes all present records for the day/session and clears the session-absent set; everyone returns to Pending.

### 5.6 Late flag — dropped (for now)

- Remove the LATE badge / `isLate` display from this flow's toasts, log pills, and grid captions.
- `checkLate` / `note: 'Late'` logic may stay in the state module but should not be surfaced in the `/attendance` page UI for new records (per user decision). Logs page and CSV export continue to show whatever exists in data.

### 5.7 Manual log dialog

- Update each row's status label to use the new states (Pending · Present by default / Present / Absent).
- Click behavior unchanged (clicking a row records present for that student; a recorded student shows as recorded).

### 5.8 Day Overview page (`/attendance/day-overview`)

- Reframe the no-record student label from **"Absent / no present record"** → **"Not recorded"** (neutral).
- The stat card label "Absent" and `absentCount` may stay (they describe the same set) but the row-level messaging should say "Not recorded" to align with the attendance page legend.
- Optionally, also update the "Absent today" wording on `/attendance/overview` for consistency (proposed, not confirmed — see §9).

---

## 6. Logic changes — `attendance-page-state.svelte.ts` (frontend only)

### 6.1 New state

```ts
absentStudentIds = $state<Set<string>>(new Set()); // session-only highlight
```

### 6.2 Derived counts

```ts
presentCount  = $derived(/* students with an 'in' record this session */); // existing recordedCount
absentCount   = $derived(this.manualStudents.filter(s => this.absentStudentIds.has(s.id)).length);
pendingCount  = $derived(this.manualStudents.length - this.presentCount - this.absentCount);
```

### 6.3 Status

```ts
getStudentStatus(student) {
  const last = this.lastEventByStudentForSession.get(student.id);
  if (last) return { label: 'Present', tone: 'present' as const };
  if (this.absentStudentIds.has(student.id)) return { label: 'Absent', tone: 'absent' as const };
  return { label: 'Pending · Present by default', tone: 'pending' as const };
}
```

### 6.4 Toggle cycle

`getNextAttendanceType` continues to return `'in'` (no record) or `null` (has record). `logForStudent`'s existing branch already handles both directions. On successful delete, add the student to `absentStudentIds`; on successful create, remove them from it.

### 6.5 Present all — updated

- Iterate the **entire roster** (`manualStudents` without the search filter applied, i.e. all students in the selected class).
- Build `CreateEventRequest`s for every student lacking an 'in' record.
- `await addEvents(...)`; prepend created events to `this.events`.
- **Clear `absentStudentIds`** (reset everyone to present).
- Toast: `"{N} students marked present"`.

### 6.6 Clear all — updated

- Delete all recorded events for the session (as today).
- **Clear `absentStudentIds`** too.

### 6.7 Date / class change

- Reset `absentStudentIds` when the selected date or selected class changes, since the session-absent highlight is per-session/per-day (matches existing `attendanceLog.resetState()` call in `selectAttendanceDate`).

---

## 7. Files to change

| File | Change |
|------|--------|
| `src/routes/attendance/attendance-grid.svelte` | Three-state boxes (Boxes + List), legend row, button label ("Present all", no count), green/red/neutral styles, per-box captions |
| `src/routes/attendance/attendance-page-state.svelte.ts` | `absentStudentIds` state, updated `getStudentStatus`, present-all/clear-all behavior, stat counts, reset on date/class change |
| `src/routes/attendance/attendance-manual-log-dialog.svelte` | Status labels per the new model |
| `src/routes/attendance/day-overview/+page.svelte` | "Absent / no present record" → "Not recorded" |
| `src/routes/attendance/+page.svelte` | Pass any new props/handlers (e.g. `absentCount`) if needed |
| *(optional)* `src/routes/attendance/overview/+page.svelte` | "Absent today" wording for consistency (unconfirmed — see §9) |

**No changes** to: `src/lib/types.ts`, `src/lib/db-rust/*`, `src-tauri/**`.

---

## 8. Edge cases & behavior notes

1. **Absent student after reload** — session-only highlight means a marked-absent student (record deleted) reverts to *Pending* after reload. The record stays deleted (still counts as absent in SF2). Acceptable per user decision; the caption "Pending · Present by default" is a session-UX framing.
2. **Pending student on a closed day** — with the recommended workflow ("Present all" → click-to-absent), everyone gets a record first, so no student is both "pending" and on a closed day. If a teacher records only a subset without "Present all", unrecorded students will get `X` in SF2 (existing backend behavior, explicitly accepted — no backend changes).
3. **Search filter + Present all** — Present all ignores the filter and records the whole roster; individual clicks remain filtered by search.
4. **Card reader mode** — untouched; the manual grid changes apply only in `manual` attendance mode.
5. **Late** — not surfaced in this flow for new records (user decision, "for now").
6. **Undo toast** — no undo for absent deletions; the toggle cycle (click again) restores present. Existing undo toast for present recordings stays.
7. **Duplicate records** — `addEvent`/`addEvents` already guards against duplicates ("already recorded"); Present all skips students who already have a record.

---

## 9. Open questions / proposed follow-ups

- **`/attendance/overview` wording** — should its "Absent today" panel (students with no present record) also be reframed to "Not recorded"? (Proposed for consistency; user confirmed only `/attendance/day-overview`.)
- **Legend copy** — exact wording/placement of the legend row (above stats vs. above search bar) to be finalized during implementation.
- **Late flag future** — "drop for now" implies a possible future reintroduction; no action now.
- **Persistent absent markers** — if the teacher later wants absent highlights to survive reloads (or wants SF2 "no record = present" semantics), that would require a backend change (new event type or absent flag table). Documented as a possible future iteration; explicitly out of scope today.

---

## 10. Implementation steps (suggested)

1. Add `absentStudentIds` session state + derived counts in `attendance-page-state.svelte.ts`.
2. Update `getStudentStatus` and toggle bookkeeping (`logForStudent` / `markStudent`).
3. Update `presentAllStudents` (record whole roster, ignore search filter, clear absent set) and `clearAllAttendance` (clear absent set).
4. Reset absent set on date/class change.
5. Restyle `attendance-grid.svelte`: three-state boxes in both Boxes and List views, green/red/neutral styles, per-box captions, legend row, "Present all" label.
6. Update `attendance-manual-log-dialog.svelte` status labels.
7. Update `/attendance/day-overview` messaging to "Not recorded".
8. Wire new props through `/attendance/+page.svelte`.
9. Validate: `npm run check` (or project typecheck), `npm test`, run the app in manual mode and verify the toggle cycle, Present all, Clear all, and stat counts.
