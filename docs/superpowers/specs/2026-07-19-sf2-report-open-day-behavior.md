# SF2 Report: Open-Day Behavior & Bulk Present Changes

**Date:** 2026-07-19
**Status:** Draft
**Author:** Buffy (AI Assistant)
**Stakeholder:** Client (Teacher feedback)

---

## 1. Background

### 1.1 Current Behavior (Problem)

The SF2 Excel report currently auto-marks any student as **Absent** (writes "X") for any past school-day date where that student has no recorded attendance event. This matches the standard DepEd SF2 workflow: the Excel form is empty by default, and teachers manually write "X" for absent learners.

However, the EES-AMS system auto-generates these "X" marks after the day ends **even when the teacher never opened the app on that day**. This creates a significant problem:

> **Teachers often batch-process attendance at the end of the month.** If the system auto-fills "X" on all days where no attendance was taken, the entire month shows every learner as absent — which is wrong and requires manual correction of every single cell.

### 1.2 Client Feedback (Verbatim)

> "my client feedback is that.. similar to the sf2 excel - it is empty by default, you just put x mark on the absent, but the system put x after the day ends without attendance. The case sometimes is the teacher just open the app every end of the month meaning no time to attendance daily. So what ui/ux changes in sf2 report that should not put x if the days passed entirely. Or we can just present all button in report page, just like in daily attendance."

### 1.3 Design Philosophy

The SF2 Excel report should match the mental model of the physical SF2 form:

- **Empty = Present** (no mark needed)
- **"X" = Absent** (explicit teacher action)
- The system should NOT assume absence on days where attendance was never taken

---

## 2. Decision Log (from Client Interview)

| #   | Question                                               | Decision                                                                                                             |
| --- | ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| 1   | How to handle days with zero attendance taken?         | **Show all students as "Open"** — not "Absent"                                                                       |
| 2   | What counts as "attendance taken"?                     | **Only "in" events**. If at least one student has an "in" event on a date, that day is considered "attendance taken" |
| 3   | How should non-school days display?                    | **Always show as "Open"** regardless of any events                                                                   |
| 4   | Add a "Present All" button?                            | **Yes** — add to the SF2 report page                                                                                 |
| 5   | Should "Open" cells be clickable/editable?             | **Yes** — teachers can toggle past Open days between Present/Absent                                                  |
| 6   | How should Open cells export to the actual Excel file? | **Leave empty** (the SF2 Excel formula counts empty cells as present)                                                |
| 7   | How should Present status be shown in the UI?          | **Remove the green checkmark entirely.** Empty = Present. Only show "X" for Absent.                                  |

---

## 3. Core Logic Change: Open-Day Detection

### 3.1 New Rule

> **A day is "Open" (no attendance taken) if ZERO "in" attendance events exist for ANY student on that date for the class.**

Previously:

```
if is_present → Present
else if is_future → Open
else → Absent  ← THIS IS THE PROBLEM
```

New logic in `preview.rs`:

```
if is_present → Present (empty / no visual mark)
else if is_future → Open
else if day_has_any_attendance_taken → Absent (X)
else → Open (day was skipped entirely)
```

### 3.2 Detection: `day_has_any_attendance_taken`

- Query: "Are there any 'in' attendance events for _any_ student in this class on this date?"
- If YES → attendance sf2-ewas taken → students without an event are truly **Absent**
- If NO → attendance was NOT taken → show all students as **Open**

### 3.3 Filtering Logic

The "day has any attendance" check and the "is student present" check should both use the same `present_by_day` data already computed in `preview.rs`. The difference is:

- **per-student**: Is THIS specific student present?
- **per-day**: Is ANY student present on this date?

A day qualifies as "attendance taken" when `present_by_day[date]` contains at least one student ID.

### 3.4 Non-School Days

- Non-school days (weekends/holidays outside the class schedule) should always show as **Open**
- This applies regardless of whether any events exist on those dates
- A class schedule day matches when the `Class.days` array includes the day-of-week for that date

---

## 4. Preview UI Changes

### 4.1 Visual Status Changes

The three statuses in the SF2 report table (`report-table.svelte`) change as follows:

| Status  | Current Visual                                   | New Visual                                                      | Meaning                                                       |
| ------- | ------------------------------------------------ | --------------------------------------------------------------- | ------------------------------------------------------------- |
| Present | Green checkmark (`Check` icon, green background) | **Empty cell** (no icon, no colored background, neutral border) | Student was present for that day                              |
| Absent  | Red "X" (`X` icon, red background)               | **Red "X"** (`X` icon, red background) — **unchanged**          | Student was absent on a day where attendance was taken        |
| Open    | Dash ("-", gray/muted)                           | **Dash ("-")** — gray/muted (as-is)                             | Day is in the future, or no attendance was taken for this day |

### 4.2 Cell Classes (`report-state.svelte`)

Update `cellClass()` function:

- `present` → `bg-background text-muted-foreground border-border` (neutral, no green)
- `absent` → `border-red-500/35 bg-red-50 text-red-700` (unchanged)
- `open` → `border-border bg-background text-muted-foreground` (unchanged)

### 4.3 Cell Rendering (`report-table.svelte`)

- `present` → render nothing (empty cell, `<span>-</span>` but without the dash? or just fully empty)
  - Client preference: completely remove the checkmark. Empty cell = Present.
- `absent` → render `X` icon (unchanged)
- `open` → render `-` (unchanged)

### 4.4 Editability

- **Present** cells: clickable, toggle to Absent
- **Absent** cells: clickable, toggle to Present
- **Open** cells on PAST dates: **clickable** — clicking toggles to Absent (X)
  - When clicking an Open cell: it toggles directly to Absent (X)
  - This matches the teacher's mental model: "click open = first toggle goes to absent"
- **Open** cells on FUTURE dates: **not clickable** (unchanged)

### 4.5 Legend Update

Update the legend chips in `report-table.svelte` to reflect the new meaning:

```
Already shown:      [Absent]  [Open day]
New to add:         [Present] (shown as empty — no icon needed)
```

Or simplify to just:

```
[X] Absent    [-] Open day (no attendance taken yet)
```

Since "Present" is now invisible/empty, it may be helpful to add a small legend note:
"Empty cells = Present (no mark needed)"

---

## 5. "Present All" Button

### 5.1 Location

Add a **"Present all"** button to the SF2 report page, similar to the one on the daily attendance page.

Suggested location: In the report page header action bar, near the existing "Sync Roster" and "Open SF2" buttons.

### 5.2 Behavior

**What it does:**

- Clears ALL "X" (Absent) marks for the current report month's visible students, setting every cell to Present (empty)
- Does NOT affect Open days (they're already empty = Present)

**In other words:** The "Present All" button resets all attendance marks to the default state (all Present), allowing the teacher to then selectively mark only those students who were actually absent.

### 5.3 Implementation

- The button calls a new Rust command or reuses the existing `toggleSf2PreviewAttendance` logic in batch
- For each Absent cell: create an "in" attendance event to mark the student present
- For Open cells (no attendance taken): leave them as-is (empty count as present in SF2)
- The button should show loading state while processing

### 5.4 UX Details

- Button label: "Present all"
- Disabled state: when no Absent cells exist to clear
- Loading state: show spinner and "Clearing marks..."
- Success toast: "All students cleared to Present"
- The same "correctingCellKey" lock mechanism should be used to prevent concurrent operations

---

## 6. Export Behavior

### 6.1 What Gets Written to Excel

| Cell Status                | What gets written to Excel |
| -------------------------- | -------------------------- |
| Present                    | Empty string (no mark)     |
| Absent                     | "X"                        |
| Open (past, no attendance) | Empty string (no mark)     |
| Open (future)              | Empty string (no mark)     |

### 6.2 Rationale

Per client: "Leave Open cell empty — empty cell is counted as present by the SF2 Excel formula."

The existing SF2 TOTAL formulas use `COUNTIF(range,"X")` to count absents, then subtract from the total. Empty cells are skipped by COUNTIF, so they are implicitly counted as present.

### 6.3 Implementation Impact

In `attendance_service.rs`, the `export_marks` function currently:

1. Filters for past days
2. For each past day, writes "X" for students without present events

**Changes needed:**

- Check if the day has ANY attendance events (any "in" event for any student)
- If no attendance was taken on that day → skip writing marks for that day entirely
- Write marks only for days where attendance was actually taken

This is the same per-day logic used in the preview. The `export_marks` function should share the same "day has attendance" check.

---

## 7. Files to Modify

### Rust Backend

| File                                      | Changes Required                                                                                |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `src-tauri/src/sf2/preview.rs`            | Modify `export_preview()` to check if a day has ANY attendance taken before marking as Absent   |
| `src-tauri/src/sf2/attendance_service.rs` | Modify `export_marks()` to skip days where no attendance was taken for any student              |
| `src-tauri/src/sf2/logic.rs`              | Potentially update `attendance_marks_for_closed_day` or add helper for per-day attendance check |

### Svelte Frontend

| File                                              | Changes Required                                                          |
| ------------------------------------------------- | ------------------------------------------------------------------------- |
| `src/routes/reports/report-table.svelte`          | Remove checkmark icon for Present; update cell rendering for three states |
| `src/routes/reports/report-state.svelte`          | Update `cellClass()` function — remove green styling for present          |
| `src/routes/reports/+page.svelte`                 | Add "Present all" button in the action bar; add handling logic            |
| `src/routes/reports/report-export-dialogs.svelte` | No changes needed unless export summary text changes                      |

### Types

| File                | Changes Required                                                   |
| ------------------- | ------------------------------------------------------------------ | -------- | -------------------------- |
| `src/lib/types.ts`  | `Sf2PreviewCellStatus` already has `'present'                      | 'absent' | 'open'` — no change needed |
| `src/lib/db-rust/*` | May need a new Tauri command for "Present All" (bulk mark present) |

---

## 8. Edge Cases

### 8.1 Mixed Day (Some Present, Some Absent)

This is the normal case: attendance was taken, and some students were present while others were absent.

- Students with "in" events → **Present** (empty)
- Students without "in" events → **Absent** (X)
- This behavior is **unchanged** from current.

### 8.2 All Present Day

If all students were marked present on a day:

- Every student → **Present** (empty)
- The teacher sees no marks at all for that day, which is correct

### 8.3 All Absent Day (Theoretical)

If a teacher marks 0 students on a day where they DID take attendance (unusual but possible):

- Since there are 0 "in" events → day appears as **Open** (no attendance taken)
- This is technically correct: if no one was marked, attendance wasn't really "taken"
- If the teacher wants to record "everyone absent" they should manually mark each student as absent via the report

### 8.4 Date Range Boundary

- Dates BEFORE the month starts or AFTER the month ends: not in the date mappings, ignored
- Dates in the current month but outside the class schedule (weekends): show as **Open**
- This is already handled by the week-group building logic

### 8.5 Month Switch

When switching between report months (July → August), the per-day attendance data changes. The new logic works per-day based on the events in the database, so switching months works correctly without changes.

### 8.6 "Present All" on Partial Data

If the teacher has already marked some students present and some absent:

- "Present all" should clear all X marks → all students = Present
- Already-Present students stay Present (no-op)
- Open days stay Open

---

## 9. Open Questions

1. **"Present All" scope**: Should "Present All" apply only to the currently visible gender-filtered students, or to ALL students in the class? (Clarified during implementation: apply to all mapped students in the class)

2. **Toast messages**: What should the success message say after Present All? "All students marked as present"?

3. **Undo support**: Should Present All support an undo action? This is complex since it changes many cells at once. Likely no, unless requested.

---

## 10. Implementation Order

Phase 1: **Preview Logic Change** (Rust)

- Modify `preview.rs` to use per-day attendance check
- Add helper to check if any events exist for a date

Phase 2: **Export Logic Change** (Rust)

- Modify `attendance_service.rs` `export_marks()` to skip days without attendance

Phase 3: **UI Status Update** (Svelte)

- Remove checkmark icon for Present status
- Update cell styling and legend

Phase 4: **Present All Button** (Rust + Svelte)

- Add Tauri command for bulk clear/present
- Add button to report page
- Wire up loading states and toasts
