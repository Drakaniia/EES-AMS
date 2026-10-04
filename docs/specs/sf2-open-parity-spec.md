# SF2 Open Parity Spec — make the `.ts` open match the old `.rs` function

**Status:** spec only — no code changes yet
**Date:** 2026-10-03
**Request:** make the SF2 open function in TypeScript match the function in the old Rust files, because the TS migration is not properly applied from the old `.rs` function when opening SF2.

## 1. Problem statement

Pressing **Open SF2** in the TypeScript build produces a broken `.xlsx`:

- date header (row 6) lands in the wrong columns,
- weekday labels (row 7, `M/T/W/TH/F`) are misaligned or missing,
- `X` attendance marks land on the wrong learner rows / day columns,
- roster names / totals / metadata header fields are misplaced vs. the template.

Ground truth per user: the **old `.xls` output** (what the Rust build wrote when opening SF2), not the bundled `.xlsx` template alone and not an arbitrary teacher file. The fix is parity of the open path with the old Rust behavior, adapted from COM/`.xls` to ExcelJS/`.xlsx`.

## 2. Reference functions

### 2.1 Old Rust (HEAD, currently deleted in working tree)

The working tree deleted the Rust backend (uncommitted TS migration); the reference lives in `HEAD`:

- `src-tauri/src/sf2/attendance/attendance_service.rs` :: `sync_and_open_sf2_workbook(app, pool, class_id)` — **the in-scope reference** (user chose "sync_and_open only").
- Same file: `open_read_only_or_refuse`, `sync_attendance_to_sf2_workbook`, `set_preview_attendance_lightweight`, `set_preview_attendance`, `set_all_students_present` — context only, except where roster touches layout (see §7).
- `src-tauri/src/sf2/guard/mod.rs` (`SyncPermit`, `SyncAction`, `action_for`, `guard_before_write`, `run_write_guard`, `stale_abort_message`, `missing_workbook_file_message`) + `guard/evaluate.rs` + `guard/read_only.rs` (`open_workbook_read_only` via Excel COM `ReadOnly:=True`) — **in scope**: user chose "Restore guard" / "Service plus guard".
- `src-tauri/src/sf2/excel/excel_com/calendar.rs` (`set_sf2_month_dates`, `sf2_weekday_slots`, `configure_sf2_calendar`), `worksheet.rs` (`set_sf2_cell`, `merged_target`), `workbook_io.rs` (`write_metadata` → `configure_sf2_calendar`) — calendar/layout half of the parity.
- `src-tauri/src/commands/sf2.rs` — command wiring (`sync_and_open_sf2_workbook`, `open_sf2_workbook` pick-then-act).

Key Rust open sequence (10 progress steps via `emit_sf2_progress(app, "open", n, 10, msg)`):

1. Load template via `latest_template_for_class(class_id)` — `classId`-only, month derived from the single pre-split row. Refuse `No SF2 template imported for this class` when absent.
2. Load student mappings.
3. Check date mappings via `sf2_date_mappings_for_report_month`, then **guard check** (`Checking the workbook against the app…`).
4. `run_write_guard` → `SyncAction`:
   - `Rewrite` → proceed to clear + rewrite.
   - `ReadOnly { reason }` (`Unmeasured`) → `open_read_only_or_refuse`, emit Done, return path with **zero writes**.
   - `Aborted { message }` (`Stale` even after additive import) → `Err(InvalidInput(message))`, nothing cleared.
   - Deliberately **no** "has anything changed since last sync" row-count shortcut — `Proven` always rewrites; `last_synced_at` is record-only.
5. Clear previous marks → compute marks → `write_template_marks_for_days_with_progress` (fine-grained progress inside step 6) → save.
6. `set_last_synced_at(now)`.
7. Existence re-check → `open_path_in_default_app` (step 9 `Opening in Microsoft Excel…`) → Done (step 10).

### 2.2 Current TypeScript (working tree)

- `src/lib/features/sf2/attendance/attendance-service.ts` :: `syncAndOpenSf2Workbook({ classId, reportMonth, progress })` — takes explicit `reportMonth`, resolves via `resolveMonthWriteContext` (per-month model), **no guard** (dropped per spec D15 in favor of differential clear in `attendance-marks.ts`), writes via `writeAttendanceToWorkbook`, stamps `setMonthLastSyncedAt`, returns `sourcePath` (actual OS open done by caller `src/lib/api/sf2.ts` :: `syncAndOpenSf2Workbook` → `openWithOs`).
- `src/lib/features/sf2/attendance/write-context.ts` :: `resolveMonthWriteContext`, `monthAbsences`.
- `src/lib/features/sf2/calendar.ts` :: `sf2WeekdaySlots`, `dayNumbersForSlots`, `setSf2MonthDates` (private), `configureSf2Calendar`, `sf2ReportYear` (June wrap) vs `reportYearForSchoolMonth` (September wrap).
- `src/lib/features/excel/workbook.ts` :: `writableCell`, `writableDayColumns`, `activateSheet`, `openWorkbook`, `saveWorkbookAtomic`.
- `src/lib/features/excel/formulas.ts` — `{ formula, value }` pairs.
- `src/lib/api/sf2.ts` :: `syncAndOpenSf2Workbook`, `openSf2Workbook` (pick-then-act), `killAllExcelProcesses` (throwing stub).

## 3. Decisions from interview

| #   | Question                 | Answer                                                                              |
| --- | ------------------------ | ----------------------------------------------------------------------------------- |
| 1   | Which Rust reference?    | `sync_and_open` only (+ guard/read-only helpers)                                    |
| 2   | Symptom                  | xlsx broken: cells/placement/layout vs template                                     |
| 3   | Guard                    | **Restore guard** (Proven/Stale/Unmeasured + read-only open + abort refusal)        |
| 4   | Signature                | **Hybrid**: keep `reportMonth` param, fall back to latest-template-like lookup      |
| 5   | Broken areas             | All four: row 6, row 7, marks grid, roster+header                                   |
| 6   | Ground truth             | Old `.xls` output                                                                   |
| 7   | Merged cells             | **Fix merged writes** (unmerge-then-write / master-only like Rust `merged_target`)  |
| 8   | Sheet handling           | **Activate and rename** target month sheet (`MONTH YEAR`, visible, active tab)      |
| 9   | Formulas                 | **Full formula parity**: exact Rust formula text + cached values                    |
| 10  | Hybrid order             | **Month-then-fallback**: month write context first, latest-template fallback second |
| 11  | Read-only in TS (no COM) | **Skip writes, open normal**: zero workbook writes, hand path to OS opener normally |
| 12  | Progress                 | **Keep 10 steps/messages** incl. guard-checking message                             |
| 13  | Verification             | **Both**: visual xlsx-vs-xls diff + automated vitest                                |
| 14  | Missing file             | **Keep TS message**: `WORKBOOK_MISSING_MESSAGE` (re-import wording), not Rust E4    |
| 15  | First-school-day         | **Validate strictly**: weekend/out-of-range refuses, no silent fallback             |
| 16  | Scope                    | Open path only **+ roster where it touches the same layout**                        |

## 4. Functional requirements

### 4.1 Hybrid resolution (FR-H)

- `syncAndOpenSf2Workbook` keeps `{ classId, reportMonth, progress }`.
- Empty/whitespace `reportMonth` → refuse `NO_MONTH_SELECTED_MESSAGE` (unchanged).
- Non-empty: `resolveMonthWriteContext(classId, month)` first.
- If month row absent → fall back to latest-template-for-class equivalent (newest `sf2_month_templates` row for the class, else legacy `sf2_templates` row if still present) and derive month from that row — mirrors Rust `latest_template_for_class`. If neither exists → Rust-compatible refusal (`No SF2 template imported for this class` — confirm exact wording against `AppError` contract during implementation; missing-file case separately keeps TS message per §4.5).
- Resolved month still decides all three: whose absences are written, whose day columns they land in, which worksheet is activated. Syncing one month and opening another is a bug.

### 4.2 Guard restoration, ExcelJS-adapted (FR-G)

Port `SyncPermit` / `SyncAction` / `decide` / `action_for` / `guard_before_write` semantics to TS:

- `Proven` requires **both**: `db_count >= workbook_count` AND every workbook `X` is producible by the DB for the mapped month (cell-level match on mapped learner rows × mapped day columns), not count-only.
- `Stale` → run the **additive-only** import (`importAbsentMarks`: X becomes `absent` event only when DB lacks it; never deletes), re-evaluate once (max 2 evaluations). Second `Stale` → `Aborted` refusal with `stale_abort_message(count)` wording; **no output writes, no `last_synced_at` stamp**.
- `Unmeasured` default for: missing file, unreadable workbook, locked/failed measure, empty mapped dates (`NO_MAPPED_DATES`), empty mapped learners (`NO_MAPPED_LEARNERS`). Never clears.
- `ReadOnly { reason }` → **skip all writes**, skip `last_synced_at` stamp, emit read-only progress message, return path for normal OS open (no COM `ReadOnly:=True` exists; the guarantee is "app never writes", mtime untouched by the app).
- No row-count "already in sync, skip write" shortcut on top of the guard. `Proven` always rewrites; `last_synced_at` stays record-only.
- Background `syncSf2Attendance` keeps the same guard: non-`Proven` → log + skip, never fail the triggering attendance action.

### 4.3 Date header + weekday rows (FR-C)

- `sf2WeekdaySlots` must reproduce Rust slot discovery on the old output: skip merge slaves, skip non-weekday labels (`ABSENT`/`PRESENT`), skip blanks; 25 labelled day cells expected.
- `setSf2MonthDates`-equivalent must write **every slot including empties** (clear stale numbers from another month), through the merge master only; fix the F7:G7-style overwrite where writing the second column clobbers the first (unmerge weekday header rows 6–7 before writing, or master-only writes — implementation to confirm against `writableCell`/`writableDayColumns`).
- Strict `firstSchoolDay` validation: out-of-range → `First attendance day must be between 1 and {lastDay} for this report month`; weekend → `First attendance day must be a Monday-Friday school day`. No silent fallback to first Mon–Fri.
- `daysWithoutASlot` still computed and reported; empty for every real month (25 slots ≥ max 23 school days).

### 4.4 Marks grid + roster + header (FR-M)

- X marks land on the intersection of mapped learner rows and mapped day columns only; differential clear bounds what a sync can blank (guard + differential clear are two layers, as in Rust).
- Roster sync included where it writes the same layout (learner names, TOTAL divider gender blocks, learner-ID-merged `No.` cell). Do not invent rows for unmapped learners.
- Header/metadata: 8 fields + `configureCalendar`/`firstSchoolDay` semantics preserved; `configureCalendar=false` means do not touch the calendar.

### 4.5 Sheets, activation, open (FR-S)

- Target = `{MONTH} {reportYear}` sheet; if absent (single-sheet or variant naming) use most-populated monthly sheet (`bestSf2MonthlySheet` quality order: total day cells → learner count → male → female), never silently drop the header.
- Make target `visible`, rename (31-char cap, no uniquify counter), write dates, then set active tab so the OS opens on the synced month.
- Atomic saves only (`saveWorkbookAtomic` / `writeFileAtomic`, temp + rename).
- Missing file on disk → `WORKBOOK_MISSING_MESSAGE` (`The app SF2 working workbook no longer exists. Import the SF2 workbook again`).
- No Excel process: writing is ExcelJS; opening is `@tauri-apps/plugin-opener`. `killAllExcelProcesses` stays a throwing stub.

### 4.6 Formulas (FR-F)

- Every cell that carries a formula also carries the value Excel would cache. Port exact Rust formula text for learner absent/present, totals, and summary marks plus cached values; new formula shape in the template means a new function in `features/excel/formulas.ts` + test.

### 4.7 Progress + errors (FR-P)

- Preserve the 10-step numbers and messages including the guard message (`Checking the workbook against the app…` / `Checking date mappings…` sequence per Rust), adapted from Tauri events to the `Sf2ProgressReporter` callback. `report-sf2-open.svelte.ts` contract unchanged beyond the callback.
- Error strings are a UI contract (`errorMessage` byte-for-byte): reword nothing except where this spec explicitly picks TS vs Rust (§4.5).

## 5. Non-goals

- No COM/Excel automation, no Google Drive/keyring/background scheduler resurrection.
- No `.xls` writing; output stays `.xlsx`. Old `.xls` is the layout oracle, not the output format.
- No second data store; OPFS SQLite only.
- No distributive redesign of the per-month model — parity is scoped to the open path (+ roster layout touches).

## 6. Verification (both required)

1. **Visual xlsx diff:** generate via TS open for a representative month (e.g. a 31-day month starting Monday for max-23-school-day coverage + a weekend-opening month like Nov 2026), open alongside the old `.xls` output, and diff: row 6 numbers, row 7 labels, X placement, roster order, totals formulas, active tab. Reuse/adapt the `debug-sf2-workbook` skill diagnostic (xlrd on the old copy; ExcelJS dump on the new copy) — check merged ranges in rows 6–7 explicitly.
2. **Automated vitest:** new test(s) colocated under `src/lib/features/sf2/__tests__/` (or `attendance/__tests__/`) that load the real bundled template via `template-fixture.ts` + `MemoryFileSystem` + `NodeSqlDriver`, run `syncAndOpenSf2Workbook`, and assert cell-by-cell parity against checked-in old-output expectations (dates, labels, marks, formulas, active sheet). Must run under `bun run test` (not `bun test`).

## 7. Files to touch (implementation phase, not this spec)

- `src/lib/features/sf2/attendance/attendance-service.ts` (guard + hybrid + progress)
- `src/lib/features/sf2/attendance/write-context.ts` (fallback resolver)
- `src/lib/features/sf2/calendar.ts` (`setSf2MonthDates`, slots, validation)
- `src/lib/features/excel/workbook.ts` (merged-cell writes), `src/lib/features/excel/formulas.ts` (formula parity)
- `src/lib/features/sf2/roster/sync.ts` (layout touches only)
- `src/lib/features/sf2/guard/*` (new, if restored as a module) + tests
- Old-Rust oracle paths (read-only): `HEAD:src-tauri/src/sf2/attendance/attendance_service.rs`, `HEAD:src-tauri/src/sf2/guard/*`, `HEAD:src-tauri/src/sf2/excel/excel_com/calendar.rs`, `HEAD:src-tauri/src/sf2/excel/excel_com/worksheet.rs`

## 8. Open questions for implementation

- Exact `latest-template-fallback` query in the per-month schema (newest `sf2_month_templates` row ordering + legacy `sf2_templates` fallback — confirm against `month.ts` read path).
- Whether unmerge-before-write or master-only-write is the ExcelJS-safe fix for rows 6–7 (verify with a workbook round-trip test).
- Exact Rust refusal strings to preserve byte-for-byte (`AppError` contract) — pull from `HEAD:src-tauri/src/domain/error.rs` + `guard` messages.
