# Right-Click to Mark Absent — Attendance Page

**Date:** 2026-07-31
**Status:** Approved (design choices confirmed by user)
**Scope:** `src/routes/attendance/` frontend only — no Rust backend changes

## Summary

Add a right-click shortcut on student rows/cards in the Attendance page that marks the
student **absent** directly, as an alternative to the current click-to-toggle flow.
The native browser context menu is suppressed at the interaction sites.

## Background: current behavior

In the manual grid ("Student boxes") / list view, left-click cycles a student through:

- **Pending** (no `in` record, not absent-marked) → marks **present** (creates an `in` record)
- **Present** (has an `in` record) → marks **absent** (deletes the `in` record + adds session absent highlight)
- **Absent** (no record, absent-marked) → marks **present** again

Absence is represented by the *absence of an `in` record*. The `absentStudentIds`
`SvelteSet` on `AttendancePageState` is a **session-only UI highlight** (drives box
styling and the Absent stat). It is reset on date change, class change, "Clear all",
and previously on "Present all".

## Behavior spec (confirmed with user)

Right-click on a student (in Boxes view, List view, and the Manual log dialog):

1. **Present student** → delete their `in` record (same database effect as left-click on a
   present box) and add the absent highlight. Toast: `{name} marked absent`.
2. **Pending student** → add to `absentStudentIds` (session absent highlight). No database
   write — absence is the absence of a record. Toast: `{name} marked absent`.
3. **Already absent** → silent no-op (the box is already styled absent).

Left-click behavior is unchanged (it remains the "mark present / toggle" control).

### "Present all" interaction (confirmed with user)

`presentAllStudents()` **skips** students in `absentStudentIds` and **keeps** the absent
highlights after the bulk record. This supports the natural workflow of right-clicking
the missing kids first, then recording the rest of the class.

- New toast when absents are kept: `{X} marked present · {Y} kept absent`.
- Empty-result edge: when every student is recorded or absent-marked, toast
  `All students are already recorded or marked absent` (instead of the current
  misleading "All students are already recorded").

## Implementation

### 1. `src/routes/attendance/attendance-page-state.svelte.ts`

**New method `markAbsent(student: Student)`:**

```
1. Guard: if isProcessing || dateLoading → toast 'Please wait - processing previous request', return.
2. last = getLastEventForSession(student)
3. If last exists (present):
     isProcessing = true
     try: await logForStudent(student, null, { suppressLate: true, message: `${student.name} marked absent` })
     finally: isProcessing = false
4. Else (no record):
     If absentStudentIds.has(student.id) → return (already absent, no-op)
     absentStudentIds.add(student.id)
     toast `${student.name} marked absent`
```

**`LogOptions` change** (`attendance-state.svelte.ts`): add optional
`message?: string`. In `logForStudent`'s delete-record branch (type `null` + last exists),
use `options.message` for the toast when provided, else the current
`{student.name} - Attendance removed`. Keeps the right-click feedback consistent
("marked absent") while leaving the existing left-click message untouched.

**`presentAllStudents()` change:**

```
studentsToMark filter adds: && !this.absentStudentIds.has(student.id)
```

Remove the `this.absentStudentIds.clear()` after the batch write (highlights persist).
Toast becomes `${createdEvents.length} ${n===1?'student':'students'} marked present`
plus ` · ${absentStudentIds.size} kept absent` when the set is non-empty.
Empty-result toast distinguishes the absent-marked case as above.

### 2. `src/routes/attendance/attendance-grid.svelte`

- Add prop `onMarkAbsent: (student: Student) => void`.
- **Boxes view:** on each box `<button>` add
  `oncontextmenu={(e) => { e.preventDefault(); onMarkAbsent(student); }}`.
  Extend `title` to `{student.name} - {status.label} · Right-click to mark absent`.
- **List view:** on each row `<li>` add the same `oncontextmenu` handler (whole row
  is the shortcut; right-clicking the Record button also marks absent — consistent).
- Existing `disabled={isProcessing || dateLoading}` stays; contextmenu on a disabled
  control is inert (native menu may appear during processing — harmless).

### 3. `src/routes/attendance/attendance-manual-log-dialog.svelte`

- Add prop `onMarkAbsent: (student: Student) => void`.
- On each picker row `<li>` add the same `oncontextmenu` handler.
- Dialog **stays open** after right-click (unlike left-click which closes via
  `closePicker`) so several students can be marked absent in a row.

### 4. `src/routes/attendance/+page.svelte`

- Pass `onMarkAbsent={(student) => void attendanceState.markAbsent(student)}` to
  `AttendanceGrid` and `AttendanceManualLogDialog`.

## Out of scope (unchanged)

- Left-click toggle behavior in all three views.
- Card-reader mode (no student boxes rendered there; the Manual log dialog right-click
  still works since it shares the roster).
- "Clear all" (still resets absent highlights), date/class-change highlight reset,
  midnight refresh.
- No new unit tests for `AttendancePageState` (coupled to Tauri commands; no existing
  test harness — the project tests only pure helpers, e.g. `report-state.test.ts`).
  Covered by `bun run check` / typecheck and manual QA.

## Verification

1. `bun run check` passes (Svelte check + types).
2. Manual QA:
   - Right-click a pending box → turns absent (red), Absent stat +1, toast shown.
   - Right-click a present box → `in` record deleted, turns absent.
   - Right-click an absent box → nothing changes, no toast.
   - Right-click a pending **list row** → same as boxes.
   - Open Manual log dialog → right-click a pending student → marked absent, dialog stays open.
   - Right-click 3 students absent, then "Present all" → 3 stay absent, rest recorded,
     toast `X marked present · 3 kept absent`.
   - "Present all" with everyone recorded/absent → toast `All students are already recorded or marked absent`.
   - Left-click flows still behave exactly as before.
