# Design: Remaining Audit Refactoring

**Date:** 2026-07-25
**Status:** Approved

## Overview

Complete the remaining items from the ees-ams audit report. The audit identified 22 issues total; 12 are already resolved. This covers the remaining 10 items.

## Remaining Items

### 1. Split `workbook.rs` (925 lines → target: ≤400)

Current structure under `excel_com/`:
- `workbook.rs` (925) — high-level API, still too large
- `workbook_utils.rs` — already extracted
- `com_session.rs` — already extracted
- `learners.rs` — already extracted
- `worksheet.rs` — already extracted
- `calendar.rs` — already extracted

**Approach:** Extract remaining COM infrastructure and I/O operations into:
- `workbook_com.rs` — ComObject, ComVariant, ExcelSession, ComApartment
- `workbook_io.rs` — read/write marks, formulas, batch operations

### 2-6. Near-threshold Rust files (436-503 lines)

Lean extraction: pull clearly separable helper functions, data structs, and pure utility code into adjacent files. Files:

- `attendance_service.rs` (503) — extract remaining non-core sync helpers
- `excel_service.rs` (455) — extract init/helper code
- `roster_parser.rs` (484) — extract mapping/data logic
- `data_transfer.rs` (436) — extract thin helpers
- `roster_sync.rs` (470) — extract learner/helper code

### 7. `reports/+page.svelte` (534 lines)

Already componentized (14 files). Extract remaining inline markup/logic into existing or new child components.

### 8. `app.css` (619 lines)

- Keep global tokens/variables in `app.css`
- Extract component-specific styles into per-component CSS files
- Extract page-specific styles into route-level styles

## Success Criteria

- All files under 400 lines (or demonstrably at clean seam boundaries)
- Build passes: `cargo build` and `npm run build`
- All existing tests pass
- No functional changes — pure refactoring
