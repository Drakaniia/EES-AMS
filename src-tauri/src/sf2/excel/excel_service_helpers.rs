use crate::domain::error::Result;
use crate::infrastructure::database::{DbPool, StudentRepository};
use crate::sf2::models::{Sf2StudentMappingRecord, Sf2TemplateRecord};
use crate::sf2::repository::Sf2Repository;
use std::collections::HashSet;

pub(super) fn latest_template_for_request(
    sf2_repo: &Sf2Repository,
    class_id: Option<&str>,
) -> Result<Option<Sf2TemplateRecord>> {
    if let Some(class_id) = class_id.filter(|value| !value.is_empty()) {
        return sf2_repo.latest_template_for_class(class_id);
    }

    Ok(sf2_repo
        .list_templates()?
        .into_iter()
        .next()
        .map(|summary| Sf2TemplateRecord {
            id: summary.id,
            source_path: summary.source_path,
            source_hash: String::new(),
            school_id: summary.school_id,
            school_name: summary.school_name,
            school_year: summary.school_year,
            report_month: summary.report_month,
            grade_level: summary.grade_level,
            section: summary.section,
            adviser_name: summary.adviser_name,
            school_head_name: summary.school_head_name,
            layout_fingerprint: String::new(),
            active_class_id: summary.class_id,
            imported_at: summary.imported_at,
            last_synced_at: None,
        }))
}

pub(super) fn unmapped_roster_student_names(
    pool: DbPool,
    template: &Sf2TemplateRecord,
    student_mappings: &[Sf2StudentMappingRecord],
) -> Result<Vec<String>> {
    let mapped_student_ids = student_mappings
        .iter()
        .map(|mapping| mapping.student_id.as_str())
        .collect::<HashSet<_>>();
    let students = StudentRepository::new(pool).list_by_class(Some(&template.active_class_id))?;

    Ok(students
        .into_iter()
        .filter(|student| !mapped_student_ids.contains(student.id.to_string().as_str()))
        .map(|student| student.name)
        .collect())
}

pub(super) fn unmapped_roster_issue(student_names: &[String]) -> String {
    let shown = student_names
        .iter()
        .take(5)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let more = student_names.len().saturating_sub(5);
    let suffix = if more > 0 {
        format!(", and {more} more")
    } else {
        String::new()
    };

    format!(
        "{}{} {} not mapped to an SF2 learner row. Sync the SF2 workbook roster before exporting.",
        shown,
        suffix,
        if student_names.len() == 1 {
            "is"
        } else {
            "are"
        }
    )
}
