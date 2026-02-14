// Excel Import Module
// Handles importing student data from Excel files (SF1 format)

use crate::application::handlers::student_handler::{CreateStudentFromSF1Input, ImportResult};
use crate::domain::entities::student::Student;
use anyhow::Result;
use calamine::{open_workbook, Data, Reader, Xls, Xlsx, Range};
use std::path::Path;

pub struct StudentImporter;

impl StudentImporter {
    pub fn new() -> Self {
        Self
    }

    /// Import students from an Excel file (SF1 format)
    pub fn import_from_excel<P: AsRef<Path>>(
        &self,
        file_path: P,
        class_id: Option<i64>,
    ) -> Result<ImportResult> {
        let file_path = file_path.as_ref();

        if !file_path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", file_path.display()));
        }

        // Try to determine file type and open appropriately
        let file_ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let range: Range<Data>;

        match file_ext {
            "xlsx" => {
                let mut workbook: Xlsx<_> = open_workbook(file_path)
                    .map_err(|e| anyhow::anyhow!("Failed to open XLSX workbook: {}", e))?;
                let sheet_names = workbook.sheet_names().to_vec();
                if sheet_names.is_empty() {
                    return Err(anyhow::anyhow!("No worksheets found in the workbook"));
                }
                let sheet_name = &sheet_names[0];
                range = workbook
                    .worksheet_range(sheet_name)
                    .map_err(|e| anyhow::anyhow!("Cannot read worksheet {}: {}", sheet_name, e))?;
            }
            "xls" => {
                let mut workbook: Xls<_> = open_workbook(file_path)
                    .map_err(|e| anyhow::anyhow!("Failed to open XLS workbook: {}", e))?;
                let sheet_names = workbook.sheet_names().to_vec();
                if sheet_names.is_empty() {
                    return Err(anyhow::anyhow!("No worksheets found in the workbook"));
                }
                let sheet_name = &sheet_names[0];
                range = workbook
                    .worksheet_range(sheet_name)
                    .map_err(|e| anyhow::anyhow!("Cannot read worksheet {}: {}", sheet_name, e))?;
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported file format. Only .xls and .xlsx files are supported."
                ))
            }
        }

        let mut imported_students = Vec::new();
        let mut errors = Vec::new();
        let mut success_count = 0;

        // Analyze the worksheet structure
        let column_mapping = self.analyze_columns(&range)?;

        // Process data rows (skip header rows)
        for (row_idx, row) in range.rows().enumerate() {
            // Skip first row (header)
            if row_idx == 0 {
                continue;
            }

            // Skip empty rows
            if row.iter().all(|cell| matches!(cell, Data::Empty)) {
                continue;
            }

            match self.parse_student_row(row, &column_mapping, class_id) {
                Ok(student_input) => {
                    // Create student from parsed data
                    let student = Student::new_from_sf1(
                        0, // Will be assigned by database
                        student_input.lrn,
                        student_input.last_name,
                        student_input.first_name,
                        student_input.middle_name,
                        student_input.gender,
                        student_input.birthday,
                        student_input.age,
                        student_input.mother_name,
                        student_input.father_name,
                        student_input.guardian_name,
                        student_input.address,
                        student_input.class_id,
                    );
                    imported_students.push(student);
                    success_count += 1;
                }
                Err(e) => {
                    errors.push(format!("Row {}: {}", row_idx + 1, e));
                }
            }
        }

        Ok(ImportResult {
            success_count,
            error_count: errors.len(),
            errors,
            imported_students,
        })
    }

    /// Analyze columns to identify which column contains what data
    fn analyze_columns(
        &self,
        range: &Range<Data>,
    ) -> Result<ColumnMapping> {
        let mut mapping = ColumnMapping::default();

        // Look at the first few rows to identify column types
        let sample_rows: Vec<_> = range.rows().take(5).collect();

        if sample_rows.is_empty() {
            return Err(anyhow::anyhow!("Empty worksheet"));
        }

        // Try to identify columns by header names in first row
        if let Some(header_row) = sample_rows.first() {
            for (col_idx, cell) in header_row.iter().enumerate() {
                if let Data::String(header_text) = cell {
                    let header = header_text.to_lowercase();
                    let header = header.trim();

                    // Match column headers to fields
                    if header.contains("lrn") || header.contains("learner") {
                        mapping.lrn = Some(col_idx);
                    } else if header.contains("last") && header.contains("name") {
                        mapping.last_name = Some(col_idx);
                    } else if header.contains("first") && header.contains("name") {
                        mapping.first_name = Some(col_idx);
                    } else if header.contains("middle") && header.contains("name") {
                        mapping.middle_name = Some(col_idx);
                    } else if header.contains("gender") || header.contains("sex") {
                        mapping.gender = Some(col_idx);
                    } else if header.contains("birth") || header.contains("birthday") {
                        mapping.birthday = Some(col_idx);
                    } else if header.contains("age") {
                        mapping.age = Some(col_idx);
                    } else if header.contains("mother") {
                        mapping.mother_name = Some(col_idx);
                    } else if header.contains("father") {
                        mapping.father_name = Some(col_idx);
                    } else if header.contains("guardian") {
                        mapping.guardian_name = Some(col_idx);
                    } else if header.contains("address") {
                        mapping.address = Some(col_idx);
                    }
                }
            }
        }

        // If some required fields are missing, try to infer by content
        if mapping.last_name.is_none() || mapping.first_name.is_none() {
            self.infer_name_columns(&sample_rows, &mut mapping)?;
        }

        // Validate required fields
        if mapping.last_name.is_none() || mapping.first_name.is_none() {
            return Err(anyhow::anyhow!(
                "Cannot identify name columns in the Excel file"
            ));
        }

        Ok(mapping)
    }

    /// Infer name columns by analyzing data patterns
    fn infer_name_columns(
        &self,
        sample_rows: &[&[Data]],
        mapping: &mut ColumnMapping,
    ) -> Result<()> {
        // Only infer if there are rows with data
        if !sample_rows.is_empty() {
            for (col_idx, column_cells) in sample_rows[0].iter().enumerate() {
                if let Data::String(text) = column_cells {
                    // Check if this looks like a "Last, First" format
                    if text.contains(',') && mapping.last_name.is_none() {
                        mapping.last_name = Some(col_idx);
                    } else if !text.contains(',') && mapping.first_name.is_none() && text.len() > 2
                    {
                        mapping.first_name = Some(col_idx);
                    }
                }
            }
        }
        Ok(())
    }

    /// Parse a single row into student input
    fn parse_student_row(
        &self,
        row: &[Data],
        mapping: &ColumnMapping,
        class_id: Option<i64>,
    ) -> Result<CreateStudentFromSF1Input> {
        let get_string = |col_idx: Option<usize>| -> Option<String> {
            col_idx
                .and_then(|idx| row.get(idx))
                .and_then(|cell| match cell {
                    Data::String(s) => Some(s.trim().to_string()),
                    Data::Float(f) => Some(f.to_string()),
                    Data::Int(i) => Some(i.to_string()),
                    _ => None,
                })
                .filter(|s: &String| !s.is_empty())
        };

        let get_i32 = |col_idx: Option<usize>| -> Option<i32> {
            col_idx
                .and_then(|idx| row.get(idx))
                .and_then(|cell| match cell {
                    Data::Int(i) => Some(*i as i32),
                    Data::Float(f) => Some(*f as i32),
                    Data::String(s) => s.trim().parse().ok(),
                    _ => None,
                })
        };

        // Parse name from "Last, First Middle" format if needed
        let (last_name, first_name, middle_name) =
            if let (Some(_name_col_idx), Some(combined_name)) =
                (mapping.last_name, get_string(mapping.last_name))
            {
                if combined_name.contains(',') {
                    // Split "Last, First Middle" into components
                    let parts: Vec<&str> = combined_name.splitn(3, ',').collect();
                    let last = parts[0].trim().to_string();
                    let first_part = parts.get(1).unwrap_or(&"").trim();
                    let middle = parts
                        .get(2)
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string());

                    // Further split first part if it contains middle name
                    if let Some(space_pos) = first_part.rfind(' ') {
                        let first = first_part[..space_pos].trim().to_string();
                        let middle_from_first = first_part[space_pos + 1..].trim().to_string();
                        (last, first, Some(middle_from_first))
                    } else {
                        (last, first_part.to_string(), middle.map(|s| s.to_string()))
                    }
                } else {
                    // Single column with full name
                    (
                        combined_name,
                        String::new(),
                        get_string(mapping.middle_name),
                    )
                }
            } else {
                // Separate columns for first and last name
                (
                    get_string(mapping.last_name)
                        .ok_or_else(|| anyhow::anyhow!("Last name is required"))?,
                    get_string(mapping.first_name)
                        .ok_or_else(|| anyhow::anyhow!("First name is required"))?,
                    get_string(mapping.middle_name),
                )
            };

        Ok(CreateStudentFromSF1Input {
            lrn: get_string(mapping.lrn),
            last_name,
            first_name,
            middle_name,
            gender: get_string(mapping.gender),
            birthday: get_string(mapping.birthday),
            age: get_i32(mapping.age),
            mother_name: get_string(mapping.mother_name),
            father_name: get_string(mapping.father_name),
            guardian_name: get_string(mapping.guardian_name),
            address: get_string(mapping.address),
            class_id,
        })
    }
}

#[derive(Debug, Default)]
struct ColumnMapping {
    lrn: Option<usize>,
    last_name: Option<usize>,
    first_name: Option<usize>,
    middle_name: Option<usize>,
    gender: Option<usize>,
    birthday: Option<usize>,
    age: Option<usize>,
    mother_name: Option<usize>,
    father_name: Option<usize>,
    guardian_name: Option<usize>,
    address: Option<usize>,
}
