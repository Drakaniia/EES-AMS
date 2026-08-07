use crate::domain::error::{AppError, Result};
use crate::infrastructure::database::DbPool;
use crate::sf2::attendance_marks;
use crate::sf2::excel;
use crate::sf2::logic::Sf2CellMark;
use crate::sf2::models::{Sf2DateMappingRecord, Sf2StudentMappingRecord, Sf2TemplateRecord};
use crate::sf2::repository::Sf2Repository;
use crate::sf2::sf2_metadata::sf2_date_mappings_for_report_month;
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::Emitter;

/// Callback used to report fine-grained progress while Excel writes marks.
/// The no-op variant is used by callers without a UI (e.g. roster sync).
type WriteProgress = Box<dyn Fn(u32, u32, &str) + Send>;

/// Emit a progress event to the frontend during SF2 workbook operations.
pub(super) fn emit_sf2_progress<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    task: &str,
    current: u32,
    total: u32,
    message: &str,
) {
    let _ = app.emit(
        "sf2-progress",
        serde_json::json!({
            "task": task,
            "current": current,
            "total": total,
            "message": message,
        }),
    );
}

fn noop_progress() -> WriteProgress {
    Box::new(|_: u32, _: u32, _: &str| {})
}

/// Emit a fine-grained write-phase progress event.
///
/// The Excel write phase spans 61%–69% on a 100-point scale: the outer
/// "Writing marks to workbook…" step is 6/10 (60%) and the next outer step
/// is 7/10 (70%), so the frontend bar crawls instead of pausing at 60%.
fn emit_write_step(progress: &WriteProgress, units_done: usize, total_units: usize, message: &str) {
    let total = total_units.max(1) as u64;
    let offset = (units_done as u64).saturating_mul(8).div_ceil(total).min(8);
    progress(61 + offset as u32, 100, message);
}

pub(super) fn write_template_marks_for_days(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    days: &[String],
) -> Result<usize> {
    write_template_marks_for_days_impl(pool, template, days, noop_progress())
}

/// Same as [`write_template_marks_for_days`] but emits fine-grained
/// `sf2-progress` events (task "open") while Excel writes marks, so the
/// opening flow can show the bar advancing through the write phase.
pub(super) fn write_template_marks_for_days_with_progress<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    pool: DbPool,
    template: &Sf2TemplateRecord,
    days: &[String],
) -> Result<usize> {
    let app = app.clone();
    let progress: WriteProgress = Box::new(move |current, total, message| {
        emit_sf2_progress(&app, "open", current, total, message);
    });
    write_template_marks_for_days_impl(pool, template, days, progress)
}

fn write_template_marks_for_days_impl(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    days: &[String],
    progress: WriteProgress,
) -> Result<usize> {
    let workbook_path = PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let sf2_repo = Sf2Repository::new(pool.clone());
    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let date_mappings = sf2_date_mappings_for_report_month(
        template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    if date_mappings.is_empty() {
        return Ok(0);
    }

    write_template_marks_for_mappings_impl(
        pool,
        template,
        days,
        &student_mappings,
        &date_mappings,
        progress,
    )
}

pub(super) fn write_template_marks_for_mappings(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    days: &[String],
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> Result<usize> {
    write_template_marks_for_mappings_impl(
        pool,
        template,
        days,
        student_mappings,
        date_mappings,
        noop_progress(),
    )
}

fn write_template_marks_for_mappings_impl(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    days: &[String],
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
    progress: WriteProgress,
) -> Result<usize> {
    let workbook_path = PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }

    let mapped_dates: HashSet<&str> = date_mappings
        .iter()
        .map(|mapping| mapping.date.as_str())
        .collect();
    let export_days = days
        .iter()
        .filter(|day| mapped_dates.contains(day.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    let sf2_repo = Sf2Repository::new(pool.clone());
    let all_date_mappings = sf2_repo.date_mappings_for_template(&template.id)?;
    let clear_date_mappings: Vec<Sf2DateMappingRecord> = if all_date_mappings.is_empty() {
        date_mappings.to_vec()
    } else {
        sf2_date_mappings_for_report_month(template, &all_date_mappings)
    };

    let owns_roster = crate::sf2::roster_parser::template_owns_roster(template);

    // ── Attendance marks (sparse "X" for absent students) ───────────
    let attendance_marks = if export_days.is_empty() || student_mappings.is_empty() {
        Vec::new()
    } else {
        attendance_marks::export_marks(
            pool,
            &template.active_class_id,
            &export_days,
            student_mappings,
            date_mappings,
        )?
    };
    let attendance_mark_count = attendance_marks.len();

    // ── Clear marks ─────────────────────────────────────────────────
    // Bundled templates: use Range.ClearContents() per sheet (2 COM
    // calls per sheet) instead of per-cell writes (~1,400+ COM calls).
    // Imported templates: fall back to per-cell clear marks.
    let clear_marks: Vec<Sf2CellMark> = if !owns_roster {
        attendance_marks::clear_attendance_marks_for_records(
            template,
            &clear_date_mappings,
            student_mappings,
        )
    } else {
        Vec::new()
    };

    // Unique sheet names for bulk clear (bundled templates only).
    let bulk_sheets: Vec<String> = if owns_roster {
        clear_date_mappings
            .iter()
            .map(|m| m.sheet_name.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    } else {
        Vec::new()
    };

    // Row positions for bulk clear (only needed for bundled templates).
    let male_count = student_mappings
        .iter()
        .filter(|m| m.gender_block.as_deref() == Some("MALE"))
        .count();
    let female_count = student_mappings
        .iter()
        .filter(|m| m.gender_block.as_deref() == Some("FEMALE"))
        .count();
    let (male_total_row, female_total_row, _combined_total_row) =
        crate::sf2::roster_parser::bundled_template_total_rows(male_count, female_count);

    // ── ABSENT/PRESENT formula marks ────────────────────────────────
    let formula_marks_opt = compute_absent_present_marks(template, student_mappings, date_mappings);

    // ── Single Excel session ────────────────────────────────────────
    excel::batch_operations(&workbook_path, true, move |session| {
        // Fine-grained write progress: chunk per-cell writes so the frontend
        // bar crawls through 61%–69% instead of pausing at 60% while COM
        // writes thousands of cells. The no-op reporter keeps non-UI callers
        // (e.g. roster sync) unaffected.
        const CHUNK_SIZE: usize = 100;
        let (formula_marks, static_marks) = match &formula_marks_opt {
            Some((formula, static_marks)) => (formula.as_slice(), static_marks.as_slice()),
            None => (&[][..], &[][..]),
        };
        let clear_chunks = clear_marks.len().div_ceil(CHUNK_SIZE);
        let marks_chunks = attendance_marks.len().div_ceil(CHUNK_SIZE);
        let formula_chunks = formula_marks.len().div_ceil(CHUNK_SIZE);
        let static_chunks = static_marks.len().div_ceil(CHUNK_SIZE);
        // INVARIANT: total_units must equal the sum of every phase's unit count
        // below (sheet clears + chunked per-cell writes). Keep it in sync when
        // adding or removing phases so the 61–69% mapping stays accurate.
        let total_units =
            (bulk_sheets.len() + clear_chunks + marks_chunks + formula_chunks + static_chunks)
                .max(1);
        let mut units_done = 0usize;

        // Move the bar as soon as the write phase starts, before the first
        // sheet/chunk completes, so it never feels paused at 60%.
        emit_write_step(&progress, 0, total_units, "Preparing the workbook…");

        // Phase 1: Bulk-clear attendance grid (bundled templates).
        for (index, sheet_name) in bulk_sheets.iter().enumerate() {
            session.clear_attendance_grid(sheet_name, male_total_row, female_total_row)?;
            units_done += 1;
            emit_write_step(
                &progress,
                units_done,
                total_units,
                &format!(
                    "Clearing attendance grid (sheet {}/{})…",
                    index + 1,
                    bulk_sheets.len()
                ),
            );
        }

        // Phase 2: Per-cell clear marks (imported templates).
        if clear_chunks > 0 {
            for (index, chunk) in clear_marks.chunks(CHUNK_SIZE).enumerate() {
                session.write_marks_force(chunk)?;
                units_done += 1;
                emit_write_step(
                    &progress,
                    units_done,
                    total_units,
                    &format!("Clearing previous marks ({}/{})…", index + 1, clear_chunks),
                );
            }
        }

        // Phase 3: Sparse attendance "X" marks.
        if marks_chunks > 0 {
            for (index, chunk) in attendance_marks.chunks(CHUNK_SIZE).enumerate() {
                session.write_marks_force(chunk)?;
                units_done += 1;
                emit_write_step(
                    &progress,
                    units_done,
                    total_units,
                    &format!("Writing attendance marks ({}/{})…", index + 1, marks_chunks),
                );
            }
        }

        // Phase 4: AM/AO formulas (non-fatal on failure — matches
        // the original warn! semantics).
        if formula_chunks > 0 {
            for (index, chunk) in formula_marks.chunks(CHUNK_SIZE).enumerate() {
                let _ = session.write_formulas(chunk);
                units_done += 1;
                emit_write_step(
                    &progress,
                    units_done,
                    total_units,
                    &format!("Updating formulas ({}/{})…", index + 1, formula_chunks),
                );
            }
        }
        if static_chunks > 0 {
            for (index, chunk) in static_marks.chunks(CHUNK_SIZE).enumerate() {
                let _ = session.write_marks_force(chunk);
                units_done += 1;
                emit_write_step(
                    &progress,
                    units_done,
                    total_units,
                    &format!("Writing totals ({}/{})…", index + 1, static_chunks),
                );
            }
        }

        Ok(())
    })?;

    Ok(attendance_mark_count)
}

/// Compute ABSENT/PRESENT (AM/AO) formula marks without opening Excel.
/// Returns `None` when the workbook isn't a bundled template or there's
/// nothing to compute (empty mappings, etc.).
fn compute_absent_present_marks(
    template: &Sf2TemplateRecord,
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> Option<(Vec<Sf2CellMark>, Vec<Sf2CellMark>)> {
    if !crate::sf2::roster_parser::template_owns_roster(template) {
        return None;
    }
    if student_mappings.is_empty() || date_mappings.is_empty() {
        return None;
    }

    let male_count = student_mappings
        .iter()
        .filter(|m| m.gender_block.as_deref() == Some("MALE"))
        .count();
    let female_count = student_mappings
        .iter()
        .filter(|m| m.gender_block.as_deref() == Some("FEMALE"))
        .count();
    let (male_total_row, female_total_row, combined_total_row) =
        crate::sf2::roster_parser::bundled_template_total_rows(male_count, female_count);

    let sheet_names: Vec<&str> = date_mappings
        .iter()
        .map(|m| m.sheet_name.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let (formula_marks, static_marks) = attendance_marks::learner_absent_present_formula_marks(
        student_mappings,
        male_count,
        female_count,
        date_mappings.len(),
        male_total_row,
        female_total_row,
        combined_total_row,
        &sheet_names,
    );
    if formula_marks.is_empty() && static_marks.is_empty() {
        return None;
    }
    Some((formula_marks, static_marks))
}

/// Write ABSENT/PRESENT (AM/AO) formulas for every mapped student plus the
/// MALE/FEMALE/Combined subtotal cells, and correct AW5 ("TOTAL NO. OF DAYS")
/// to the actual mapped day count.
///
/// Only applies to bundled templates (the app fully owns their layout). Imported
/// templates are left untouched so their original formulas are preserved.
pub(super) fn write_learner_absent_present_formulas_for_mappings(
    template: &Sf2TemplateRecord,
    student_mappings: &[Sf2StudentMappingRecord],
    date_mappings: &[Sf2DateMappingRecord],
) -> Result<usize> {
    let Some((formula_marks, static_marks)) =
        compute_absent_present_marks(template, student_mappings, date_mappings)
    else {
        return Ok(0);
    };

    let workbook_path = PathBuf::from(&template.source_path);
    if !workbook_path.exists() {
        return Err(AppError::InvalidInput(
            "The app SF2 working workbook no longer exists. Import the SF2 workbook again"
                .to_string(),
        ));
    }
    // Write formulas and the AW5 static in a single Excel session to keep the
    // repair cheap (it also runs on every attendance toggle / workbook open).
    let marks_total = formula_marks.len() + static_marks.len();
    excel::batch_operations(&workbook_path, true, move |session| {
        if !formula_marks.is_empty() {
            session.write_formulas(&formula_marks)?;
        }
        if !static_marks.is_empty() {
            session.write_marks_force(&static_marks)?;
        }
        Ok(())
    })?;
    Ok(marks_total)
}

/// Load student/date mappings from the DB and write ABSENT/PRESENT formulas.
/// Used to self-heal existing bundled workbooks when they are opened.
pub(super) fn repair_learner_absent_present_formulas(
    pool: DbPool,
    template: &Sf2TemplateRecord,
) -> Result<usize> {
    let sf2_repo = Sf2Repository::new(pool.clone());
    let student_mappings = sf2_repo.student_mappings_for_template(&template.id)?;
    let date_mappings = sf2_date_mappings_for_report_month(
        template,
        &sf2_repo.date_mappings_for_template(&template.id)?,
    );
    write_learner_absent_present_formulas_for_mappings(template, &student_mappings, &date_mappings)
}
