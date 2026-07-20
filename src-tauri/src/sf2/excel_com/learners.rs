use crate::domain::error::Result;
use crate::sf2::excel_com::workbook::ComObject;
use crate::sf2::excel_com::worksheet::cell_text;
use crate::sf2::models::Sf2WorkbookLearner;

/// Parse learner names from an SF2 worksheet.
pub fn workbook_learners(sheet: &ComObject) -> Result<Vec<Sf2WorkbookLearner>> {
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
            learners.push(Sf2WorkbookLearner {
                row_index: row as u32,
                name,
                gender_block: gender_block.clone(),
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
