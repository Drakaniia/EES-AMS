use crate::domain::error::Result;
use crate::infrastructure::database::{DbPool, EventRepository, StudentRepository};
use crate::sf2::attendance::{absent_student_ids, present_student_ids};
use crate::sf2::logic::{attendance_marks_for_day, Sf2CellMark, Sf2StudentMapping};
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord, Sf2TemplateRecord};

use std::collections::{HashMap, HashSet};

/// Generate attendance Excel marks for a set of days.
///
/// An "X" is written only for students with an explicit absent record; every
/// other student stays blank (present by default). Days with no records at all
/// (open days) are skipped.
pub(super) fn export_marks(
    pool: DbPool,
    class_id: &str,
    closed_days: &[String],
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> Result<Vec<Sf2CellMark>> {
    let date_by_day: HashMap<&str, &Sf2DateMappingRecord> = date_mappings
        .iter()
        .map(|mapping| (mapping.date.as_str(), mapping))
        .collect();
    let student_repo = StudentRepository::new(pool.clone());
    let event_repo = EventRepository::new(pool);
    let students = student_repo.list_by_class(Some(class_id))?;
    let events = event_repo.list()?;

    let mut marks = Vec::new();
    for day in closed_days {
        let Some(date_mapping) = date_by_day.get(day.as_str()) else {
            continue;
        };

        let day_students: Vec<Sf2StudentMapping> = student_mappings
            .iter()
            .map(|student| Sf2StudentMapping {
                student_id: student.student_id.clone(),
                sheet_name: date_mapping.sheet_name.clone(),
                row_index: student.row_index,
            })
            .collect();
        let absent_ids = absent_student_ids(&events, &students, class_id, day);

        // Skip days with no records at all (open days) - nothing to mark.
        if absent_ids.is_empty()
            && present_student_ids(&events, &students, class_id, day).is_empty()
        {
            continue;
        }

        marks.extend(attendance_marks_for_day(
            &day_students,
            &absent_ids,
            &date_mapping.column_letter,
        ));
    }

    Ok(marks)
}

/// Clear all attendance marks for the given date mappings and student mappings.
/// Generates empty cell marks for every combination of sheet × weekday column × attendance row,
/// ensuring stale marks from the bundled template or previous months are erased before
/// writing new ones.
pub(super) fn clear_attendance_marks_for_records(
    template: &Sf2TemplateRecord,
    date_mappings: &[Sf2DateMappingRecord],
    student_mappings: &[Sf2StudentMappingRecord],
) -> Vec<Sf2CellMark> {
    let row_indices = if crate::sf2::calendar_service::template_owns_roster(template) {
        let row_slots = crate::sf2::calendar_service::template_roster_slots();
        attendance_grid_rows(
            &row_slots,
            student_mappings.iter().map(|mapping| mapping.row_index),
        )
    } else {
        mapped_attendance_rows(student_mappings.iter().map(|mapping| mapping.row_index))
    };

    // Get the unique set of visible sheet names from the date mappings.
    let sheet_names: Vec<&str> = date_mappings
        .iter()
        .map(|m| m.sheet_name.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if sheet_names.is_empty() {
        return Vec::new();
    }

    // Generate column letters for ALL standard DepEd SF2 weekday columns (F through AL).
    // This ensures stale marks are cleared even from weekday columns that have no
    // valid date in the report month (e.g. Monday/Tuesday in the first week when
    // the month starts mid-week).
    let all_column_letters: Vec<String> = (6..=38).map(column_number_to_letter).collect();

    let mut marks =
        Vec::with_capacity(sheet_names.len() * all_column_letters.len() * row_indices.len());
    for sheet_name in &sheet_names {
        for col_letter in &all_column_letters {
            for row_index in &row_indices {
                marks.push(Sf2CellMark {
                    sheet_name: sheet_name.to_string(),
                    cell_address: format!("{col_letter}{row_index}"),
                    value: String::new(),
                });
            }
        }
    }
    marks
}

/// Convert a 1-based column index to an Excel column letter (e.g., 1 -> A, 26 -> Z, 27 -> AA).
fn column_number_to_letter(mut column: i32) -> String {
    let mut letter = String::new();
    while column > 0 {
        let modulo = (column - 1) % 26;
        letter.insert(0, (b'A' + modulo as u8) as char);
        column = (column - modulo) / 26;
    }
    letter
}

/// Generate Excel formulas and static values for the SF2 summary section (rows 53–71).
///
/// Returns `(formula_marks, static_marks)`:
/// - `formula_marks` — Excel formulas for rows 59 (Registered Learners), 61 (Percentage of
///   Enrolment), 63 (Average Daily Attendance), and 65 (Percentage of Attendance).
///   These are written with `set_sf2_formula`.
/// - `static_marks` — Static numeric values for row 53 (Enrolment).
///   These are written with `set_sf2_mark_force`.
///
/// Marks are generated per unique sheet name found in `date_mappings`, so all visible
/// monthly sheets get the same summary formulas.
pub(super) fn summary_formula_marks(
    male_count: usize,
    female_count: usize,
    total_students: usize,
    male_total_row: u32,
    female_total_row: u32,
    combined_total_row: u32,
    date_mappings: &[Sf2DateMappingRecord],
) -> (Vec<Sf2CellMark>, Vec<Sf2CellMark>) {
    // Extract unique sheet names from date_mappings
    let sheet_names: Vec<&str> = date_mappings
        .iter()
        .map(|m| m.sheet_name.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if sheet_names.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut formula_marks = Vec::new();
    let mut static_marks = Vec::new();
    let columns = ["AR", "AS", "AT"];

    for sheet_name in &sheet_names {
        let sn = sheet_name.to_string();

        // ── Static marks (row 53: Enrolment) ─────────────────────────
        static_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AR53".to_string(),
            value: male_count.to_string(),
        });
        static_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AS53".to_string(),
            value: female_count.to_string(),
        });
        static_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AT53".to_string(),
            value: total_students.to_string(),
        });

        // ── Row 59: Registered Learners ─────────────────────────────
        // Formula: =col53+col55-col67-col69+col71
        for col in &columns {
            formula_marks.push(Sf2CellMark {
                sheet_name: sn.clone(),
                cell_address: format!("{col}59"),
                value: format!("={col}53+{col}55-{col}67-{col}69+{col}71"),
            });
        }

        // ── Row 61: Percentage of Enrolment ──────────────────────────
        // Formula: =IF(col53>0, col59/col53*100, 0)
        for col in &columns {
            formula_marks.push(Sf2CellMark {
                sheet_name: sn.clone(),
                cell_address: format!("{col}61"),
                value: format!("=IF({col}53>0,{col}59/{col}53*100,0)"),
            });
        }

        // ── Row 63: Average Daily Attendance ─────────────────────────
        // Male ADA references male_total_row, Female ADA female_total_row, Total ADA combined_total_row
        // Formula: =IFERROR(AVERAGE(F{total_row}:AL{total_row}),0)
        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AR63".to_string(),
            value: format!("=IFERROR(AVERAGE(F{male_total_row}:AL{male_total_row}),0)"),
        });
        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AS63".to_string(),
            value: format!("=IFERROR(AVERAGE(F{female_total_row}:AL{female_total_row}),0)"),
        });
        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AT63".to_string(),
            value: format!("=IFERROR(AVERAGE(F{combined_total_row}:AL{combined_total_row}),0)"),
        });

        // ── Row 65: Percentage of Attendance ─────────────────────────
        // Formula: =IF(col59>0, col63/col59*100, 0)
        for col in &columns {
            formula_marks.push(Sf2CellMark {
                sheet_name: sn.clone(),
                cell_address: format!("{col}65"),
                value: format!("=IF({col}59>0,{col}63/{col}59*100,0)"),
            });
        }
    }

    (formula_marks, static_marks)
}

/// Generate Excel formulas for the MALE TOTAL, FEMALE TOTAL, and Combined TOTAL rows.
///
/// Writes formulas that dynamically calculate present student count per day:
///   MALE TOTAL:     ={male_count}-COUNTIF({col}{first_male_row}:{col}{last_male_row},"X")
///   FEMALE TOTAL:   ={female_count}-COUNTIF({col}{first_female_row}:{col}{last_female_row},"X")
///   Combined TOTAL: ={col}{male_total}+{col}{female_total}
///
/// The formulas use "X" marks (absent) to compute PRESENT count per day.
/// Empty template rows (no student assigned) never have X marks, so they don't affect the count.
pub(super) fn total_formula_marks(
    male_count: usize,
    female_count: usize,
    male_total_row: u32,
    female_total_row: u32,
    combined_total_row: u32,
    date_mappings: &[Sf2DateMappingRecord],
) -> Vec<Sf2CellMark> {
    // Male range: first male slot (8) to last male slot before TOTAL
    let first_male_row = 8u32;
    let last_male_row = male_total_row.saturating_sub(1);
    // Female range: first female slot (after MALE TOTAL) to last female slot before FEMALE TOTAL
    let first_female_row = male_total_row + 1;
    let last_female_row = female_total_row.saturating_sub(1);

    let mut formula_marks = Vec::new();
    for date_mapping in date_mappings {
        if date_mapping.date.trim().is_empty() {
            continue;
        }
        let col = &date_mapping.column_letter;

        // MALE TOTAL: ={male_count}-COUNTIF({col}{first}:{col}{last},"X")
        formula_marks.push(Sf2CellMark {
            sheet_name: date_mapping.sheet_name.clone(),
            cell_address: format!("{col}{male_total_row}"),
            value: format!(
                "={}-COUNTIF({}{}:{}{},\"X\")",
                male_count, col, first_male_row, col, last_male_row,
            ),
        });

        // FEMALE TOTAL: ={female_count}-COUNTIF({col}{first}:{col}{last},"X")
        formula_marks.push(Sf2CellMark {
            sheet_name: date_mapping.sheet_name.clone(),
            cell_address: format!("{col}{female_total_row}"),
            value: format!(
                "={}-COUNTIF({}{}:{}{},\"X\")",
                female_count, col, first_female_row, col, last_female_row,
            ),
        });

        // Combined TOTAL: ={col}{male_total}+{col}{female_total}
        formula_marks.push(Sf2CellMark {
            sheet_name: date_mapping.sheet_name.clone(),
            cell_address: format!("{col}{combined_total_row}"),
            value: format!("={}{}+{}{}", col, male_total_row, col, female_total_row),
        });
    }
    formula_marks
}

/// Generate Excel formulas for the ABSENT (AM) and PRESENT (AO) columns so the
/// per-learner absent/present totals recalculate automatically in Excel.
///
/// The bundled template ships with AM/AO formulas on most learner rows, but some
/// rows are missing them (e.g. the SEPT. sheet lacks `=COUNTIF(F{r}:AL{r},"X")`
/// on several rows, and the subtotal rows lack the `AO` formula). This generates
/// complete formulas for every mapped student row plus the MALE/FEMALE/Combined
/// subtotal cells.
///
/// Returns `(formula_marks, static_marks)`:
/// - `formula_marks` — AM/AO formulas written with `set_sf2_formula`.
/// - `static_marks` — a numeric `AW5` ("TOTAL NO. OF DAYS") value so `$AW$5-AM{r}`
///   computes PRESENT from the actual mapped school-day count. The template's
///   AW5 values are stale/inconsistent (e.g. 22 while 23 days are mapped), which
///   would otherwise make every PRESENT count wrong.
///
/// Formula conventions (mirroring the bundled template):
///   AM{r} = COUNTIF(F{r}:AL{r},"X")            → absent days for learner row r
///   AO{r} = $AW$5-AM{r}                        → present days (total − absent)
///   AM{male_total}   = SUM(AM8:AN{male_total-1})
///   AO{male_total}   = $AW$5*{male_count}-AM{male_total}
///   AM{female_total} = SUM(AM{male_total+1}:AN{female_total-1})
///   AO{female_total} = $AW$5*{female_count}-AM{female_total}
///   AM{combined}     = AM{male_total}+AM{female_total}
///   AO{combined}     = AO{male_total}+AO{female_total}
// The row counts and totals are intentionally flat: they mirror the SF2 layout
// columns and are consumed directly by the formula builders below. Grouping
// them into a struct would obscure that 1:1 mapping at every call site.
#[allow(clippy::too_many_arguments)]
pub(super) fn learner_absent_present_formula_marks(
    student_mappings: &[Sf2StudentMappingRecord],
    male_count: usize,
    female_count: usize,
    day_count: usize,
    male_total_row: u32,
    female_total_row: u32,
    combined_total_row: u32,
    sheet_names: &[&str],
) -> (Vec<Sf2CellMark>, Vec<Sf2CellMark>) {
    if sheet_names.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut formula_marks = Vec::new();
    let mut static_marks = Vec::new();

    for sheet_name in sheet_names {
        let sn = sheet_name.to_string();

        // Correct "TOTAL NO. OF DAYS" so `$AW$5` matches the mapped day count.
        static_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: "AW5".to_string(),
            value: day_count.to_string(),
        });

        // Per-learner ABSENT / PRESENT formulas.
        for mapping in student_mappings {
            let row = mapping.row_index;
            if row == 0 {
                continue;
            }
            formula_marks.push(Sf2CellMark {
                sheet_name: sn.clone(),
                cell_address: format!("AM{row}"),
                value: format!("=COUNTIF(F{row}:AL{row},\"X\")"),
            });
            formula_marks.push(Sf2CellMark {
                sheet_name: sn.clone(),
                cell_address: format!("AO{row}"),
                value: format!("=$AW$5-AM{row}"),
            });
        }

        // MALE TOTAL / FEMALE TOTAL / Combined TOTAL cells in the same columns.
        let male_last = male_total_row.saturating_sub(1);
        let female_first = male_total_row + 1;
        let female_last = female_total_row.saturating_sub(1);

        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: format!("AM{male_total_row}"),
            value: format!("=SUM(AM8:AN{male_last})"),
        });
        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: format!("AO{male_total_row}"),
            value: format!("=$AW$5*{male_count}-AM{male_total_row}"),
        });

        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: format!("AM{female_total_row}"),
            value: format!("=SUM(AM{female_first}:AN{female_last})"),
        });
        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: format!("AO{female_total_row}"),
            value: format!("=$AW$5*{female_count}-AM{female_total_row}"),
        });

        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: format!("AM{combined_total_row}"),
            value: format!("=AM{male_total_row}+AM{female_total_row}"),
        });
        formula_marks.push(Sf2CellMark {
            sheet_name: sn.clone(),
            cell_address: format!("AO{combined_total_row}"),
            value: format!("=AO{male_total_row}+AO{female_total_row}"),
        });
    }

    (formula_marks, static_marks)
}

/// Generate empty cell marks for all TOTAL PER DAY formula cells across ALL
/// weekday columns (6–38). This clears stale template values (default `0` or
/// leftover formulas) from columns that have no corresponding date in the
/// report month — e.g. columns for Monday/Tuesday in the first week when
/// the month starts mid-week.
///
/// Must be called with `write_marks_force` *before* `write_formulas` so that
/// columns WITHOUT a valid date end up clean/empty rather than showing a
/// stale value inherited from the bundled template.
pub(super) fn clear_total_cell_marks(
    male_total_row: u32,
    female_total_row: u32,
    combined_total_row: u32,
    date_mappings: &[Sf2DateMappingRecord],
) -> Vec<Sf2CellMark> {
    let sheet_names: Vec<&str> = date_mappings
        .iter()
        .map(|m| m.sheet_name.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if sheet_names.is_empty() {
        return Vec::new();
    }

    let mut marks = Vec::with_capacity(sheet_names.len() * 33 * 3);
    for sheet_name in sheet_names {
        for col in 6..=38 {
            let col_letter = column_number_to_letter(col);
            marks.push(Sf2CellMark {
                sheet_name: sheet_name.to_string(),
                cell_address: format!("{col_letter}{male_total_row}"),
                value: String::new(),
            });
            marks.push(Sf2CellMark {
                sheet_name: sheet_name.to_string(),
                cell_address: format!("{col_letter}{female_total_row}"),
                value: String::new(),
            });
            marks.push(Sf2CellMark {
                sheet_name: sheet_name.to_string(),
                cell_address: format!("{col_letter}{combined_total_row}"),
                value: String::new(),
            });
        }
    }
    marks
}

pub(super) fn attendance_grid_rows<I>(
    row_slots: &[super::calendar_service::TemplateRosterSlot],
    extra_rows: I,
) -> Vec<u32>
where
    I: IntoIterator<Item = u32>,
{
    let mut rows = row_slots
        .iter()
        .map(|slot| slot.row_index)
        .collect::<Vec<_>>();
    rows.extend(extra_rows);
    rows.sort_unstable();
    rows.dedup();
    rows
}

pub(super) fn mapped_attendance_rows<I>(rows: I) -> Vec<u32>
where
    I: IntoIterator<Item = u32>,
{
    let mut rows = rows
        .into_iter()
        .filter(|row_index| *row_index > 0)
        .collect::<Vec<_>>();
    rows.sort_unstable();
    rows.dedup();
    rows
}
