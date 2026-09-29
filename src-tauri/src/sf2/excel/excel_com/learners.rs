use crate::domain::error::Result;
use crate::sf2::excel::excel_com::com_session::ComObject;
use crate::sf2::excel::excel_com::worksheet::cell_text;
use crate::sf2::models::Sf2WorkbookLearner;

/// The `No.` column of the DepEd SF2 form, which is the item number.
const ITEM_NUMBER_COLUMN: i32 = 1;

/// The DepEd learner-ID column of the SF2 form, i.e. column B.
///
/// Verified against the bundled `TEMPLATE_AUTOMATED_SF2.xls` on 2026-09-27: the
/// learner row is `A8:B8` (item number, header `A5:B7` = "No.") merged with
/// `C8:E8` (name, header `C5:E7` = "NAME"). Column 2 is therefore the DepEd
/// ID's slot but the bundled template does not use it - reading it returns the
/// merged item number. [`deped_learner_id_from_cells`] rejects that case, so a
/// template that merges the two columns leaves `sf2_learner_id` NULL instead of
/// inventing an identity out of the item number.
const DEPEP_LEARNER_ID_COLUMN: i32 = 2;

/// Parse learner names and DepEd learner IDs from an SF2 worksheet.
pub fn workbook_learners(sheet: &ComObject) -> Result<Vec<Sf2WorkbookLearner>> {
    use crate::sf2::month::student_repo::deped_learner_id_from_cells;

    let used_range = sheet.get_object("UsedRange")?;
    let rows = used_range.get_object("Rows")?;
    let row_count = rows.get_i32("Count")?;
    let mut gender_block = Some("MALE".to_string());
    let mut learners = Vec::new();

    for row in 1..=row_count {
        let name = cell_text(sheet, row, 3)?.trim().to_string();
        if name.is_empty() {
            continue;
        }

        let upper_name = name.to_uppercase();
        if upper_name.contains("MALE") && upper_name.contains("TOTAL") {
            gender_block = Some("FEMALE".to_string());
            continue;
        }
        if upper_name.contains("FEMALE") && upper_name.contains("TOTAL") {
            gender_block = None;
            continue;
        }

        if crate::sf2::logic::is_learner_name(&name) {
            let item_number = cell_text(sheet, row, ITEM_NUMBER_COLUMN)?;
            let learner_id_cell = cell_text(sheet, row, DEPEP_LEARNER_ID_COLUMN)?;
            learners.push(Sf2WorkbookLearner {
                row_index: row as u32,
                name,
                gender_block: gender_block.clone(),
                sf2_learner_id: deped_learner_id_from_cells(&learner_id_cell, &item_number),
            });
        }
    }

    Ok(learners)
}

/// Find the best monthly sheet by quality.
pub fn best_sf2_monthly_sheet(sheets: &[ComObject]) -> Result<Option<ComObject>> {
    let mut best_sheet: Option<(ComObject, Sf2SheetQuality)> = None;
    for sheet in sheets {
        let quality = sf2_sheet_quality(sheet)?;
        if best_sheet
            .as_ref()
            .is_none_or(|(_, best_quality)| quality > *best_quality)
        {
            best_sheet = Some((sheet.clone(), quality));
        }
    }

    Ok(best_sheet.map(|(sheet, _)| sheet))
}

/// Assess the quality of an SF2 worksheet based on learner and day data.
pub fn sf2_sheet_quality(sheet: &ComObject) -> Result<Sf2SheetQuality> {
    let learners = workbook_learners(sheet)?;
    let learner_count = learners
        .iter()
        .filter(|learner| crate::sf2::logic::is_learner_name(&learner.name))
        .count();
    let male_count = learners
        .iter()
        .filter(|learner| {
            learner.gender_block.as_deref() == Some("MALE")
                && crate::sf2::logic::is_learner_name(&learner.name)
        })
        .count();
    let female_count = learners
        .iter()
        .filter(|learner| {
            learner.gender_block.as_deref() == Some("FEMALE")
                && crate::sf2::logic::is_learner_name(&learner.name)
        })
        .count();
    let total_day_cells = sf2_total_day_cell_count(sheet)?;

    Ok(Sf2SheetQuality {
        total_day_cells,
        learner_count,
        male_count,
        female_count,
    })
}

fn sf2_total_day_cell_count(sheet: &ComObject) -> Result<usize> {
    let mut count = 0usize;
    for row in [29, 49] {
        for column in 6..=38 {
            if !cell_text(sheet, row, column)?.trim().is_empty() {
                count += 1;
            }
        }
    }
    Ok(count)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sf2SheetQuality {
    pub total_day_cells: usize,
    pub learner_count: usize,
    pub male_count: usize,
    pub female_count: usize,
}
