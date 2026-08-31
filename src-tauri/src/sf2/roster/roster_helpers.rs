use crate::domain::error::Result;
use crate::domain::models::{Class, CreateClassRequest, Settings};
use crate::infrastructure::database::ClassRepository;
use crate::sf2::logic::Sf2CellMark;
use crate::sf2::models::Sf2WorkbookAnalysis;
use std::collections::HashSet;

/// The standard column letters for the SF2 learner info section (learner number, LRN, learner name).
const SF2_LEARNER_INFO_COLUMNS: [&str; 3] = ["A", "B", "C"];

/// Generate marks to clear unused learner rows (columns A, B, C) for all visible sheets.
/// Used after roster sync to remove leftover/empty learner rows in the learner range.
pub(crate) fn clear_unused_learner_marks(
    analysis: &Sf2WorkbookAnalysis,
    mapped_rows: &[u32],
    expanded_male_count: Option<usize>,
    expanded_female_count: Option<usize>,
) -> Vec<Sf2CellMark> {
    let sheet_names: Vec<String> = analysis
        .sheets
        .iter()
        .filter(|sheet| sheet.visible != 0)
        .map(|sheet| sheet.name.clone())
        .collect();

    let all_possible_rows = if let (Some(male_count), Some(female_count)) =
        (expanded_male_count, expanded_female_count)
    {
        let extra_male = male_count.saturating_sub(21) as u32;
        let mut rows = Vec::new();
        rows.extend(8u32..(8 + male_count as u32));
        let female_start = 30 + extra_male;
        rows.extend(female_start..(female_start + female_count as u32));
        rows
    } else {
        let mut rows = Vec::new();
        rows.extend(8u32..=28);
        rows.extend(30u32..=48);
        rows
    };

    let mapped: HashSet<u32> = mapped_rows.iter().copied().collect();
    let unused_rows: Vec<&u32> = all_possible_rows
        .iter()
        .filter(|r| !mapped.contains(r))
        .collect();

    if unused_rows.is_empty() {
        return Vec::new();
    }

    let mut marks =
        Vec::with_capacity(sheet_names.len() * unused_rows.len() * SF2_LEARNER_INFO_COLUMNS.len());
    for sheet_name in sheet_names {
        for col in &SF2_LEARNER_INFO_COLUMNS {
            for row in &unused_rows {
                marks.push(Sf2CellMark {
                    sheet_name: sheet_name.clone(),
                    cell_address: format!("{col}{row}"),
                    value: String::new(),
                });
            }
        }
    }
    marks
}

/// Find a class by name, or create it if it doesn't exist.
pub(crate) fn find_or_create_class(
    class_repo: &ClassRepository,
    class_name: &str,
    settings: Option<&Settings>,
) -> Result<Class> {
    if let Some(existing) = class_repo
        .list()?
        .into_iter()
        .find(|class: &Class| class.name.eq_ignore_ascii_case(class_name))
    {
        return Ok(existing);
    }

    let day_start = settings
        .map(|s| s.day_start.clone())
        .unwrap_or_else(|| "08:00".to_string());
    let day_end = settings
        .map(|s| s.day_end.clone())
        .unwrap_or_else(|| "15:00".to_string());
    let late_after = settings
        .map(|s| s.late_after.clone())
        .unwrap_or_else(|| "08:45".to_string());

    class_repo.create(CreateClassRequest {
        name: class_name.to_string(),
        room: Some("N/A".to_string()),
        day_start,
        day_end,
        late_after,
        sessions: Vec::new(),
        days: vec![1, 2, 3, 4, 5],
    })
}
