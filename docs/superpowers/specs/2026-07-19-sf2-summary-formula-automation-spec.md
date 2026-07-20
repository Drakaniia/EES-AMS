# SF2 Summary Formula Automation Spec

**Date:** 2026-07-19  
**Status:** Draft, updated with template verification

## Problem

The automated SF2 template (`TEMPLATE_AUTOMATED_SF2.xls`, compiled as `BUNDLED_TEMPLATE_BYTES`) has **static placeholder values** (not formulas) in its summary table section (rows 51–72). After the app creates a working copy from this template and expands the student roster (inserting rows for >21 male or >19 female students), the summary cells remain as the old placeholder demo data — they never update to reflect the actual student count and attendance.

Currently, the app only rewrites per-day column formulas for **MALE TOTAL, FEMALE TOTAL, Combined TOTAL** (rows 29/49/50 + expansion offset). The summary section cells (AR53/AS53/AT53 through AR71/AS71/AT71) are **never written** by the app, so they keep the template's demo values (12M/14F/26T, 100%, etc.).

This fix is needed in **three flows**:

1. **Create working copy from template** (`template_ops.rs::create_workbook_from_template_in_dir`)
2. **Import existing SF2 workbook** (`validation_service.rs::import_workbook_with_analysis`)
3. **Update workbook settings – bundled template branch** (`template_ops.rs::update_workbook_settings`)

---

## Template Layout (Verified via xlrd)

The template has **6 sheets**:
- JUNE 2025 (83 rows × 51 cols)
- JULY 2025 (84 rows × 51 cols)
- AUGUST 2025 (83 rows × 51 cols)
- SEPT. 2025 (83 rows × 51 cols)
- OCTOBER 2025 (83 rows × 51 cols)
- COMPLETE DAYS (83 rows × 51 cols)

All sheets share an **identical layout** for the summary section. The `write_metadata` and `write_formulas` operations already loop over all visible monthly sheets, so the summary formulas will be written to all sheets automatically.

### Columns

| Column letter | 0-index | 1-index | Content        |
|---------------|---------|---------|----------------|
| AM            | 38      | 39      | Field labels   |
| AN            | 39      | 40      | (empty in template) |
| AO            | 40      | 41      | (empty in template) |
| AP            | 41      | 42      | Extended labels (e.g., "No. of Days of Classes:") |
| AQ            | 42      | 43      | (empty in template) |
| **AR**        | **43**  | **44**  | **Male (M)**   |
| **AS**        | **44**  | **45**  | **Female (F)** |
| **AT**        | **45**  | **46**  | **Total**      |

### Verified Rows (xlrd cell dump)

| Row 1-idx | AM (col 38) Label | AR (M) | AS (F) | AT (TOTAL) | Notes |
|-----------|-------------------|--------|--------|-------------|-------|
| 51 | `Month :` / `No. of Days of Classes:` / `Summary` | `Summary` (TEXT) | — | — | Label area. Month value needs location check |
| 52 | (guidelines text in col A) | `M` (TEXT) | `F` (TEXT) | `TOTAL` (TEXT) | Column headers |
| **53** | `* Enrolment as of (1st Friday of the SY)` | **12.0** | **14.0** | **26.0** | **Static demo value — REPLACE with total students** |
| 54 | — | — | — | — | Blank separator |
| **55** | `Late enrolment` | **0.0** | **0.0** | **0.0** | **Manual entry — leave as 0** |
| 56 | (guidelines in col B: `a. Percentage of Enrolment =`) | — | — | — | Instructional text |
| 57 | `(beyond cut-off)` | — | — | — | Sub-label for row 55 |
| 58 | (guidelines in col G: `Enrolment as of 1st Friday...`) | — | — | — | Instructional text |
| **59** | `Registered Learners as of` | **12.0** | **14.0** | **26.0** | **Static demo — REPLACE with formula** |
| 60 | `end of month` | — | — | — | Continuation of row 59 label |
| **61** | `Percentage of Enrolment as of` | **100.0** | **100.0** | **100.0** | **Static demo — REPLACE with formula** |
| 62 | `end of month` | — | — | — | Continuation |
| **63** | `Average Daily Attendance` | **11.545** | **13.272** | **24.818** | **Static demo — REPLACE with formula** |
| 64 | (guidelines text in col A) | — | — | — | Guidelines |
| **65** | `Percentage of Attendance for the month` | **96.212** | **94.805** | **95.454** | **Static demo — REPLACE with formula** |
| **66** | `Number of students absent for 5 consecutive days` | **0.0** | **0.0** | **0.0** | **SKIP — TODO** |
| **67** | `NLS` | **0.0** | **0.0** | **0.0** | **Leave as-is** |
| 68 | (note text in col B) | — | — | — | Note |
| **69** | `Transferred out` | **0.0** | **0.0** | **0.0** | **Manual entry** |
| 70 | (guidelines in col Z) | — | — | — | Guidelines column |
| **71** | `Transferred in` | **0.0** | **0.0** | **0.0** | **Manual entry** |
| 72 | — | — | — | — | Blank |
| 74 | — | `I certify...` in AM | — | — | Signature area |

**Key finding:** xlrd shows all summary data cells with `ctype=2 (NUMBER)`. Since xlrd cannot distinguish static numbers from formula cached results, but no formula records are found internally, these are **all static placeholder values** that need to be replaced with formulas or computed values by the app.

### Cross-sheet consistency

All monthly sheets have the same layout. The demo values differ slightly across sheets (e.g., SEP and OCT show 13M/14F/27T with 108.33% enrolment %), confirming they are just different demo datasets, not formulas.

---

## Formula Details

### Key variables to compute for each flow

After roster expansion (which may insert rows), the row positions shift:

```
male_count          = number of male students in the class
female_count        = number of female students
total_students      = male_count + female_count
extra_male          = max(0, male_count - 21)
extra_female        = max(0, female_count - 19)
male_total_row      = 29 + extra_male
female_total_row    = 49 + extra_male + extra_female
combined_total_row  = 50 + extra_male + extra_female
```

### Per-field formulas

These formulas use Excel's `set_sf2_formula` (not `set_sf2_mark` or `set_sf2_mark_force`) in columns AR(44), AS(45), AT(46).

#### Row 53 — Enrolment as of 1st Friday of SY

**Computed statically in Rust** (not a formula), written with `set_sf2_mark_force`:

```
AR53 = male_count           (integer)
AS53 = female_count         (integer)
AT53 = total_students       (integer)
```

#### Row 55 — Late enrolment during the month

**Manual entry** — leave as-is (currently 0 in template). Do NOT overwrite.

#### Row 59 — Registered Learners as of end of month

Excel formula:
```
AR59: =AR53+AR55-AR67-AR69+AR71
AS59: =AS53+AS55-AS67-AS69+AS71
AT59: =AT53+AT55-AT67-AT69+AT71
```

This computes: enrolment + late enrolment + transferred_in - NLS - transferred_out.
If manual fields are empty (0), it defaults to just the enrolment value.

#### Row 61 — Percentage of Enrolment

Excel formula:
```
AR61: =IF(AR53>0,AR59/AR53*100,0)
AS61: =IF(AS53>0,AS59/AS53*100,0)
AT61: =IF(AT53>0,AT59/AT53*100,0)
```

#### Row 63 — Average Daily Attendance

Excel formulas referencing the TOTAL rows for each date column range (F through AL):

```
AR63: =IFERROR(AVERAGE(F{male_total_row}:AL{male_total_row}),0)
AS63: =IFERROR(AVERAGE(F{female_total_row}:AL{female_total_row}),0)
AT63: =IFERROR(AVERAGE(F{combined_total_row}:AL{combined_total_row}),0)
```

Using the range `F{row}:AL{row}` works because:
- Columns without a day number (row 6 empty) contain 0 or are empty
- `AVERAGE` ignores empty cells and treats 0 as 0
- `IFERROR` handles the edge case of zero school days

#### Row 65 — Percentage of Attendance for the month

Excel formula:
```
AR65: =IF(AR59>0,AR63/AR59*100,0)
AS65: =IF(AS59>0,AS63/AS59*100,0)
AT65: =IF(AT59>0,AT63/AT59*100,0)
```

#### Row 66 — Number of students absent for 5 consecutive days

**SKIP** — mark as `// TODO` in code. Leave the template's static 0 value.

#### Row 67 — NLS, Row 69 — Transferred out, Row 71 — Transferred in

**Manual entry** — leave as-is (currently 0 in template). Do NOT overwrite.

---

## Month and Days of Classes (Row 51)

Row 51 currently has:
- AM51: `Month :` (label)
- AP51: `No. of Days of Classes:` (label)
- AR51: `Summary` (header text)

The Month value and Days count need to be written to specific cells. *These cells need to be identified from the template.*

**Options:**
- Write report month to a cell adjacent to the `Month :` label (e.g., AN51 or AO51)
- Write the count of date_mappings to a cell adjacent to `No. of Days of Classes:` (e.g., AQ51)

*Determine exact target cells from template inspection.*

---

## Implementation Plan

### 1. Add a `summary_formula_marks` function

**File:** `src-tauri/src/sf2/attendance_service.rs`

Add a new public function:

```rust
pub fn summary_formula_marks(
    male_count: usize,
    female_count: usize,
    total_students: usize,
    male_total_row: u32,
    female_total_row: u32,
    combined_total_row: u32,
    date_mappings: &[Sf2DateMappingRecord],
    report_month: &str,
    school_day_count: usize,
) -> (Vec<Sf2CellMark>, Vec<Sf2CellMark>)
// Returns: (formula_marks, static_marks)
```

This function returns two mark lists:
1. `formula_marks` — Excel formulas written via `set_sf2_formula` (rows 59, 61, 63, 65)
2. `static_marks` — Static values written via `set_sf2_mark_force` (row 53, plus month/day cells)

### 2. Write formula cells

Use `session.write_formulas(&formula_marks)` in the batch session context, or `excel::write_formulas()` for the standalone flow.

### 3. Write static values

Use `session.write_marks_force(&static_marks)` for row 53 enrolment values.

### 4. Integration points

#### a. `create_workbook_from_template_in_dir` (template_ops.rs)
Inside the batch closure, after `session.write_formulas(&formula_marks)`:
```rust
let (summary_marks, static_marks) = summary_formula_marks(
    male_count, female_count, students.len(),
    male_total_row_inner, female_total_row_inner, combined_total_row_inner,
    &date_mappings, &metadata.report_month, date_mappings.len(),
);
session.write_formulas(&summary_marks)?;
session.write_marks_force(&static_marks)?;
```

#### b. `import_workbook_with_analysis` (validation_service.rs)
After `excel::write_formulas(...)` on line ~259, add:
```rust
let (summary_marks, static_marks) = summary_formula_marks(...);
excel::write_formulas(&working_copy_path, &summary_marks)?;
excel::write_marks_force(&working_copy_path, &static_marks)?;
```

#### c. `update_workbook_settings` bundled branch (template_ops.rs)
Inside the batch closure, after `session.write_formulas(&formula_marks)`:
```rust
let (summary_marks, static_marks) = summary_formula_marks(...);
session.write_formulas(&summary_marks)?;
session.write_marks_force(&static_marks)?;
```

### 5. Cell addressing

Target cells use string addresses like `"AR53"`, `"AS53"`, `"AT53"`, `"AR59"`, etc.
These are standard Excel range addresses that `set_sf2_formula` and `set_sf2_mark_force` accept.

---

## Edge Cases

1. **Empty class (0 students):** male_count=0, female_count=0. Enrolment = 0. All formulas guarded with `IF(x>0, ...)` to avoid division by zero.
2. **Expanded roster (>21 male or >19 female):** All summary formulas reference the correct shifted total rows via the computed `male_total_row`, `female_total_row`, `combined_total_row` variables.
3. **No school days configured:** `date_mappings` is empty → `school_day_count = 0`. The ADA formula uses `IFERROR(AVERAGE(...), 0)` so it returns 0 instead of `#DIV/0!`.
4. **Single-gender class:** One gender's count is 0. Formulas handle this via IF guards.
5. **Re-import:** Summary formulas are rewritten fresh during the batch session, overwriting any stale values.
6. **Late enrolment / Transferred in/out cells are empty:** The formulas add them anyway (0 + value = value), so registered learners defaults to enrolment value.

---

## Success Criteria

1. After creating a working copy from template, AR53/AS53/AT53 shows correct student counts (not 12/14/26)
2. AR59/AS59/AT59 shows a formula computing registered learners
3. AR61/AS61/AT61 shows percentage of enrolment formula
4. AR63/AS63/AT63 shows average of per-day combined totals
5. AR65/AS65/AT65 shows percentage of attendance formula
6. After roster expansion, all formulas reference the correct shifted row ranges
7. All cells are written to ALL monthly sheets (not just the first one)
8. Division by zero is guarded (no `#DIV/0!` in Excel)
9. The three existing per-day TOTAL formulas continue to work correctly
