# SF2 Template Structure Reference

> **File:** `src-tauri/resources/sf2/TEMPLATE_AUTOMATED_SF2.xls`  
> **Generated:** Python inspection via xlrd  
> **Purpose:** Reference document to compare the template against SF2 workbook copies when debugging

---

## Overview

The bundled template has **6 sheets** (5 monthly sheets + 1 "COMPLETE DAYS" sheet). Each sheet has **83 rows × 51 columns** (rows 1-83, cols A-AY).

| Sheet | Rows | Cols |
|-------|------|------|
| JUNE 2025 | 83 | 51 |
| JULY 2025 | 84 | 51 |
| AUGUST 2025 | 83 | 51 |
| SEPT. 2025 | 83 | 51 |
| OCTOBER 2025 | 83 | 51 |
| COMPLETE DAYS | 83 | 51 |

---

## Critical: Column-to-Weekday Mapping (Row 7)

**IMPORTANT:** The template does NOT use 1 column per weekday. Some weekdays span **2 merged columns**. Empty columns between weekday labels are the **sub-cells of merged pairs** and must be skipped.

### Merged-Pair Columns (the sub-cell has no label)

| Column | Cell | Part of | Skips |
|--------|------|---------|-------|
| 7 | G | F7:G7 merged (Monday) | G has no label |
| 13 | M | L7:M7 merged (Monday) | M has no label |
| 19 | S | R7:S7 merged (Monday) | S has no label |
| 23 | W | V7:W7 merged (Thursday) | W has no label |
| 25 | Y | X7:Y7 merged (Friday) | Y has no label |
| 27 | AA | Z7:AA7 merged (Monday) | AA has no label |
| 34 | AH | AG7:AH7 merged (Tuesday) | AH has no label |
| 38 | AL | AK7:AL7 merged (Friday) | AL has no label |

### Complete Column-to-Weekday Table (F7..AL7)

```
 Column | Cell | Label | Mapped Weekday | Week Index
--------|------|-------|----------------|------------
   6    |  F7  |  'M'  | Monday    (0)  | week 0  ← merged with G7
   8    |  H7  |  'T'  | Tuesday   (1)  | week 0
   9    |  I7  |  'W'  | Wednesday (2)  | week 0
  10    |  J7  | 'TH'  | Thursday  (3)  | week 0
  11    |  K7  |  'F'  | Friday    (4)  | week 0
  12    |  L7  |  'M'  | Monday    (0)  | week 1  ← merged with M7
  14    |  N7  |  'T'  | Tuesday   (1)  | week 1
  15    |  O7  |  'W'  | Wednesday (2)  | week 1
  16    |  P7  | 'TH'  | Thursday  (3)  | week 1
  17    |  Q7  |  'F'  | Friday    (4)  | week 1
  18    |  R7  |  'M'  | Monday    (0)  | week 2  ← merged with S7
  20    |  T7  |  'T'  | Tuesday   (1)  | week 2
  21    |  U7  |  'W'  | Wednesday (2)  | week 2
  22    |  V7  | 'TH'  | Thursday  (3)  | week 2  ← merged with W7
  24    |  X7  |  'F'  | Friday    (4)  | week 2  ← merged with Y7
  26    |  Z7  |  'M'  | Monday    (0)  | week 3  ← merged with AA7
  28    | AB7  |  'T'  | Tuesday   (1)  | week 3
  29    | AC7  |  'W'  | Wednesday (2)  | week 3
  30    | AD7  | 'TH'  | Thursday  (3)  | week 3
  31    | AE7  |  'F'  | Friday    (4)  | week 3
  32    | AF7  |  'M'  | Monday    (0)  | week 4
  33    | AG7  |  'T'  | Tuesday   (1)  | week 4  ← merged with AH7
  35    | AI7  |  'W'  | Wednesday (2)  | week 4
  36    | AJ7  | 'TH'  | Thursday  (3)  | week 4
  37    | AK7  |  'F'  | Friday    (4)  | week 4  ← merged with AL7
```

**Total: 25 day slots across 33 columns (F-AL)** — 5 weeks × 5 weekdays.

---

## Merged Cell Pattern in Data Rows (Rows 8+)

Every data row has the SAME merged cell pairs as the header. This pattern applies to ALL learner rows (8 through ~50+):

```
F8:G8    (cols 6-7)  → Monday (2-column)
L8:M8    (cols 12-13) → Monday (2-column)
R8:S8    (cols 18-19) → Monday (2-column)
V8:W8    (cols 22-23) → Thursday (2-column)
X8:Y8    (cols 24-25) → Friday (2-column)
Z8:AA8   (cols 26-27) → Monday (2-column)
AG8:AH8  (cols 33-34) → Tuesday (2-column)
AK8:AL8  (cols 37-38) → Friday (2-column)
```

**Consequence:** When writing attendance marks, writing to the PRIMARY column (the one with the label) is sufficient — the merged area displays the value in both columns. Writing to the sub-column will be redirected by `merged_target` and may overwrite the primary column's value.

---

## Date Examples (Row 6)

### JUNE 2025
```
F6=2   (Mon Jun 2, week 0)
H6=3   (Tue Jun 3)
I6=4   (Wed Jun 4)
J6=5   (Thu Jun 5)
K6=6   (Fri Jun 6)
L6=9   (Mon Jun 9, week 1)
N6=10  (Tue Jun 10)
...
AF6=30 (Mon Jun 30, week 4)
```

### JULY 2025
```
H6=1   (Tue Jul 1, week 0)  ← Note: F6 is empty! Month starts on Tuesday
I6=2   (Wed Jul 2)
J6=3   (Thu Jul 3)
K6=4   (Fri Jul 4)
L6=7   (Mon Jul 7, week 1)
N6=8   (Tue Jul 8)
...
AJ6=31 (Thu Jul 31, week 4)
```

### AUGUST 2025
```
K6=1   (Fri Aug 1, week 0)  ← Note: Starts at column K (Friday)
L6=4   (Mon Aug 4, week 1)
...
AK6=29 (Fri Aug 29, week 4)
```

### SEPT. 2025
```
F6=1   (Mon Sep 1, week 0)
...
AG6=30 (Tue Sep 30, week 4)
```

### OCTOBER 2025
```
I6=1   (Wed Oct 1, week 0)
...
AK6=31 (Fri Oct 31, week 4)
```

---

## Column Widths

The 2-column merged Monday cells have a WIDER total width than single-day columns:
```
F: width=182     G: width=512     (Monday = 694 total)
H: width=841     (single day)
I: width=877     (single day)
J: width=877     (single day)
K: width=841     (single day)
L: width=658     M: width=73      (Monday = 731 total)
R: width=182     S: width=512     (Monday = 694 total)
V: width=182     W: width=585     (Thursday = 767 total)
AG: width=694    AH: width=73     (Tuesday = 767 total)
```

The narrow sub-cell columns (width 73) are effectively hidden placeholders.

---

## Key: How to Validate a Workbook Copy

When comparing a workbook copy against this reference:

1. **Row 7 labels** should match EXACTLY — same labels in same columns
2. **Merged cells** in rows 6-7 and data rows should be IDENTICAL
3. **Date numbers** in row 6 vary by month but must appear only in PRIMARY columns (never in sub-cells)
4. **Column widths** should be preserved
