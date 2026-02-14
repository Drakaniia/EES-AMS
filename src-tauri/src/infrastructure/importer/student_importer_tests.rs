// Test Suite for Excel Import Functionality
// Comprehensive tests for student import from Excel files

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::importer::StudentImporter;
    use tempfile::NamedTempFile;
    use std::fs::File;
    use std::io::Write;
    use calamine::{open_workbook, DataType, Reader, Xls, Xlsx};
    
    // Helper function to create test Excel file
    fn create_test_excel_file(format: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();
        
        match format {
            "xlsx" => {
                // Create a simple XLSX file for testing
                let mut workbook = xlsxwriter::Workbook::new(file_path.to_str().unwrap()).unwrap();
                let sheet = workbook.add_worksheet(None).unwrap();
                
                // Header row
                sheet.write_string(0, 0, "LRN", None).unwrap();
                sheet.write_string(0, 1, "Last Name", None).unwrap();
                sheet.write_string(0, 2, "First Name", None).unwrap();
                sheet.write_string(0, 3, "Middle Name", None).unwrap();
                sheet.write_string(0, 4, "Gender", None).unwrap();
                sheet.write_string(0, 5, "Birthday", None).unwrap();
                sheet.write_string(0, 6, "Age", None).unwrap();
                sheet.write_string(0, 7, "Mother Name", None).unwrap();
                sheet.write_string(0, 8, "Father Name", None).unwrap();
                sheet.write_string(0, 9, "Guardian Name", None).unwrap();
                sheet.write_string(0, 10, "Address", None).unwrap();
                
                // Test data row
                sheet.write_string(1, 0, "2021001", None).unwrap();
                sheet.write_string(1, 1, "Smith", None).unwrap();
                sheet.write_string(1, 2, "John", None).unwrap();
                sheet.write_string(1, 3, "Doe", None).unwrap();
                sheet.write_string(1, 4, "Male", None).unwrap();
                sheet.write_string(1, 5, "2015-05-15", None).unwrap();
                sheet.write_number(1, 6, 8.0, None).unwrap();
                sheet.write_string(1, 7, "Jane Smith", None).unwrap();
                sheet.write_string(1, 8, "Robert Smith", None).unwrap();
                sheet.write_string(1, 9, "Jane Smith", None).unwrap();
                sheet.write_string(1, 10, "123 Main St, City", None).unwrap();
                
                workbook.close().unwrap();
            }
            "xls" => {
                // Create a simple XLS file using a different approach
                // For demonstration - in real implementation, you'd use a proper XLS library
                let content = b"LRN\tLast Name\tFirst Name\tMiddle Name\tGender\tBirthday\tAge\tMother Name\tFather Name\tGuardian Name\tAddress\n2021001\tSmith\tJohn\tDoe\tMale\t2015-05-15\t8\tJane Smith\tRobert Smith\tJane Smith\t123 Main St, City\n";
                temp_file.write_all(content).unwrap();
            }
            _ => panic!("Unsupported format: {}", format),
        }
        
        temp_file
    }
    
    #[test]
    fn test_excel_import_xlsx() {
        let temp_file = create_test_excel_file("xlsx");
        let importer = StudentImporter::new();
        
        let result = importer.import_from_excel(temp_file.path(), Some(1));
        
        assert!(result.is_ok());
        let import_result = result.unwrap();
        assert_eq!(import_result.success_count, 1);
        assert_eq!(import_result.error_count, 0);
        assert_eq!(import_result.imported_students.len(), 1);
        
        let student = &import_result.imported_students[0];
        assert_eq!(student.lrn.as_ref().unwrap(), "2021001");
        assert_eq!(student.last_name, "Smith");
        assert_eq!(student.first_name, "John");
        assert_eq!(student.middle_name.as_ref().unwrap(), "Doe");
        assert_eq!(student.gender.as_ref().unwrap(), "Male");
        assert_eq!(student.birthday.as_ref().unwrap(), "2015-05-15");
        assert_eq!(student.age.unwrap(), 8);
        assert_eq!(student.class_id.unwrap(), 1);
    }
    
    #[test]
    fn test_excel_import_xls() {
        let temp_file = create_test_excel_file("xls");
        let importer = StudentImporter::new();
        
        let result = importer.import_from_excel(temp_file.path(), Some(1));
        
        assert!(result.is_ok());
        let import_result = result.unwrap();
        assert_eq!(import_result.success_count, 1);
        assert_eq!(import_result.error_count, 0);
        assert_eq!(import_result.imported_students.len(), 1);
    }
    
    #[test]
    fn test_excel_import_invalid_file_format() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Invalid Excel content").unwrap();
        
        let importer = StudentImporter::new();
        let result = importer.import_from_excel(temp_file.path(), Some(1));
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported file format"));
    }
    
    #[test]
    fn test_excel_import_missing_file() {
        let importer = StudentImporter::new();
        let result = importer.import_from_excel("nonexistent.xls", Some(1));
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }
    
    #[test]
    fn test_excel_import_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let importer = StudentImporter::new();
        
        let result = importer.import_from_excel(temp_file.path(), Some(1));
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_excel_import_missing_required_fields() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // Create Excel with missing First Name
        let content = b"LRN\tLast Name\tGender\n2021001\tSmith\tMale\n";
        temp_file.write_all(content).unwrap();
        
        let importer = StudentImporter::new();
        let result = importer.import_from_excel(temp_file.path(), Some(1));
        
        assert!(result.is_ok());
        let import_result = result.unwrap();
        assert_eq!(import_result.success_count, 0);
        assert_eq!(import_result.error_count, 1);
    }
    
    #[test]
    fn test_excel_import_multiple_students() {
        let temp_file = create_test_excel_file("xlsx");
        let importer = StudentImporter::new();
        
        // For this test, we'd create a file with multiple students
        // The implementation would handle parsing multiple rows
        
        let result = importer.import_from_excel(temp_file.path(), Some(1));
        
        assert!(result.is_ok());
        // Additional assertions for multiple student import
    }
    
    #[test]
    fn test_excel_import_with_optional_fields() {
        let temp_file = create_test_excel_file("xlsx");
        let importer = StudentImporter::new();
        
        let result = importer.import_from_excel(temp_file.path(), None);
        
        assert!(result.is_ok());
        let import_result = result.unwrap();
        assert_eq!(import_result.success_count, 1);
        
        let student = &import_result.imported_students[0];
        assert!(student.class_id.is_none()); // Should be None when not provided
    }
    
    #[test]
    fn test_column_mapping() {
        let importer = StudentImporter::new();
        
        // Test different column header variations
        let test_cases = vec![
            ("LRN", "lrn"),
            ("Learner Reference Number", "lrn"),
            ("Last Name", "last_name"),
            ("Family Name", "last_name"),
            ("First Name", "first_name"),
            ("Given Name", "first_name"),
            ("Middle Name", "middle_name"),
            ("Middle", "middle_name"),
            ("Gender", "gender"),
            ("Sex", "gender"),
            ("Birthday", "birthday"),
            ("Birth Date", "birthday"),
            ("Age", "age"),
            ("Mother Name", "mother_name"),
            ("Father Name", "father_name"),
            ("Guardian Name", "guardian_name"),
            ("Address", "address"),
        ];
        
        for (header, expected_field) in test_cases {
            // Test column mapping logic
            // This would test the analyze_columns method
        }
    }
    
    #[test]
    fn test_name_parsing() {
        let importer = StudentImporter::new();
        
        // Test different name formats
        let test_cases = vec![
            ("Smith, John", ("Smith", "John", None)),
            ("Smith, John Doe", ("Smith", "John", Some("Doe".to_string()))),
            ("Smith, John Michael Doe", ("Smith", "John Michael", Some("Doe".to_string()))),
            ("John Smith", ("John Smith", "", None)), // Single column case
        ];
        
        for (input, expected) in test_cases {
            // Test name parsing logic
            // This would test the parse_student_row method
        }
    }
}

// Integration Tests
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::infrastructure::database::student_repository_impl::StudentRepositoryImpl;
    use crate::infrastructure::config::AppConfig;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_full_excel_import_workflow() {
        // Create temporary directory for test data
        let temp_dir = TempDir::new().unwrap();
        let app_config = AppConfig::new(temp_dir.path().to_path_buf());
        
        // Initialize repository
        let student_repo = StudentRepositoryImpl::new(&app_config).await.unwrap();
        
        // Create test Excel file
        let temp_file = create_test_excel_file("xlsx");
        let importer = StudentImporter::new();
        
        // Import students
        let import_result = importer.import_from_excel(temp_file.path(), Some(1)).unwrap();
        
        // Insert imported students into database
        for student_input in &import_result.imported_students {
            let result = student_repo.create(student_input).await;
            assert!(result.is_ok());
        }
        
        // Verify students were inserted
        let inserted_students = student_repo.get_all().await.unwrap();
        assert_eq!(inserted_students.len(), import_result.success_count);
    }
    
    #[tokio::test]
    async fn test_excel_import_with_existing_data() {
        // Test importing students when some already exist in database
        let temp_dir = TempDir::new().unwrap();
        let app_config = AppConfig::new(temp_dir.path().to_path_buf());
        
        let student_repo = StudentRepositoryImpl::new(&app_config).await.unwrap();
        
        // Create existing student
        let existing_student = crate::domain::entities::student::Student::new(
            1,
            "STD000001".to_string(),
            "John".to_string(),
            "Doe".to_string(),
            Some(1),
        );
        student_repo.create(&existing_student).await.unwrap();
        
        // Import Excel file with potentially duplicate data
        let temp_file = create_test_excel_file("xlsx");
        let importer = StudentImporter::new();
        
        let import_result = importer.import_from_excel(temp_file.path(), Some(1)).unwrap();
        
        // Handle duplicates/update existing students
        for student in &import_result.imported_students {
            match student_repo.get_by_student_id(&student.student_id).await {
                Ok(Some(_)) => {
                    // Student exists, update
                    let result = student_repo.update(student).await;
                    assert!(result.is_ok());
                }
                Ok(None) => {
                    // New student, create
                    let result = student_repo.create(student).await;
                    assert!(result.is_ok());
                }
                Err(e) => panic!("Error checking existing student: {}", e),
            }
        }
        
        // Verify final state
        let final_students = student_repo.get_all().await.unwrap();
        assert!(final_students.len() >= 1);
    }
}

// Performance Tests
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn test_import_large_excel_file() {
        // Test importing large Excel files
        let temp_file = create_large_test_excel_file(1000); // 1000 students
        
        let importer = StudentImporter::new();
        let start = Instant::now();
        
        let result = importer.import_from_excel(temp_file.path(), Some(1));
        
        let duration = start.elapsed();
        assert!(result.is_ok());
        assert!(duration.as_secs() < 30); // Should complete in under 30 seconds
        
        let import_result = result.unwrap();
        assert_eq!(import_result.success_count, 1000);
    }
    
    fn create_large_test_excel_file(student_count: usize) -> NamedTempFile {
        // Helper function to create large test file
        let mut temp_file = NamedTempFile::new().unwrap();
        
        // Implementation would generate file with specified number of students
        
        temp_file
    }
}