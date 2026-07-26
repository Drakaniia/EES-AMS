---
name: debug-sf2-workbook
description: Use when debugging SF2 workbook open/render bugs — day labels misaligned (not starting at M in F7), missing weekday labels, dates in wrong columns, or merged cells interfering with writes. Also use when inspecting the raw cell state of an SF2 .xls copy to compare expected vs actual output.
license: MIT
metadata:
  author: ees-ams
  version: 1.0.0
  category: debugging
  tags:
    - sf2
    - excel
    - xlrd
    - merged-cells
    - calendar
    - deped
    - attendance
---

# Debug SF2 Workbook

A systematic debugging workflow for SF2 workbook open bugs — the written-to-disk copy that the app produces when the user opens an SF2 workbook. Uses Python + xlrd to inspect the raw cell state of the output `.xls` file, then traces the issue back to the Rust code, fixes, re-opens in the app, and re-inspects.

## Red Flags

- Day labels (M/T/W/TH/F) don't start at **F7**
- Some day labels are missing or in wrong columns
- Date numbers (row 6) don't align with their day-of-week label (row 7)
- The MTWThF pattern is broken or shifted
- Writing to a column seems to "overwrite" a previous column's value
- xlrd shows `empty` for a column where a value was expected

## Diagnostic Script

Run this inline Python script to inspect the SF2 workbook copy and identify the problem:

```python
"""
SF2 Workbook Diagnostic Tool

REQUIRED: pip install xlrd   (the .xls files use the old Excel format)
CONFIG:  Set PATH below to the target SF2 workbook copy path.
         The copy lives in Tauri's sf2-workbooks directory, e.g.:
           C:\Users\<USER>\AppData\Roaming\com.ees.ams\sf2-workbooks\SF2-*.xls
"""
import xlrd
import sys

try:
    xlrd
except NameError:
    print("ERROR: xlrd not installed. Run: pip install xlrd")
    sys.exit(1)

# ── CONFIG: set this to the target SF2 workbook path ──────────────────────────
PATH = r'C:\Users\Qwenzy\AppData\Roaming\com.ees.ams\sf2-workbooks\SF2-GRADE-3-MATAPAT-41c15e31.xls'
# ───────────────────────────────────────────────────────────────────────────────

wb = xlrd.open_workbook(PATH, formatting_info=True)  # formatting_info required for merged_cells detection
print(f"Sheets: {wb.sheet_names()}\n")

for sn in wb.sheet_names():
    ws = wb.sheet_by_name(sn)
    print(f"=== Sheet: {sn} (rows={ws.nrows}, cols={ws.ncols}) ===")

    # Row 7 — day-of-week labels (F7..AL7)
    labels = {}
    for col in range(6, 39):
        val = ws.cell_value(6, col-1)
        col_l = chr(64+col) if col <= 26 else 'A'+chr(64+col-26)
        labels[col] = val
        if val:
            print(f"  {col_l}7: {repr(val)}")

    # Row 6 — date numbers
    dates = {}
    for col in range(6, 39):
        val = ws.cell_value(5, col-1)
        col_l = chr(64+col) if col <= 26 else 'A'+chr(64+col-26)
        dates[col] = val
        if val:
            print(f"  {col_l}6: {repr(val)}")

    # Merged cells in row 7
    merges = []
    for rlo, rhi, clo, chi in ws.merged_cells:
        if rlo == 6 or rlo == 5:   # row 7 or row 6 (0-indexed)
            cl1 = chr(65+clo) if clo < 26 else 'A'+chr(65+clo-26)
            cl2 = chr(65+chi-1) if chi-1 < 26 else 'A'+chr(65+chi-1-26)
            merges.append(f"  {cl1}{rlo+1}:{cl2}{rhi}  (cols {clo+1}-{chi})")
    if merges:
        print(f"  Merged cells:")
        for m in merges:
            print(m)

    print()

# ── Analysis: check for merged-cell overwrite pattern ──────────────────────
print("=== Analysis ===")
# Expected labels (hardcoded: 7 weeks × 5 weekdays starting from col 6)
WEEKDAY_LABELS = ["M","T","W","TH","F"] * 7
expected = {}
for i, col in enumerate(range(6, 39)):
    expected[col] = WEEKDAY_LABELS[i]

print("Label comparison (actual vs expected):")
errors = 0
for col in range(6, 39):
    cl = chr(64+col) if col <= 26 else 'A'+chr(64+col-26)
    act = labels.get(col, "")
    exp = expected[col]
    if act != exp:
        print(f"  {cl}7: actual={repr(act or 'empty')}  expected={repr(exp)}  ← MISMATCH")
        errors += 1

if errors == 0:
    print("  ✓ All labels correct!")
else:
    print(f"\n  {errors}/33 labels are wrong.")
    print("\n  Common cause: merged cells cause second-column writes to overwrite first.")
    print("  Check if merged cell columns in row 7 match the pattern:")
    print("  (col 6-7 merged → write to col 7 overwrites col 6)")
```

## Debugging Workflow

The inspect → analyze → fix → verify loop:

```
┌─────────────────────────────────────────────┐
│ 1. Get the SF2 workbook path from Tauri's   │
│    sf2-workbooks directory                  │
└──────────┬──────────────────────────────────┘
           ▼
┌─────────────────────────────────────────────┐
│ 2. Run the Diagnostic Script above           │
│    → Shows labels, dates, merged cells       │
│    → Compares actual vs expected labels      │
└──────────┬──────────────────────────────────┘
           ▼
┌─────────────────────────────────────────────┐
│ 3. Trace to Rust source code                │
│    → calendar.rs: set_sf2_month_dates        │
│    → worksheet.rs: set_sf2_cell / merged_target│
│    → Find where the write logic fails        │
└──────────┬──────────────────────────────────┘
           ▼
┌─────────────────────────────────────────────┐
│ 4. Fix the Rust code                        │
│    → cargo check to verify compilation       │
└──────────┬──────────────────────────────────┘
           ▼
┌─────────────────────────────────────────────┐
│ 5. Re-open the SF2 in the app               │
│    → This re-writes the workbook copy        │
└──────────┬──────────────────────────────────┘
           ▼
┌─────────────────────────────────────────────┐
│ 6. Re-run the Diagnostic Script             │
│    → Verify fix worked (all ✓)              │
│    → If not, go back to step 3              │
└─────────────────────────────────────────────┘
```

## Root Causes & Fixes

### 1. Merged Cells Overwrite

**Symptom:** Labels are shifted — root cell of a merged pair shows the **second** column's label.

**Why:** `set_sf2_cell` calls `merged_target(&cell)` which returns the top-left cell of the merged range. Writing column 7's "T" to F7 (root of merged F7:G7) overwrites column 6's "M".

**Fix:** Unmerge cells before writing. In `set_sf2_month_dates` (calendar.rs):
```rust
// Before writing dates/labels, unmerge rows 6 and 7
unmerge_weekday_header_cells(sheet)?;
```

Where `unmerge_weekday_header_cells` iterates columns 6-38 for rows 6 and 7, checks `MergeCells`, and sets `MergeCells = false` on the MergeArea.

### 2. Template Has Stale Labels

**Symptom:** Some labels are from the old template, not overwritten.

**Why:** The code only overwrites columns 6-38, but some columns outside this range still have old values, or the code hit an error mid-loop and stopped.

**Fix:** Ensure the loop covers all 33 weekday columns (6..=38) and errors are properly handled.

### 3. Date Numbers Don't Match Labels

**Symptom:** A column shows date "8" with label "T" (Tue), but July 8 is Wednesday.

**Why:** Same merged-cell overwrite — the date number in row 6 is also corrupted by merged pairs.

**Fix:** Same as #1 — unmerge row 6 before writing dates.

## Key Source Files

| File | Purpose |
|------|---------|
| `src-tauri/resources/sf2/TEMPLATE_AUTOMATED_SF2.xls` | The bundled original template |
| `src-tauri/src/sf2/excel_com/calendar.rs` | `set_sf2_month_dates` — writes date numbers (row 6), `sf2_weekday_slots` — reads labels from row 7 |
| `src-tauri/src/sf2/excel_com/worksheet.rs` | `set_sf2_cell`, `merged_target` |
| `src-tauri/src/sf2/excel_com/workbook_io.rs` | `write_metadata` — triggers `configure_sf2_calendar` |
| `src-tauri/src/sf2/calendar.rs` | Pure calendar logic (date math, weekday detection) |

## Template Reference

See [`references/TEMPLATE_STRUCTURE_REFERENCE.md`](references/TEMPLATE_STRUCTURE_REFERENCE.md) for a complete dump of the original template structure — column-to-weekday mapping, merged cell pairs, date examples per month, and column widths.

## Common Mistakes

- **Running the script on the template `.xlt` file instead of the output copy.** The debug target is the copy in `sf2-workbooks/`, not the template.
- **Only checking row 7 (labels) without checking row 6 (dates).** Merged cells affect both rows.
- **Forgetting that xlrd uses 0-indexed rows/cols.** Row 7 in Excel = index 6 in xlrd. Column F = index 5.
- **Not checking merged cells.** If you see `empty` cells in a pattern, check `ws.merged_cells`.
- **Fixing without verifying.** Always re-open and re-run the script to confirm the fix.

## When NOT to Use

- Debugging the SF2 template creation (use other tools for that)
- Debugging frontend rendering issues
- Debugging Tauri command errors unrelated to workbook content
