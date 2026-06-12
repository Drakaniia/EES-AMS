use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2WorkbookAnalysis {
    pub file_format: i32,
    pub has_vb_project: bool,
    pub school_id: String,
    pub school_name: String,
    pub school_year: String,
    pub report_month: String,
    pub grade_level: String,
    pub section: String,
    pub adviser_name: String,
    pub school_head_name: String,
    pub learners: Vec<Sf2WorkbookLearner>,
    pub dates: Vec<Sf2WorkbookDate>,
    pub sheets: Vec<Sf2WorkbookSheet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2WorkbookLearner {
    pub row_index: u32,
    pub name: String,
    pub gender_block: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2WorkbookDate {
    pub sheet_name: String,
    pub date: String,
    pub column_letter: String,
    pub column_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2WorkbookSheet {
    pub name: String,
    pub visible: i32,
    pub used_range: String,
}

#[derive(Debug, Clone)]
pub struct Sf2TemplateRecord {
    pub id: String,
    pub source_path: String,
    pub source_hash: String,
    pub school_id: String,
    pub school_name: String,
    pub school_year: String,
    pub report_month: String,
    pub grade_level: String,
    pub section: String,
    pub adviser_name: String,
    pub school_head_name: String,
    pub layout_fingerprint: String,
    pub active_class_id: String,
    pub imported_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2TemplateSummary {
    pub id: String,
    pub source_path: String,
    pub school_id: String,
    pub school_name: String,
    pub school_year: String,
    pub report_month: String,
    pub grade_level: String,
    pub section: String,
    pub adviser_name: String,
    pub school_head_name: String,
    pub class_id: String,
    pub imported_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2WorkbookSettings {
    pub template_id: String,
    pub class_id: String,
    pub class_name: String,
    pub source_path: String,
    pub school_id: String,
    pub school_name: String,
    pub school_year: String,
    pub report_month: String,
    pub grade_level: String,
    pub section: String,
    pub adviser_name: String,
    pub school_head_name: String,
    pub first_school_day: u32,
    pub learner_names: Vec<String>,
    pub dates_mapped: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2WorkbookMetadata {
    #[serde(default)]
    pub school_id: String,
    #[serde(default)]
    pub school_name: String,
    #[serde(default)]
    pub school_year: String,
    #[serde(default)]
    pub report_month: String,
    #[serde(default)]
    pub grade_level: String,
    #[serde(default)]
    pub section: String,
    #[serde(default)]
    pub adviser_name: String,
    #[serde(default)]
    pub school_head_name: String,
    #[serde(default)]
    pub configure_calendar: bool,
    #[serde(default)]
    pub first_school_day: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2TemplateDraft {
    #[serde(default)]
    pub class_id: Option<String>,
    #[serde(default)]
    pub school_id: String,
    #[serde(default)]
    pub school_name: String,
    #[serde(default)]
    pub school_year: String,
    #[serde(default)]
    pub report_month: String,
    #[serde(default)]
    pub grade_level: String,
    #[serde(default)]
    pub section: String,
    #[serde(default)]
    pub adviser_name: String,
    #[serde(default)]
    pub school_head_name: String,
    #[serde(default)]
    pub first_school_day: Option<u32>,
    #[serde(default)]
    pub learner_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Sf2StudentMappingRecord {
    pub template_id: String,
    pub student_id: String,
    pub workbook_name: String,
    pub normalized_name: String,
    pub row_index: u32,
    pub gender_block: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Sf2DateMappingRecord {
    pub template_id: String,
    pub sheet_name: String,
    pub date: String,
    pub column_letter: String,
    pub column_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2ImportSummary {
    pub template_id: String,
    pub class_id: String,
    pub class_name: String,
    pub source_path: String,
    pub school_year: String,
    pub grade_level: String,
    pub section: String,
    pub learners_found: usize,
    pub students_created: usize,
    pub students_reused: usize,
    pub dates_mapped: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2ImportValidation {
    pub source_path: String,
    pub class_id: Option<String>,
    pub class_name: String,
    pub current_student_count: usize,
    pub sf2_learner_count: usize,
    pub missing_from_sf2: Vec<Sf2ValidationStudent>,
    pub missing_from_current: Vec<Sf2ValidationLearner>,
    pub possible_name_mismatches: Vec<Sf2ValidationNameMismatch>,
    pub duplicate_current_students: Vec<Sf2ValidationDuplicate>,
    pub duplicate_sf2_learners: Vec<Sf2ValidationDuplicate>,
    pub missing_learner_info: Vec<Sf2ValidationLearner>,
    pub has_discrepancies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2ValidationStudent {
    pub student_id: String,
    pub name: String,
    pub normalized_name: String,
    pub gender: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2ValidationLearner {
    pub row_index: u32,
    pub name: String,
    pub normalized_name: String,
    pub gender_block: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2ValidationNameMismatch {
    pub current_student: Sf2ValidationStudent,
    pub sf2_learner: Sf2ValidationLearner,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2ValidationDuplicate {
    pub normalized_name: String,
    pub names: Vec<String>,
    pub student_ids: Vec<String>,
    pub row_indexes: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2CloseDaySummary {
    pub class_id: String,
    pub date: String,
    pub present_count: usize,
    pub absent_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2ExportReadiness {
    pub template: Option<Sf2TemplateSummary>,
    pub mapped_students: usize,
    pub mapped_dates: usize,
    pub can_export: bool,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2ExportPreview {
    pub template: Option<Sf2TemplateSummary>,
    pub class_id: Option<String>,
    pub class_name: String,
    pub source_path: Option<String>,
    pub dates: Vec<Sf2PreviewDate>,
    pub students: Vec<Sf2PreviewStudentRow>,
    pub absent_list: Vec<Sf2PreviewAbsence>,
    pub mapped_students: usize,
    pub mapped_dates: usize,
    pub present_count: usize,
    pub absence_count: usize,
    pub unmapped_student_count: usize,
    pub can_export: bool,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2PreviewDate {
    pub date: String,
    pub sheet_name: String,
    pub column_letter: String,
    pub column_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2PreviewStudentRow {
    pub student_id: String,
    pub student_name: String,
    pub workbook_name: String,
    pub gender: Option<String>,
    pub row_index: u32,
    pub mapped: bool,
    pub present_count: usize,
    pub absent_count: usize,
    pub warnings: Vec<String>,
    pub cells: Vec<Sf2PreviewCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2PreviewCell {
    pub date: String,
    pub status: Sf2PreviewCellStatus,
    pub editable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sf2PreviewCellStatus {
    Present,
    Absent,
    Open,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2PreviewAbsence {
    pub student_id: String,
    pub student_name: String,
    pub date: String,
    pub row_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2ExportResult {
    pub output_path: String,
    pub marks_written: usize,
}
