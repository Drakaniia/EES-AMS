# SF2 Per-Month Workbooks & Attendance Durability — Spec

**Status:** In progress — **D2/D4 reversed 2026-09-27, see §0**
**Date:** 2026-09-27
**Scope:** `src-tauri/src/sf2/`, `src-tauri/src/backup/`, `src-tauri/src/commands/`, `src/routes/reports/`, `src/routes/settings/`
**Schema target version:** 19 → 22

---

## 0. AMENDMENT (2026-09-27) — the workbook is ONE file with 12 month sheets

**This section overrides D2 and D4, and §6.1, §6.2 v19–v21, and §11.**

D2 ("one `.xls` per month, 12 files") and D4 ("each month file keeps only its own month tab") were a
**mis-transcription of the request.** §1 quotes the request as _"only one .xlsx file with different
worksheet tabs to all month"_. The implementation followed D2 instead of the request, and the user
confirmed the original intent when shown the result.

### A1 — Superseded decisions

| ID  | Was                                                                    | **Now**                                                                                                 |
| --- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| D2  | One `.xls` per month, 12 files                                         | **ONE `.xls` file, containing 12 month worksheets**                                                     |
| D4  | Each month file keeps only its own month tab; the other 11 are deleted | **All 12 month sheets are kept, visible, and each carries its own day-number grid and its own X marks** |

The bundled DepEd template already ships 12 month sheets (`JUNE`…`APRIL`). The target file is that
template with all 12 populated, not a template trimmed to one.

### A2 — Why the original code was restructured away, and why that was wrong

The old `configure_sf2_calendar` hide/rename/clear cycle existed only to make a single-sheet model work:
it made the target month's sheet visible, renamed it, and cleared the other 11 so `analyze_workbook`
(which reads visible monthly sheets only) would return one month. Under A1 that cycle is **unnecessary
and must be deleted** — every sheet stays visible and populated, so no configuration pass is needed to
select a month, and a month switch is a pure read.

**But the per-month TABLES were also split, and that split is what broke the user's grid.** See A3.

### A3 — The regression this amendment fixes (acceptance failures in the shipped work)

The reports grid reads `sf2_month_date_mappings` / `sf2_month_student_mappings`. On an existing install
those tables are populated only by the v22 backfill, for one month. The 12-month split that would
populate the rest is **never invoked from anywhere**, so the grid renders empty and every cell is
disabled (`report-table.svelte:359` — `disabled={!cell.editable || !row.mapped}`). The user reported
X marks present in the `.xls` but absent from the grid, and the grid unclickable. Both are the same
cause.

### A4 — Revised data model (supersedes §6.2 v19–v21)

| Concern                      | Model                                                                                                                                                                                                                                                                                            |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Workbook file                | **One file.** `sf2-workbooks/SF2-{grade}-{section}-{id8}.xls`, all 12 month sheets.                                                                                                                                                                                                              |
| `sf2_month_templates`        | **One row per month still** — it identifies a month's grid, not a file. `source_path` is the same file for all 12 rows. `workbook_x_count` is per month.                                                                                                                                         |
| `sf2_month_date_mappings`    | **Must regain a `sheet_name` (or `report_month`) column.** The date is a full `YYYY-MM-DD`, so dates do not collide across months, but the **sheet** must be resolvable to read or write a mark. This is the column D-dropped in v21; under A1 it comes back.                                    |
| `sf2_month_student_mappings` | **One shared roster, not 12.** There is one class; the roster is the roster. `PRIMARY KEY(template_id, student_id)` on a _shared_ row set is what the legacy `sf2_student_mappings` already is, and reusing it is the lower-risk path. Per-month rosters (the D2-era rationale, E8) are dropped. |
| Month switching              | Still instant and Excel-free (D9): a month switch reads `sf2_month_date_mappings` filtered by month, plus `events`.                                                                                                                                                                              |
| Selecting a month in Excel   | **Not needed.** All sheets exist and are visible. The "Switch month" button in the app changes only the _app's_ view.                                                                                                                                                                            |

### A5 — Data-safety constraint, binding above all other sections

**On the affected install the `.xls` is the only known copy of the X marks.** The `.db`'s contents are
unverified. Therefore, until a mark-count comparison proves the database holds at least what the
workbook shows:

1. **No code path may write to, clear, or re-save the workbook.** The §9.1 guard's `Proven` permit must
   additionally require that the mark comparison has actually been run and recorded for that month.
2. A **read-only** comparison (workbook X count vs DB absent count, per month) must be run and its
   result shown to the user before any write path is enabled.
3. If the workbook holds marks the database lacks, they are **imported first**, then the normal guard
   applies. The `.xls` is the recovery source, not a derived mirror, until proven otherwise.

---

## 1. The request (as stated)

> why is the x mark missing always when i try to install new update. is it stored in the old .xlsx file? is it not in the .db file so its backupable?, also slow loading when switching account, can we make it that only one .xlsx file with different worksheet tabs to all month, when i open sf2 should open current month, remove the button to import x from workbook, the data X is critical so it should not left or wipe when new update.

Broken into four asks:

| #   | Ask                                                                                                       | Section |
| --- | --------------------------------------------------------------------------------------------------------- | ------- |
| A   | Find out why X marks disappear on app update, and whether the marks are in the `.db` or only the workbook | §4      |
| B   | Make month switching fast (currently shows a loading modal)                                               | §7      |
| C   | Restructure the workbook model to one file per month, auto-opening the current month                      | §6      |
| D   | Remove the "Import X from Workbook" button; make X marks impossible to lose or wipe on update             | §8, §9  |

---

## 2. Corrections to the premise

Two things in the request are factually wrong about the current codebase. Recording them here so nobody re-litigates them mid-implementation.

| Premise                                        | Reality                                                                                                                                                                                                                                                                                                              |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| "is it stored in the old .xlsx file?"          | The files are **`.xls`** (Excel 97‑2003), not `.xlsx`. The bundled DepEd template is `src-tauri/resources/sf2/TEMPLATE_AUTOMATED_SF2.xls`, copied byte-for-byte into a per-class working copy.                                                                                                                       |
| "is it not in the .db file so its backupable?" | **The X marks ARE in the `.db`.** An absence is a real row in `events` with `event_type = 'absent'` — a first-class record introduced in schema v17 (`src-tauri/src/sf2/sql/migrate_to_v17.sql:9`). The `.db` backup is lossless and does contain them. The workbook is a _derived mirror_, not the source of truth. |

The actual problem is the opposite of "not backed up": **the app destroys the workbook's copy when it believes the database is authoritative, and it does this without checking whether the database actually agrees.**

---

## 3. Current state (code map)

### 3.1 Storage layout

| Thing                   | Location                                                                      | Notes                                                           |
| ----------------------- | ----------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Live database           | `%APPDATA%\com.ees.ams\attendance.db`                                         | `src-tauri/src/lib.rs:196`                                      |
| Pre-migration snapshots | `%APPDATA%\com.ees.ams\attendance.db.pre-v{N}-to-v{M}`                        | 3 kept, `migrations.rs:38`                                      |
| Local backups           | `%APPDATA%\com.ees.ams\backups\*.db`                                          | **`.db` only** — `backup_ops.rs:68` filters `extension == "db"` |
| Sync folder copy        | `<chosen folder>\EES-AMS-Backups\*.db`                                        | `.db` only — `backup_ops.rs:275`                                |
| Google Drive            | one `.db` object per backup                                                   | `.db` only — `google_drive.rs`                                  |
| SF2 working copies      | `%APPDATA%\com.ees.ams\sf2-workbooks\SF2-{grade}-{section}-{templateId8}.xls` | `workbook_files.rs:22-27`, one file per class                   |

**Consequence:** the 12 workbooks are the only artifacts in the app that no backup, sync, or export path ever touches.

### 3.2 The X mark

- `src-tauri/src/sf2/logic.rs:9` — `SF2_ABSENT_MARK = "X"`, `SF2_PRESENT_MARK = ""` (present is a blank cell).
- Written to Excel by `attendance_marks::export_marks` (`attendance_marks.rs:14`) — sparse: a cell gets `"X"` only if an `absent` event exists for that student and day.
- The formulas that make Excel count the Xs (`COUNTIF(...,"X")` in `total_formula_marks` and `learner_absent_present_formula_marks`) live in the same workbook and are regenerated on every sync.
- Read back out of the workbook by `import_absent_marks_from_workbook` (`attendance_import.rs:53`) — the manual recovery the user wants to remove.

### 3.3 Workbook internals (today)

The bundled template already contains **12 month sheets** (`JUNE`…`APRIL`). On every calendar configuration, `configure_sf2_calendar` (`excel_com/calendar.rs:15`):

1. Makes the target month's sheet visible and renames it `"{MONTH} {year}"`.
2. Writes that month's day numbers into row 6, columns F..AL.
3. For **every other** sheet: clears its row‑6 day numbers, renames it `__SF2_HIDDEN_{n}`, and sets `Visible = 0`.
4. `analyze_workbook` (`workbook_analysis.rs:54`) then **only reads visible monthly sheets** — so after any refresh the database holds date mappings for exactly one month.

This hide/rename/clear cycle is the entire cost of a month switch, and it is why the file _feels_ like it has "tabs for all months" already but behaves as if it has one.

### 3.4 Month switch cost

`set_report_month` → `refresh_template_calendar_from_saved_month(pool, template, force_refresh = true)` (`template_ops.rs:106`) → full Excel COM session: `write_metadata` (which calls `configure_sf2_calendar`), `analyze`, clear TOTAL rows, rewrite `COUNTIF` formulas, rewrite AM/AO formulas, rewrite the AR/AS/AT summary block, then `update_template_with_mappings`. Thousands of COM round-trips. The `ReportMonthSwitchOverlay` modal exists to cover it.

---

## 4. Root cause: how an update destroys the X marks

The `.db` is the source of truth and the `.xls` is a mirror. The write path clears the mirror unconditionally and re-fills it from the database. There is **no check that the database actually holds the marks before the clear happens.** One failure in the refresh step therefore escalates from "grid is empty" to "marks are gone from both copies, permanently."

### The chain

**Step 1 — Trigger.** After an update the user opens Reports, clicks _Switch month_, _Open SF2_, or _Review Export_. Any of these calls `refresh_template_calendar_from_saved_month`. Post-update this is more likely to fail because Excel COM is more likely to be unavailable (a stale `EXCEL.EXE` from before the install, a workbook the user still has open, or the installer having just run).

**Step 2 — Degenerate analysis.** `excel::batch_operations` runs `session.analyze()`. `analyze_workbook` returns dates **only for visible monthly sheets** (`workbook_analysis.rs:54`: `if visible != EXCEL_SHEET_VISIBLE { continue }`). If `configure_sf2_calendar` failed to make the target sheet visible, or renamed it to something `month_number()`/`year_from_sheet_name()` cannot parse, the analysis comes back with **zero dates**. There is no guard rejecting a degenerate result.

**Step 3 — Mappings wiped, transaction commits.** `refresh_template_calendar_from_saved_month` calls `update_template_with_mappings` (`repository.rs:110`), which inside its transaction runs `DELETE FROM sf2_date_mappings WHERE template_id = ?1` (`repository.rs:173`) and re-inserts the possibly-empty set, then commits. **Every month in the school year now has zero date mappings.**

**Step 4 — Grid goes blank.** `export_preview` and `get_sf2_workbook_settings` both derive the calendar from `sf2_date_mappings_for_report_month`. Zero mappings → no weekday columns → the Reports grid renders empty. The user sees their month has vanished.

**Step 5 — The mirror is destroyed.** On the next _Open SF2_ or _Review Export_, `write_template_marks_for_days_impl` (`progress.rs:126`) runs. `attendance_changed_since(last_synced_at, latest_event_at)` (`calendar/mod.rs:217`) returns `true` whenever any event exists that is newer than the last sync — which is the normal case. So the write proceeds:

- `clear_date_mappings` resolves to **all** date mappings for the template (`progress.rs:154-158`), which is now empty, so it falls back to `date_mappings` — also empty.
- `attendance_marks::export_marks` produces an empty mark set.
- Phase 1 of the write session calls `session.clear_attendance_grid(sheet_name, …)` for every sheet in the (empty) mapping set — and `clear_attendance_grid` clears **the whole attendance range of the sheet**, i.e. all 33 weekday columns × all roster rows, not just the mapped days.
- Phase 3 then writes nothing.

**Result: the workbook's attendance grid is now completely blank, and the database has no marks to put back.**

### Why the pre-install backup does not save you

`install_staged_inner` (`commands/updates.rs:345`) writes a `.db` backup before launching the installer. That backup is a _correct_ copy of the database. But the failure above happens **after** the backup, in normal app operation, and by the time anyone notices the grid is blank the `.db` is also empty of marks for that month. Restoring the pre-install backup restores a database whose `sf2_date_mappings` are already degenerate — so the guard in §9 still refuses to clear and still refuses to show marks. The backup is a rollback point for the _installer_, not for _this_ bug.

### Contributing factors

| #   | Factor                                                                                                                      | Location                                                      |
| --- | --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| 1   | A degenerate (empty) date analysis is committed as if valid                                                                 | `repository.rs:173` — no `if dates.is_empty() { return Err }` |
| 2   | Date mappings are deleted for the whole template, not scoped per month                                                      | `delete_date_mappings.sql`                                    |
| 3   | Clearing is unconditional and total, not diff-based                                                                         | `progress.rs:180-200`, `attendance_marks.rs:67`               |
| 4   | `attendance_changed_since` treats "the DB has fewer marks" as "the workbook is stale"                                       | `calendar/mod.rs:217`                                         |
| 5   | Workbooks are absent from every backup target                                                                               | `backup_ops.rs:68`, `backup_ops.rs:275`                       |
| 6   | The JSON export/import path silently converts every absence into a present                                                  | `commands/data_transfer.rs:141` — hardcoded `"in"`            |
| 7   | `migrate_to_v11.sql:3` — `DELETE FROM events WHERE event_type <> 'in'` — destroys absences on any database crossing v10→v11 | `migrate_to_v11.sql:3`                                        |
| 8   | No migration-time assertion that `events` row count is preserved                                                            | `migrations.rs:124`                                           |

> **Note on factor 6.** `Export JSON` → `Import JSON` is a one-click data-loss path available today in Settings. It is independent of updates and worth fixing on its own merits.

---

## 5. Decisions

Recorded from the interview. Each is binding unless a later section contradicts it.

| ID  | Decision                                                                                                                                                                               | Rationale given                                                                                                                 |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| D1  | X marks are the critical record. They must never be deleted by an app action without the database proving it holds them.                                                               | "the data X is critical so it should not left or wipe when new update"                                                          |
| D2  | **One `.xls` file per month** — 12 files total. Not per class, because there is only one class.                                                                                        | "One class only — 12 files total is right"                                                                                      |
| D3  | Keep **`.xls`** (Excel 97‑2003). Do not convert to `.xlsx`.                                                                                                                            | Avoids DepEd layout/macro risk and Excel's conversion prompt.                                                                   |
| D4  | Each month file keeps **only its own month tab**; the other 11 tabs are deleted.                                                                                                       | `analyze_workbook` only reads visible monthly sheets, so the rest are dead weight. Removes the hide/rename/clear code entirely. |
| D5  | On app start, the app opens **today's calendar month**, falling back to the last-used month if that month's file does not exist.                                                       | "Today's month, always"                                                                                                         |
| D6  | Remove the **"Import X from Workbook"** button. Replace it with **automatic self-heal at startup**.                                                                                    | "Auto self-heal on app start, no button"                                                                                        |
| D7  | Destructive sync is blocked unless **the database holds at least as many absences as the workbook shows X marks**. If it does not, the app imports the missing ones first, then opens. | "Never clear unless the .db is proven good"                                                                                     |
| D8  | Startup self-heal scans **only the current month's file**.                                                                                                                             | Speed; avoid Excel contention at launch.                                                                                        |
| D9  | Month switching must be **under 300 ms with zero Excel COM**. No modal.                                                                                                                | "Instant switch, no modal at all"                                                                                               |
| D10 | The month picker dialog and its _Switch month_ button stay, but become instant.                                                                                                        | "Keep the month picker, but make it instant"                                                                                    |
| D11 | Backups become a **folder per backup** containing `attendance.db` plus a `workbooks/` subfolder. Workbooks go to the sync folder and Google Drive too.                                 | "A folder per backup: attendance.db + workbooks/"                                                                               |
| D12 | Also snapshot the workbooks **before risky moments**: update install, wipe, and any grid clear.                                                                                        | Chosen in addition to D11.                                                                                                      |
| D13 | Add a **"Back up workbooks now"** button in Settings → Data Management.                                                                                                                | Chosen in addition to D11/D12.                                                                                                  |
| D14 | Existing per-class workbook is **split into 12 per-month files** on first launch after upgrade, verified before the original is retired.                                               | "Split the existing .xls into 12 per-month files"                                                                               |
| D15 | Keep the `classes` table and all class-aware code. Do not lock the app to one class in the schema.                                                                                     | "Keep the classes table, just don't create extra workbooks"                                                                     |
| D16 | Each month's **first attendance day is derived** from a single new setting: the real date classes started. Overridable per month.                                                      | "I'll enter the real start date once"                                                                                           |
| D17 | Fix the JSON import to preserve `event_type`.                                                                                                                                          | "Fix it — preserve event_type on import"                                                                                        |
| D18 | Remove from Settings → SF2: _Create From Template_, _Import SF2_, _Sync Roster_. Reduce Settings → Classes to a single read-only record.                                               | See §12 — the answer to this question was self-contradictory; see §12.1.                                                        |

---

## 6. Target data model

### 6.1 Files on disk

```
%APPDATA%\com.ees.ams\
├── attendance.db
├── attendance.db.pre-v18-to-v22            (pre-migration snapshot)
├── sf2-workbooks\
│   ├── SF2-SEPTEMBER-2026.xls              ← 1 visible sheet: "SEPTEMBER 2026"
│   ├── SF2-OCTOBER-2026.xls
│   ├── SF2-NOVEMBER-2026.xls
│   ├── SF2-DECEMBER-2026.xls
│   ├── SF2-JANUARY-2027.xls
│   ├── SF2-FEBRUARY-2027.xls
│   ├── SF2-MARCH-2027.xls
│   ├── SF2-APRIL-2027.xls
│   ├── SF2-MAY-2027.xls
│   ├── SF2-JUNE-2027.xls
│   ├── SF2-JULY-2027.xls
│   ├── SF2-AUGUST-2027.xls
│   └── _legacy\
│       └── SF2-1-A-1a2b3c4d.xls             (the pre-split per-class file, read-only)
└── backups\
    └── 2026-09-27-0830\                     (folder per backup — D11)
        ├── attendance.db
        ├── manifest.json
        └── workbooks\
            ├── SF2-SEPTEMBER-2026.xls
            └── …
```

Each `.xls` contains exactly one worksheet named `"{MONTH} {year}"` (e.g. `SEPTEMBER 2026`), plus whatever non-monthly helper sheets the bundled template ships that are not month sheets (the analyzer already skips those via `sheet_is_analysis_candidate`).

### 6.2 Schema changes

`CURRENT_SCHEMA_VERSION` 18 → 22.

#### v19 — `sf2_month_templates`

One row per month file. Replaces the "one template with a mutable `report_month`" model.

```sql
CREATE TABLE IF NOT EXISTS sf2_month_templates (
    id                TEXT PRIMARY KEY NOT NULL,
    active_class_id   TEXT NOT NULL,
    school_year       TEXT NOT NULL,
    report_month      TEXT NOT NULL,          -- 'SEPTEMBER'
    report_year       INTEGER NOT NULL,       -- 2026
    source_path       TEXT NOT NULL,
    source_hash       TEXT NOT NULL,
    school_id         TEXT,
    school_name       TEXT,
    grade_level       TEXT,
    section           TEXT,
    adviser_name      TEXT,
    school_head_name  TEXT,
    first_school_day  INTEGER NOT NULL,       -- per-month, derived or overridden
    imported_at       INTEGER NOT NULL,
    last_synced_at    INTEGER,
    workbook_x_count  INTEGER NOT NULL DEFAULT 0,   -- last observed X count in the file
    workbook_scanned_at INTEGER,                    -- when workbook_x_count was measured
    UNIQUE(active_class_id, school_year, report_month),
    FOREIGN KEY(active_class_id) REFERENCES classes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_sf2_month_templates_class
    ON sf2_month_templates(active_class_id, school_year);
```

#### v20 — `sf2_month_student_mappings` (replaces `sf2_student_mappings`)

```sql
CREATE TABLE IF NOT EXISTS sf2_month_student_mappings (
    template_id     TEXT NOT NULL,
    student_id      TEXT NOT NULL,
    workbook_name   TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    row_index       INTEGER NOT NULL,
    gender_block    TEXT,
    sf2_learner_id  TEXT,                      -- NEW — see §6.3
    PRIMARY KEY(template_id, student_id),
    UNIQUE(template_id, normalized_name),
    FOREIGN KEY(template_id) REFERENCES sf2_month_templates(id) ON DELETE CASCADE,
    FOREIGN KEY(student_id)  REFERENCES students(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_sf2_month_student_mappings_student
    ON sf2_month_student_mappings(student_id);
```

#### v21 — `sf2_month_date_mappings` (replaces `sf2_date_mappings`)

```sql
CREATE TABLE IF NOT EXISTS sf2_month_date_mappings (
    template_id   TEXT NOT NULL,
    date          TEXT NOT NULL,
    column_letter TEXT NOT NULL,
    column_index  INTEGER NOT NULL,
    PRIMARY KEY(template_id, date),
    FOREIGN KEY(template_id) REFERENCES sf2_month_templates(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_sf2_month_date_mappings_date ON sf2_month_date_mappings(date);
```

`sheet_name` is dropped: a per-month file has one sheet, so the column is constant and derivable from `sf2_month_templates.report_month`. This is what makes month switching a pure SQL read — no join, no sheet resolution.

**Populated eagerly for all 12 months at workbook-creation time.** Every month's day-number grid is computed from the calendar and `first_school_day` and written to the `.xls` in one batch, so no month ever needs a COM round-trip to become usable.

#### v22 — settings additions + backfill

```sql
ALTER TABLE settings ADD COLUMN school_start_date TEXT DEFAULT NULL;   -- 'YYYY-MM-DD'
ALTER TABLE settings ADD COLUMN last_report_month TEXT DEFAULT NULL;   -- 'SEPTEMBER'
ALTER TABLE settings ADD COLUMN sf2_split_completed_at INTEGER;
```

Backfill, per existing `sf2_templates` row: create a `sf2_month_templates` row for the template's own `report_month` pointing at the existing `source_path`; copy `sf2_student_mappings` → `sf2_month_student_mappings`; copy that month's `sf2_date_mappings` → `sf2_month_date_mappings`. Existing installs keep working before the split runs (§9.4).

`sf2_templates` and its two mapping tables are dropped in a later migration once the split is verified on all supported installs. They are **not** dropped in v22.

### 6.3 `sf2_learner_id` — the missing "ID"

The request asks whether an identifier survives an update. The answer today is that the DepEd learner ID is **never read from the workbook at all**: `workbook_learners` (`excel_com/learners.rs:7`) only reads column 3 (name) and infers gender from the MALE/FEMALE block headers. The only identity link between the app and the school record is `row_index` plus a normalized name.

That is fragile — a roster reshuffle between months silently re-points a student at another student's X marks. `sf2_learner_id` fixes it:

- Read the DepEd ID column (column 2 in the bundled template; verified against `TEMPLATE_AUTOMATED_SF2.xls` at implementation time) during `workbook_learners`.
- Store it on the mapping and on `students` (nullable, so existing rows are unaffected).
- Matching order for roster sync becomes: `sf2_learner_id` → `normalized_name` → `row_index`.

This is a **schema addition only**; it does not by itself fix the disappearing marks, but it removes one of the ways marks get attached to the wrong person.

---

## 7. Fast month switching (D9)

### 7.1 What runs on a month switch after this change

```
$state.reportMonth = 'OCTOBER'
  └─ $derived → matrixWeekGroups / matrixStudents  recomputed from preview.dates
  └─ one cached getSf2MonthPreview('OCTOBER')  →  SELECT from sf2_month_date_mappings + events
```

No `invoke` to a mutating command. No `setSf2ReportMonth`. No Excel. No overlay.

### 7.2 Changes

| Change                                                                                                                    | File                                                 |
| ------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| Delete `ReportMonthSwitchOverlay` usage from `reports/+page.svelte`; delete the component                                 | `src/routes/reports/ReportMonthSwitchOverlay.svelte` |
| Delete `onReportMonthChange`'s Excel path; `onMonthSelect` just sets `reportMonth` and awaits the preview                 | `report-page-state.svelte.ts:356-482`                |
| Delete `set_sf2_report_month` command + `set_report_month{,_with_progress}` service                                       | `commands/sf2.rs`, `template/template_ops.rs:38-131` |
| Delete `configure_sf2_calendar`'s hide/rename/clear loop, leaving only date-header writing for the single remaining sheet | `excel_com/calendar.rs:15-69`                        |
| New `getSf2MonthPreview(month)` command; preview cache keyed `${classId}:${schoolYear}:${month}`                          | new                                                  |
| `attendance_changed_since` and its `None/None` heuristic are deleted — the guard in §9.1 replaces them                    | `calendar/mod.rs:217`                                |

### 7.3 Performance target

Measured as the time from the month-picker click to the grid painting the new month's weekday headers, in a release build with 40 students:

- **p50 < 150 ms, p95 < 300 ms, p99 < 500 ms.** Asserted by a `bench`-style integration test over `buildMatrixWeekGroups` + a mocked `getSf2MonthPreview`.
- Zero `sf2-progress` events emitted during a switch. A regression test asserts none are observed.

### 7.4 Class switching

The user also reports class switching is slow. Since D15 keeps the `classes` table, class switching is a different code path from month switching: today the Reports page loads the class list but exposes no switcher. The spec adds a class selector to the Reports sidebar (cheap — a `<select>` over `listClasses()` plus the same `getSf2MonthPreview` call) and removes the full-page `ReportLoadingStates` blocking state in favour of a skeleton, so switching never blanks the screen. `loadReport`'s redundant `setTimeout(0)` yield (`report-page-state.svelte.ts:152`) and the double-fetch on cache miss are removed.

---

## 8. Self-heal and the removal of the import button (D6, D8)

### 8.1 What replaces the button

`import_absent_marks_from_workbook` (`attendance_import.rs:53`) stays as an internal service. The button and its two call sites in `ReportSidebar.svelte` (lines 98-112 and 250-262) are removed, along with:

- `onImportAttendance` in `report-page-state.svelte.ts:220-246`
- `canRecoverFromWorkbook` in `ReportSidebar.svelte:66`
- the `importingAttendance` prop threading through `+page.svelte:117,124` and the sidebar
- the `import_sf2_attendance_from_workbook` Tauri command registration (`lib.rs:161`) — **replaced**, not deleted; the service is still reachable from Rust (§8.2)

### 8.2 Startup sequence

New Rust command `heal_current_month_workbook() -> Sf2HealOutcome`, called once from `app_lib::run`'s `setup` **after** `init_db`, on a background thread so startup is not blocked:

```
1. Resolve target month per D5:
     month := today's calendar month
     if the file for `month` does not exist → fall back to settings.last_report_month
2. Load the sf2_month_templates row. If none → return Ok(NotApplicable).
3. Count X marks in the workbook's single sheet across the mapped learner rows ×
   mapped date columns. One COM read pass, no writes.
4. Compare against the DB's absent-event count for that month:
     db_count >  workbook_count  → nothing to do
     db_count == workbook_count  → update sf2_month_templates.workbook_x_count; done
     db_count <  workbook_count  → import the (workbook_count - db_count) missing
                                   absences, reason = "SF2 workbook self-heal"
5. Persist the measured workbook_x_count + workbook_scanned_at.
6. Emit a toast: "Recovered N X marks from SF2-<MONTH>.xls" when step 4 imported.
```

Constraints:

- **Read-only unless it is importing.** The heal never writes to the workbook.
- If Excel COM is unavailable, step 3 returns `ExcelUnavailable`; the app starts normally and §9.1's guard takes over on the next Open/Export.
- Runs **once per launch**, guarded by an in-process `AtomicBool`, and only for the current month (D8).
- Must not run while another Excel task is in flight — it takes the same `run_excel_task` serialisation as every other COM path.

### 8.3 Why this is safe to run unattended

The import is **additive only** (`attendance_import.rs:48-51`): an X becomes an `absent` event unless the DB already records that learner absent for that day. It never removes a DB mark that is missing from the workbook. It is the same code path that shipped in `160002d fix(sf2): recover X marks from the workbook and stop losing attendance`, just triggered automatically.

---

## 9. The destructive-sync guard (D1, D7)

This is the load-bearing safety property. **An X mark in a workbook can only be removed by the user explicitly marking that student present.**

### 9.1 The rule

Before `write_template_marks_for_days` clears anything, the caller must supply a `WorkbookGuard`:

```rust
pub enum SyncPermit {
    /// DB absent count >= workbook X count. Clearing is safe.
    Proven { db_count: usize, workbook_count: usize },
    /// DB is behind. Do NOT clear. Import first, then re-evaluate.
    Stale   { db_count: usize, workbook_count: usize, missing: Vec<(String /*student*/, String /*date*/)> },
    /// Could not measure the workbook (no Excel, file locked, no mappings).
    /// Treat as Stale.
    Unmeasured { reason: String },
}
```

`Unmeasured` is the important default. When in doubt, do not clear.

`sync_and_open_sf2_workbook`, `export_workbook`, and `set_preview_attendance` all evaluate the guard. Behaviour:

| Guard        | Action                                                                                                                                                                                                                                                                                               |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Proven`     | Clear + rewrite from the DB, as today. Set `last_synced_at`.                                                                                                                                                                                                                                         |
| `Stale`      | Do **not** clear. Call the §8.2 import for the listed `(student, date)` pairs, re-evaluate, then proceed only if it now returns `Proven`. If it still does not, abort with: _"The workbook has N X marks the app has no record of. Nothing was changed. Restore a backup or run workbook recovery."_ |
| `Unmeasured` | Do **not** clear. Open the workbook **read-only** (`Workbooks.Open(..., ReadOnly:=True)`) so the user can see their marks, and toast the reason. Never write.                                                                                                                                        |

### 9.2 Differential clear

`clear_attendance_grid` (`progress.rs:246`) currently blanks the entire attendance range. Replace it with a cell-level clear driven by the actual diff:

```
to_clear = { cells the DB says should be blank }
         − { cells the DB says should hold X }
```

Cells that currently hold `X` and the DB says should be blank are **the only** cells a normal sync may blank, and only when the guard returned `Proven`. Everything else is left alone. This makes an over-broad clear structurally impossible rather than merely guarded.

### 9.3 Degenerate-analysis rejection

`update_template_with_mappings` and its v21 successor must reject an empty analysis instead of committing it:

```rust
if dates.is_empty() {
    return Err(AppError::InvalidInput(
        "The SF2 workbook produced no calendar dates. The existing mappings were left untouched."
    ));
}
```

Same for `workbook_learners` returning an empty roster. This closes step 2–3 of the §4 chain at the source.

### 9.4 Update-time integrity gate

`install_staged_inner` (`commands/updates.rs:345`) already writes a pre-install `.db` backup. Extend it:

1. Snapshot the workbooks into `backups/<ts>/workbooks/` (D12) before the installer runs. **Fail the install if this fails** — same policy as the `.db` snapshot.
2. On first launch of the new version, record a `db_fingerprint` = `(count(events), count(events WHERE event_type='absent'), count(sf2_month_date_mappings))` in the settings table.
3. At the _next_ update, compare the new fingerprint against the stored one. A drop in the absent count with no corresponding user action is reported loudly on the Updates panel: _"Attendance records decreased between versions (A → B). Restore from <pre-install backup>."_
4. The stored pre-migration snapshot in `migrations.rs:38` is extended from 3 to 5 files, since the destructive migrations are in v11 and v17.

### 9.5 Fix the JSON round-trip (D17)

- `collect_export_data` must include `event_type` and `absent` events (verify `ExportData`/`AttendanceEvent` serialisation in `domain/models.rs`).
- `import_all` (`commands/data_transfer.rs:141`) must write `event.event_type` instead of the literal `"in"`, and must not synthesise a session key that changes semantics.
- Add a regression test: export a database containing an `absent` event → import into a fresh database → assert the `absent` row survives with the same `timestamp`, `class_id`, and `session_key`.

### 9.6 Fix v11 (contributing factor 7)

`migrate_to_v11.sql:3`'s `DELETE FROM events WHERE event_type <> 'in'` is correct for its time (absence was not yet a record) but is now a landmine for anyone crossing v10→v11 with data written by a newer build. Replace it with a guarded delete that only removes rows whose `event_type` is not a value the current schema knows about, and add a row-count assertion around every table rebuild in `migrate_db` so a future migration that silently drops rows fails loudly at install time.

---

## 10. Backup layout (D11, D12, D13)

### 10.1 Folder per backup

```
backups/2026-09-27-0830/
├── attendance.db
├── manifest.json
└── workbooks/
    ├── SF2-SEPTEMBER-2026.xls
    └── …                              (all 12, present or not)
```

`manifest.json`:

```json
{
	"schemaVersion": 22,
	"createdAt": "2026-09-27T08:30:00+08:00",
	"kind": "scheduled",
	"counts": { "students": 40, "events": 812, "absent": 63, "sf2MonthTemplates": 3 },
	"workbooks": [
		{ "path": "workbooks/SF2-SEPTEMBER-2026.xls", "bytes": 214016, "xCount": 12 },
		{ "path": "workbooks/SF2-OCTOBER-2026.xls", "bytes": 209920, "xCount": 0 }
	]
}
```

### 10.2 Changes to the backup module

| Change                                                                                                      | File                   |
| ----------------------------------------------------------------------------------------------------------- | ---------------------- |
| `list_backups` lists **directories**, not `*.db` files                                                      | `backup_ops.rs:57`     |
| `create_backup_at` writes the folder, then `manifest.json`                                                  | `backup_ops.rs:208`    |
| `preview_backup` accepts a folder, resolves `attendance.db` inside it, and reads `xCount` from the manifest | `backup_ops.rs:146`    |
| `restore_backup` restores `attendance.db` **and** the workbooks, taking a pre-restore snapshot of both      | `restore_service.rs:9` |
| `copy_to_sync_folder` mirrors the whole folder, not one file                                                | `backup_ops.rs:275`    |
| `google_drive` uploads the folder as a zip, keeping the single-object model Drive imposes                   | `google_drive.rs`      |
| `Retention` counts folders                                                                                  | `backup_ops.rs:197`    |

`choose_restore_backup` and `restore-backup-dialog.svelte` gain a workbooks column so the user can see what they are about to restore.

**Critical:** restoring a backup whose `manifest.json` lists a workbook with `xCount > 0` while the restored `.db` holds fewer absences must surface a warning before the restore, and the §9.1 guard must still apply after it.

### 10.3 "Back up workbooks now" (D13)

A button in Settings → Data Management, next to _Back up now_. Writes `backups/<ts>-workbooks/` with only the `workbooks/` subtree plus a manifest, kind: `manual-workbooks`. Does not duplicate the `.db`.

---

## 11. Splitting the existing workbook (D14)

One-time, idempotent, on first launch after the upgrade. Guarded by `settings.sf2_split_completed_at`.

```
For the legacy file at sf2_templates.source_path:
  1. Copy it to sf2-workbooks/_legacy/<original-name>.xls     (never modify the original in place)
  2. For each of the 12 months of the school year:
       a. Write a fresh copy of BUNDLED_TEMPLATE_BYTES to SF2-{MONTH}-{YEAR}.xls
       b. Open it with COM:
            - delete every sheet except the one for this month
            - rename the kept sheet to "{MONTH} {YEAR}"
            - write this month's day numbers into row 6, columns F..AL
            - write the header metadata (school id/name, school year, grade, section,
              adviser, school head, report month) from the legacy template row
            - copy the roster names into the learner rows and re-derive
              sf2_student_mappings for this month
            - copy the legacy sheet's X cells for this month verbatim,
              one cell at a time, reading the source with a bulk range read
       c. Insert the sf2_month_templates + mappings rows
       d. Verify: X cells copied == X cells in the source sheet;
          learner rows == legacy learner count. If not, log and abort the split
            for this month (leave the legacy file authoritative for it).
  3. Recompute first_school_day per month from settings.school_start_date (D16),
     clamped to a Mon-Fri school day; write it to the workbook and the mapping row.
  4. Set settings.sf2_split_completed_at and settings.last_report_month.
  5. Take a D13 workbook backup of the legacy file before step 1.
```

Failure handling:

- The legacy file is **never deleted**. It stays in `_legacy/` and remains importable.
- The app continues to work against the legacy template row for any month whose split failed.
- The split is reported in the UI: _"Split 12 month files. 11 verified, 1 needs attention (JUNE 2027). The original workbook is kept in sf2-workbooks_legacy."_
- Steps 2b and 2c are idempotent: re-running the split overwrites the target file and the target month's rows.

### 11.1 Deriving first_school_day (D16)

New Settings field **"Classes started on"** (`school_start_date`, `YYYY-MM-DD`).

```
first_school_day(MONTH, YEAR) =
    the first Mon-Fri day on or after max(school_start_date, first day of MONTH)
    clamped into MONTH
```

- If `school_start_date` is not set, fall back to the legacy `first_school_day` for the current month, and prompt once: _"Enter the date classes started so each month's SF2 can be dated automatically."_
- Overridable per month in the Reports sidebar, next to the _Switch month_ button. An override is stored in `sf2_month_templates.first_school_day` and re-derivation never touches an overridden value.

---

## 12. UI changes

### 12.1 Settings → SF2 Workbook

Removed (D18):

| Removed                                                | Files                                                                                                                                                                                                                          |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| _Create From Template_ button + `Sf2TemplateDialog`    | `sf2-section.svelte:24-52`, `sf2-template-dialog.svelte`, `openSf2TemplateDialog`, `onCreateSf2FromTemplate`, `applySf2Draft`, `populateSf2Draft` in `sf2-state.svelte.ts`                                                     |
| _Import SF2_ button + the whole import/validation flow | `sf2-section.svelte:53-77`, `Sf2ImportValidationDialog`, `onImportSf2`, `runSf2Import`, `finishSf2Import`, `proceedWithSf2MismatchImport`, `cancelSf2ValidationImport`, `downloadSf2ValidationReport` in `sf2-state.svelte.ts` |
| _Sync Roster_ button                                   | `ReportSidebar.svelte:84-97`, `onSyncRoster` in `report-page-state.svelte.ts:248`                                                                                                                                              |
| Settings → Classes add/edit/delete                     | `classes-section.svelte`, `class-dialog.svelte`, `class-state.svelte.ts` → replaced by a single read-only record                                                                                                               |

Also removed: `import_sf2_workbook`, `validate_sf2_workbook_import`, `create_sf2_workbook_from_template`, `sync_sf2_roster` commands, and their Rust service functions (`validation_service.rs`, `roster_sync.rs`, `template_create.rs`) — **after** confirming no remaining caller. §12.4 covers the case where callers remain.

Replaced by, in Settings → SF2 Workbook:

- **"Classes started on"** date field (D16).
- **"Month workbooks"** read-only list: 12 rows, each showing the month, whether the file exists, the recorded X count, and the last sync time.
- **"Back up workbooks now"** (D13).
- **"Re-run the workbook split"** — re-invokes §11, for the case where a month failed to split.

### 12.2 Reports sidebar

- Month picker dialog kept (D10), now instant.
- _Switch month_ button kept; _Switch class_ `<select>` added.
- The amber "No X marks in the app for this month" recovery panel (`ReportSidebar.svelte:241-264`) is **removed** — its job is now done automatically at startup. It is replaced by a passive status line: _"Last checked <time> · workbook has 12 X · app has 12"_, which is the guard's own comparison surfaced as information.
- _Review Export_ now runs §9.1 before copying.

### 12.3 Reports page

- `ReportMonthSwitchOverlay` deleted.
- `ReportLoadingStates`'s blocking full-page state replaced by a skeleton for the grid only; the sidebar stays interactive.
- Preview cache extended to `${classId}:${schoolYear}:${month}`.

### 12.4 Deletion policy

The project convention forbids orphan code. Any command or service listed in §12.1 that still has a live caller after the UI change is **kept and re-wired**, not deleted — and the spec is amended to say which. The removal list is a target, not a licence to break a path.

---

## 13. Edge cases

| #   | Case                                                                              | Behaviour                                                                                                                                                                                                                      |
| --- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| E1  | Today's month file does not exist (e.g. app opened in June before June is set up) | Fall back to `settings.last_report_month` (D5). Toast: _"Showing <MONTH>. Create <TODAY'S MONTH> to switch."_ Offer a one-click "Create <MONTH>" that clones the previous month's file, keeps the roster, and clears the grid. |
| E2  | App opened in April/May, summer, no classes                                       | Same as E1. Never auto-create a file for a month with no school days.                                                                                                                                                          |
| E3  | `school_start_date` not set                                                       | Prompt once; derive from the legacy `first_school_day` until it is set (§11.1).                                                                                                                                                |
| E4  | A month file is missing from disk but its `sf2_month_templates` row exists        | Every write path returns _"SF2-<MONTH>.xls is missing. Restore it from a backup."_ The guard is `Unmeasured`, so **nothing is cleared**.                                                                                       |
| E5  | The user has the `.xls` open in Excel when the app writes                         | Existing COM error path. The guard treats it as `Unmeasured` and opens read-only instead of writing.                                                                                                                           |
| E6  | Two months' files are open in Excel simultaneously                                | Unchanged. The guard makes the second one read-only rather than corrupting the first.                                                                                                                                          |
| E7  | A student is renamed between two month files                                      | `sf2_learner_id` match (v20) keeps the identity; `row_index` is not reused across files.                                                                                                                                       |
| E8  | A student transfers out mid-year                                                  | Present in one month file, absent in the next. Each file has its own roster, so this is a non-event — the old model forced a single shared row index.                                                                          |
| E9  | Roster grows past the bundled template's row slots                                | `template_roster_slots()` / `bundled_template_total_rows` already handle slot growth; re-verified per-month-file since roster capacity now varies by month.                                                                    |
| E10 | `attendance_day_status` (closed days) is per class, not per month                 | Add a `report_month` dimension so a closed day in September does not close the same date number in October. Currently `PRIMARY KEY(class_id, date)` — a latent bug this work exposes.                                          |
| E11 | Backup restored whose manifest disagrees with its `.db`                           | Warn before restoring; §9.1 still guards afterwards.                                                                                                                                                                           |
| E12 | Disk full while splitting                                                         | The split is resumable — `sf2_split_completed_at` is set only after all 12 months verify, and each month's target file is rewritten from scratch on retry.                                                                     |
| E13 | Google Drive backup folder upload fails                                           | Non-fatal, same as today (`backup_ops.rs:261`). The local backup is still written.                                                                                                                                             |
| E14 | The user runs the new build on a machine with the old build's data                | v19–v22 migrate the single-template row into the current month's row; the split runs on first launch; the legacy file is preserved.                                                                                            |
| E15 | Two months both end up selected (impossible by construction)                      | `UNIQUE(active_class_id, school_year, report_month)` prevents it.                                                                                                                                                              |

---

## 14. Acceptance criteria

**Durability (D1) — the point of the whole spec**

1. With an `attendance.db` whose `events` table is emptied for a month, and that month's `.xls` still holding 12 X marks: clicking _Open SF2_ writes **nothing** to the workbook, the 12 X marks are still there afterwards, and the app has imported all 12 into the database.
2. Same setup, then _Review Export_: the export is refused with a message naming the count mismatch. No output file is written.
3. With a locked/missing workbook file, _Open SF2_ opens it read-only. Verified by attempting a write and asserting the file's mtime is unchanged.
4. Killing Excel mid-`refresh_template_calendar` leaves `sf2_month_date_mappings` byte-identical to before (§9.3).
5. `export JSON` → `wipe all` → `import JSON` on a database with 63 absences restores 63 absences.
6. After `bun run tauri build` and installing over an existing install, a database with 63 absences still reports 63 after first launch.
7. Killing the app mid-split and relaunching completes the split with no duplicated or lost X marks.

**Performance (D9)**

8. Month switch p95 < 300 ms over 20 consecutive switches, release build, 40 students.
9. Zero `sf2-progress` events during a month switch.
10. Class switch keeps the sidebar interactive and never shows a blocking full-page loader.

**Model (D2, D3, D4, D5)**

11. Exactly 12 files exist in `sf2-workbooks/`, each `.xls`, each with exactly one worksheet named `"{MONTH} {year}"`.
12. On launch, the app selects the file matching today's calendar month.
13. No file in `sf2-workbooks/` contains a sheet named `__SF2_HIDDEN_*`.

**Recovery (D6, D8, D13)**

14. No "Import X from Workbook" button exists anywhere in the UI; `grep -ri "import x from" src/` returns nothing.
15. With a workbook holding more X marks than the DB, launching the app imports them and toasts the count.
16. _Back up workbooks now_ produces a folder containing all 12 files and a manifest with per-file `xCount`.

**Hygiene**

17. `bun run check` and `bun run lint` clean; `cargo clippy` and `cargo fmt --check` clean.
18. `grep -rn "configure_sf2_calendar" src-tauri/` shows only the date-header writer; the hide/rename loop is gone.
19. `grep -rn "attendance_changed_since" src-tauri/` returns nothing.
20. `grep -rn 'params!\[.*"in"' src-tauri/src/commands/data_transfer.rs` returns nothing.

---

## 15. Implementation phases

Each phase is independently shippable and leaves the app working.

| Phase                        | Work                                                                                                                                                                       | Schema |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ |
| **0. Stop the bleeding**     | §9.3 reject degenerate analyses · §9.5 fix JSON `event_type` · §9.6 guard v11 · row-count assertions in `migrate_db` · `Unmeasured` default in the existing clear path     | none   |
| **1. Back up the workbooks** | §10 folder-per-backup · §10.3 button · §9.4 update-time snapshot + fingerprint                                                                                             | none   |
| **2. Per-month file model**  | v19–v22 · `sf2_month_templates` repos · month-file creation from the bundled template · single-sheet files · `school_start_date` + per-month `first_school_day` derivation | 19→22  |
| **3. Split + backfill**      | §11 split job · legacy preservation · resumable                                                                                                                            | —      |
| **4. Instant switching**     | §7 · delete the modal, the month-switch command, `configure_sf2_calendar`'s hide/rename loop, `attendance_changed_since` · class selector + skeleton                       | —      |
| **5. The guard**             | §9.1 `SyncPermit` · §9.2 differential clear · `Unmeasured` read-only open                                                                                                  | —      |
| **6. Self-heal**             | §8 startup heal · remove the import button and its dead UI                                                                                                                 | —      |
| **7. Cleanup**               | §12.1 removals, re-wiring anything still referenced                                                                                                                        | —      |

Phase 0 alone is worth shipping immediately: it is a few hundred lines, it is entirely additive, and it stops the next update from destroying marks while the rest is built.

---

## 16. Files to touch

**Rust — new**

```
src-tauri/src/sf2/month/mod.rs
src-tauri/src/sf2/month/template_repo.rs        sf2_month_templates CRUD
src-tauri/src/sf2/month/date_repo.rs            sf2_month_date_mappings CRUD
src-tauri/src/sf2/month/student_repo.rs         sf2_month_student_mappings CRUD
src-tauri/src/sf2/month/workbook_builder.rs     create/split a single-month .xls
src-tauri/src/sf2/month/split.rs                the §11 one-time split job
src-tauri/src/sf2/month/first_school_day.rs     D16 derivation
src-tauri/src/sf2/guard/mod.rs                  SyncPermit
src-tauri/src/sf2/guard/evaluate.rs             workbook X count vs DB absent count
src-tauri/src/sf2/heal.rs                       §8.2 startup self-heal
src-tauri/src/sf2/sql/migrate_to_v19.sql … migrate_to_v22.sql
```

**Rust — modified**

```
src-tauri/src/infrastructure/database/migrations.rs       18 → 22, row-count assertions
src-tauri/src/sf2/repository.rs                            per-month repos replace the template repo
src-tauri/src/sf2/excel/excel_com/calendar.rs             hide/rename loop deleted
src-tauri/src/sf2/excel/excel_com/learners.rs              read the learner ID column
src-tauri/src/sf2/progress.rs                             differential clear, SyncPermit
src-tauri/src/sf2/attendance/attendance_marks.rs          differential clear
src-tauri/src/sf2/attendance/attendance_service.rs        guard before write; Unmeasured read-only open
src-tauri/src/sf2/attendance/attendance_import.rs         reused by the guard + self-heal
src-tauri/src/sf2/calendar/mod.rs                          attendance_changed_since deleted
src-tauri/src/sf2/template/template_ops.rs                 set_report_month* deleted
src-tauri/src/sf2/workbook_files.rs                       per-month paths, _legacy/
src-tauri/src/backup/backup_ops.rs                        folder-per-backup
src-tauri/src/backup/restore_service.rs                    restore .db + workbooks
src-tauri/src/backup/google_drive.rs                       folder → zip upload
src-tauri/src/commands/commands.rs (mod)                   new commands
src-tauri/src/commands/sf2.rs                              add/remove
src-tauri/src/commands/backup.rs                           new command
src-tauri/src/commands/updates.rs                          workbook snapshot + fingerprint
src-tauri/src/commands/data_transfer.rs                    event_type fix
src-tauri/src/lib.rs                                       register commands, run heal on setup
```

**Frontend — new**

```
src/lib/db-rust/sf2-months.ts          getSf2MonthPreview, listSf2MonthFiles, backupWorkbooksNow
src/routes/reports/report-month-bar.svelte
src/routes/settings/month-files-section.svelte
src/routes/settings/workbook-split-section.svelte
```

**Frontend — modified**

```
src/lib/types.ts                                    Sf2MonthTemplate, Sf2MonthPreview, Sf2HealOutcome
src/lib/features/settings/sf2-workbook.ts           first_school_day derivation helpers
src/routes/reports/report-page-state.svelte.ts      instant switch, class selector, guard wiring
src/routes/reports/+page.svelte                     drop the overlay + import button
src/routes/reports/ReportSidebar.svelte             drop Import X + Sync Roster; add month/class bar
src/routes/settings/sf2-section.svelte              gutted → month files list
src/routes/settings/sf2-state.svelte.ts             import/template flow deleted
src/routes/settings/backup-section.svelte           Back up workbooks now
src/routes/settings/classes-section.svelte          read-only single class
```

**Frontend — deleted**

```
src/routes/reports/ReportMonthSwitchOverlay.svelte
src/routes/reports/report-sf2-progress.svelte       (only used by the open flow, which is now guarded)
src/routes/settings/sf2-template-dialog.svelte
src/routes/settings/sf2-import-validation-dialog.svelte
src/routes/settings/class-dialog.svelte
src/routes/settings/sf2-progress.svelte.ts
```

---

## 17. Open questions

1. **The "Settings → SF2 UI" answer was self-contradictory.** _Remove Create From Template / Import SF2_, _Remove Sync Roster_, and _Remove the class dropdown_ were all selected alongside _Keep them all_. This spec implements the three removals (D18) and keeps everything else. **Confirm or correct.**
2. **`school_start_date` has no existing source.** It must be typed once. Should it instead be derived from the first `first_school_day` already recorded, or should the app offer a "typical Philippine school year" default (e.g. 2nd week of August) that the user can correct?
3. **Per-month `first_school_day` override UI** — inline in the Reports sidebar, or in Settings → Month workbooks?
4. **Should the split also read the DepEd learner ID column** (§6.3) and backfill it into `students`, or leave `sf2_learner_id` NULL for existing installs and populate it only for months created after the upgrade?
5. **Google Drive packaging** — one zip per backup, or keep one `.db` object and add a second `workbooks.zip` object? The first is one object to manage; the second keeps `.db` restores cheap.
6. **`attendance_day_status` per-month scoping (E10)** — in scope for this work, or a follow-up? It is a real latent bug but orthogonal to the X-mark loss.
7. **Is the report-month dropdown still needed at all** given D5? It is kept for reviewing/printing a past month, but the user could plausibly want it removed. Confirm.
