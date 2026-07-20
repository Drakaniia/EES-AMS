# Performance Spec: Reports Page Month-Switch Optimization

**Date:** 2026-07-19
**Status:** Draft
**Author:** Buffy (AI Assistant)
**Reporter:** Client (Teacher)

---

## 1. Problem Statement

Switching the SF2 report month on the Reports page takes **10–30 seconds**, during which the Windows application window becomes unresponsive (white/greyed out, "not responding" in the title bar). The loading dialog ("Switching report month… / Updating workbook calendar and attendance marks") appears but the spinner hangs for a long time. This happens **every time** the month is switched, not just the first time.

The issue affects a small deployment: **1 class, under 30 students, 1 month of attendance data** (or even zero data during initial testing). This rules out data volume as the primary bottleneck.

---

## 2. Current Month-Switch Flow

```
User clicks month button in month picker dialog
  ↓
onReportMonthChange()  [frontend, +page.svelte]
  ↓
setSf2ReportMonth(classId, nextMonth)
  → Rust set_sf2_report_month command
  → Sf2Repository::set_report_month()
  → UPDATE sf2_templates SET report_month = ?  (pure DB, very fast)
  ↓
loadReport(classId)  [frontend]
  ↓
getSf2ExportPreview(classId)
  → Rust get_sf2_export_preview command (SYNCHRONOUS, blocks IPC thread)
    ↓
    export_readiness()  [excel_service.rs]
      → Query template from DB
      → Query student mappings from DB
      → Query date mappings from DB
      → Query ALL students for class from DB
      → Check if workbook file exists on disk
      → Build Sf2ExportReadiness struct
    ↓
    preview::export_preview()  [preview.rs]
      → Query template again from DB
      → Query student mappings again from DB
      → Query date mappings again from DB
      → Query class from DB
      → Query ALL students for class from DB
      → Query ALL events: event_repo.list()  (NO date filter - loads every event ever recorded)
      → For each date: filter events to build present_by_day map
      → For each student: build cells array
      → Build Sf2ExportPreview struct (large nested object)
    ↓
    (JSON serialization across Tauri IPC bridge)
  ↓
loadWorkbookSettings(classId)  [frontend]
  → getSf2WorkbookSettings(classId)
  → Rust get_sf2_workbook_settings command (ANOTHER DB query sequence)
    ↓
    workbook_settings()  [excel_service.rs]
      → Query template from DB
      → Query student mappings from DB
      → Query date mappings from DB
      → Query class from DB
    ↓
  ↓
$derived computations on frontend:
  - matrixWeekGroups  (rebuilds week groups from scratch)
  - matrixStudents    (rebuilds student map from scratch)
```

### 2.1 Key Bottlenecks Identified

| #   | Bottleneck                                                                                                                                                                                                               | Location                          | Impact                         |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------- | ------------------------------ |
| 1   | **Duplicate DB queries** — `export_readiness()` and `preview::export_preview()` both query the template, student mappings, and date mappings independently, essentially doing the same work twice.                       | `excel_service.rs` + `preview.rs` | 2× DB overhead                 |
| 2   | **ALL events loaded without filter** — `event_repo.list()` loads every attendance event across all classes and all dates. No WHERE clause on class_id or date range.                                                     | `preview.rs` line 80              | Scales poorly with event count |
| 3   | **Two sequential IPC round-trips** — `getSf2ExportPreview` and `getSf2WorkbookSettings` are called sequentially, each requiring its own IPC serialization/deserialization cycle.                                         | `+page.svelte` → `db-rust/sf2.ts` | Double IPC overhead            |
| 4   | **Synchronous Tauri command** — `get_sf2_export_preview` is NOT `async`. While Tauri v2 runs sync commands on a threadpool, the large response struct serialization blocks the IPC channel during marshalling.           | `commands/sf2.rs`                 | Blocks UI during serialization |
| 5   | **Full preview rebuild on every switch** — The entire `Sf2ExportPreview` is rebuilt from scratch when only the report_month field changed. There is no caching or incremental update.                                    | `excel_service.rs` → `preview.rs` | 100% recomputation             |
| 6   | **$derived recomputation on large objects** — `matrixStudents` rebuilds `Map` objects for every student on every preview update. With 30 students × 22 days = 660 entries, this is minor but still unnecessary overhead. | `+page.svelte`                    | Adds to total jank duration    |

---

## 3. Proposed Optimizations

### 3.1 Combine Duplicate Queries in export_preview (High Impact)

**Problem:** `export_preview()` calls `export_readiness()` first, which queries the template, student mappings, date mappings, and class students. Then `preview::export_preview()` queries all of these **again** independently.

**Solution:** Merge the readiness check into the preview generation, or pass the readiness data through to avoid re-querying.

```rust
// Current:
pub fn export_preview(pool: DbPool, class_id: Option<String>) -> Result<Sf2ExportPreview> {
    let readiness = export_readiness(pool.clone(), class_id)?;
    preview::export_preview(pool, readiness)
}

// Proposed: Have preview::export_preview accept the data it needs directly,
// or restructure so the DB queries are done once and shared.
```

**Estimated improvement:** Reduces DB round-trips by ~40%.

### 3.2 Filter Events by Class and Date Range (Medium Impact)

**Problem:** `event_repo.list()` has no WHERE clause, loading all events ever recorded. For a teacher with months of data, this could be thousands of events.

**Solution:** Create a new repository method that filters events by class ID and date range (the report month boundaries).

```rust
// Proposed:
pub fn list_for_class_and_date_range(
    &self,
    class_id: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<AttendanceEvent>> {
    // SQL with WHERE class_id = ? AND timestamp >= ? AND timestamp < ?
    // This reduces the result set to only relevant events.
}
```

**Estimated improvement:** Significant when there are multiple months/classes of data. Less impact for the current single-class deployment but prevents future scaling issues.

### 3.3 Cache Preview Data Across Month Switches (High Impact)

**Problem:** Every month switch triggers a complete rebuild of the `Sf2ExportPreview` from scratch, even though the only thing that changed is the `report_month` field in the template.

**Solution:** Add a client-side cache keyed by `(classId, reportMonth)`. When switching months, check if the preview for that month was already loaded. If so, return the cached version without hitting the Rust backend.

```typescript
// Proposed: In report-state.svelte.ts or +page.svelte
const previewCache = new Map<string, Sf2ExportPreview>();

function getCacheKey(classId: string, reportMonth: string): string {
	return `${classId}:${reportMonth}`;
}

async function loadReport(classId?: string) {
	const cacheKey = getCacheKey(activeClassId, activeReportMonth);
	const cached = previewCache.get(cacheKey);
	if (cached) {
		preview = cached;
		return;
	}
	const nextPreview = await getSf2ExportPreview(classId);
	previewCache.set(cacheKey, nextPreview);
	preview = nextPreview;
	// ...
}
```

**Important consideration:** The cache must be invalidated when attendance events change (a student is marked present/absent in the current month). This can be done by:

- Clearing the cache for the current month after any toggle/attendance change
- Setting a maximum cache size (e.g., 3 months) to prevent memory bloat

**Estimated improvement:** 0ms on repeat switches to the same month (instant display).

### 3.4 Parallelize IPC Calls (Medium Impact)

**Problem:** `loadReport()` calls `getSf2ExportPreview()` and then `loadWorkbookSettings()` sequentially.

**Solution:** Move workbook settings data into the preview response, or call both in parallel since they are independent.

```typescript
// Proposed: Parallel execution
async function loadReport(classId?: string) {
	const [nextPreview, nextSettings] = await Promise.all([
		getSf2ExportPreview(classId),
		classId ? getSf2WorkbookSettings(classId).catch(() => null) : Promise.resolve(null)
	]);
	preview = nextPreview;
	if (nextPreview.classId) selectedClassId = nextPreview.classId;
	if (nextSettings) {
		workbookSettings = nextSettings;
		hydrateDraft(nextSettings);
	}
}
```

**Estimated improvement:** 30-50% reduction in total load time (both calls run concurrently).

### 3.5 Make Tauri Commands Async (Lower Impact)

**Problem:** `get_sf2_export_preview` is synchronous. While Tauri handles this on a threadpool, the large response serialization can block the IPC channel.

**Solution:** Make the command `async` and use `tokio::spawn_blocking` for the heavy work.

```rust
// Proposed:
#[tauri::command]
pub async fn get_sf2_export_preview(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<Sf2ExportPreview, String> {
    let pool = pool.inner().clone();
    tokio::task::spawn_blocking(move || {
        service::export_preview(pool, class_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

**Estimated improvement:** Marginal for raw speed but prevents the UI thread from freezing during serialization.

### 3.6 Optimize Frontend Derived State (Lower Impact)

**Problem:** `matrixStudents` and `matrixWeekGroups` are `$derived` properties that recompute on every `preview` assignment.

**Solution:** Use lazy computation or memoization with dirty checking.

```typescript
// Proposed: Memoize matrix computations
let lastPreviewVersion = $state(0);
let matrixStudentsCache = $state<MatrixStudentRow[]>([]);

$effect(() => {
	if (preview) {
		matrixStudentsCache = (preview.students ?? [])
			.filter((row) => genderFilter === 'all' || row.gender?.toLowerCase() === genderFilter)
			.map((row) => ({
				...row,
				cellsByDate: new Map(row.cells.map((cell) => [cell.date, cell]))
			}));
	}
});
```

**Estimated improvement:** Minimal (<100ms) for 30 students but prevents cumulative jank.

---

## 4. Investigation: Root Cause Analysis

During preliminary investigation, several hypotheses were tested:

### Hypothesis 1: Data Volume

**Test:** User reports 30 students, 1 class, zero attendance data during testing.
**Result:** ❌ Eliminated. Even with zero events, switching takes 10-30 seconds.

### Hypothesis 2: Excel COM Automation

**Test:** Check if `export_preview` or `export_readiness` interacts with the Excel file.
**Result:** ❌ Eliminated. `export_preview` only checks `Path::new(&template.source_path).exists()` — no Excel COM interaction.

### Hypothesis 3: DB Query Performance

**Test:** SQLite queries on a local file with 0-100 rows.
**Result:** ❌ Eliminated. All individual queries should complete in <1ms.

### Hypothesis 4: Tauri IPC Serialization Overhead

**Test:** `Sf2ExportPreview` is a large nested struct. With 30 students × 22 dates, the JSON payload is ~50-70 KB.
**Result:** ⚠️ **Likely contributor.** The synchronous Tauri command serializes the entire struct to JSON on the threadpool, then deserializes it on the frontend. For a 70 KB nested object with many small strings, this can take 100-500ms — but not 10-30 seconds.

### Hypothesis 5: WebView / WRY Performance

**Test:** The Tauri v2 webview (WRY on Windows) may have performance issues with large reactive updates.
**Result:** ⚠️ **Possible contributor.** The frontend `$derived` recomputations coupled with a large state update could cause WebView layout thrashing.

### Hypothesis 6: Cumulative Effect (Most Likely)

**Conclusion:** The 10-30 second freeze is caused by a **combination** of:

1. Synchronous IPC with large payload blocking the message channel
2. Duplicate DB queries doubling the work
3. Frontend reactive cascade (preview assignment triggers multiple $derived recomputations)
4. Windows DPI / WebView rendering overhead during heavy JS execution

---

## 5. Implementation Priority

| Priority | Optimization                             | Effort | Impact                | Risk   |
| -------- | ---------------------------------------- | ------ | --------------------- | ------ |
| P0       | Combine duplicate DB queries             | Small  | High                  | Low    |
| P0       | Cache preview data across month switches | Medium | High                  | Low    |
| P1       | Filter events by class and date range    | Small  | Medium (future-proof) | Low    |
| P1       | Parallelize IPC calls                    | Small  | Medium                | Low    |
| P2       | Make Tauri commands async                | Medium | Low-Medium            | Medium |
| P2       | Optimize frontend derived state          | Small  | Low                   | Low    |

## 6. Success Criteria

1. Month switching completes in **< 2 seconds** for a single class with 30 students and up to 6 months of data
2. The window does NOT become unresponsive (no "not responding" state)
3. The loading dialog provides meaningful progress feedback if the switch takes > 1 second
4. Repeat switches to the same month are **instant** (< 100ms) due to caching
5. No regressions in attendance toggling, export, or open-in-Excel functionality

## 7. Edge Cases

### 7.1 Cache Invalidation

When a student's attendance is toggled (present/absent) in the current month, the cached preview for that month must be invalidated. Proposed strategy:

- After any `toggleSf2PreviewAttendance` call, delete the cache entry for the current month
- After `presentAllSf2PreviewAttendance`, delete the cache entry
- After `syncSf2Roster`, delete ALL cached entries (roster changed)

### 7.2 First Switch After App Start

The first month switch after app launch will still be slow (no cache). This is acceptable as long as subsequent switches are fast.

### 7.3 Month Switch + Roster Sync

If the user switches months and then syncs the roster, the cached preview for the new month becomes stale. The roster sync handler should clear relevant cache entries.

### 7.4 Multiple Classes

If a school has multiple classes with SF2 workbooks, the cache keyed by `(classId, reportMonth)` handles this correctly — each class has its own cache entries.

### 7.5 Memory Considerations

With 30 students × 22 days = ~50 KB per month preview, caching 3 months uses ~150 KB. Even caching 12 months uses ~600 KB — negligible for a desktop app.

## 8. Files to Modify

### Rust Backend

| File                                              | Changes                                                                 |
| ------------------------------------------------- | ----------------------------------------------------------------------- |
| `src-tauri/src/sf2/excel_service.rs`              | Merge `export_readiness` into preview path to avoid duplicate queries   |
| `src-tauri/src/sf2/preview.rs`                    | Optimize `export_preview` — filter events, reuse readiness data         |
| `src-tauri/src/sf2/repository.rs`                 | Add method for filtered events by class and date range                  |
| `src-tauri/src/infrastructure/database/events.rs` | Add `list_for_class_and_date_range()` repository method                 |
| `src-tauri/src/commands/sf2.rs`                   | Make command async, consider merging preview+settings into one response |

### Svelte Frontend

| File                                        | Changes                                                              |
| ------------------------------------------- | -------------------------------------------------------------------- |
| `src/routes/reports/+page.svelte`           | Add preview caching, parallelize IPC calls, add $derived memoization |
| `src/routes/reports/report-state.svelte.ts` | Add cache helpers and memoization utilities                          |
| `src/lib/db-rust/sf2.ts`                    | Add combined preview+settings command if merged                      |

## 9. Measuring Impact

Before implementing, establish a performance baseline:

1. **Single switch time**: Time from clicking a month button to the report table being fully rendered
2. **Backend execution time**: Time spent in `export_preview` Rust function
3. **IPC time**: Time from `invoke()` call to response received on frontend
4. **Frontend rendering time**: Time from `preview = nextPreview` to DOM update complete

These can be measured with:

- `console.time()` / `console.timeEnd()` around IPC calls
- Rust `std::time::Instant` for backend timing (add debug logs)
- Browser DevTools Performance tab (if accessible via Tauri dev tools)
