# Excel Import Test Data Directory

This directory contains sample Excel files for testing the student import functionality.

## File Structure

```
assets/
└── student-list/
    ├── SF1_2025_Grade-3-MATAPAT-1.xls
    ├── SF1_2025_Grade-4-MATAPAT-1.xlsx
    └── README.md
```

## SF1 Format

The SF1 (School Form 1) format is the standard format for student enrollment data in Philippine schools. Our Excel import feature is designed to handle this format out of the box.

### Expected Columns

- **LRN** - Learner Reference Number (optional but recommended)
- **Last Name** - Student's last name (required)
- **First Name** - Student's first name (required)
- **Middle Name** - Student's middle name (optional)
- **Gender** - Student's gender (optional)
- **Birthday** - Student's birth date (optional)
- **Age** - Student's age (optional)
- **Mother Name** - Mother's full name (optional)
- **Father Name** - Father's full name (optional)
- **Guardian Name** - Guardian's full name (optional)
- **Address** - Student's address (optional)

### Supported Formats

- **XLS** - Older Excel format (.xls files)
- **XLSX** - Modern Excel format (.xlsx files)

### Import Process

1. Open the Students page in the application
2. Click the "Import from Excel" button
3. Select your SF1 formatted Excel file
4. Choose the class to import to (optional)
5. Review the import results
6. Fix any errors if present

### Error Handling

The import feature handles various error scenarios:
- Missing required fields (First Name, Last Name)
- Invalid file formats
- Empty rows
- Malformed data
- Duplicate student IDs

### Testing

To test the import functionality:

1. Use the provided sample files in this directory
2. Or create your own test file following the SF1 format
3. Verify that data is correctly imported into the system
4. Check the import results for any errors

### Sample Data

The provided sample files contain realistic student data for testing purposes, including:
- Multiple students per file
- Various data completeness levels (some fields missing)
- Different data formats and edge cases

## Best Practices

1. **Clean Your Data**: Ensure your Excel file doesn't have extra formatting, merged cells, or empty rows
2. **Verify Columns**: Make sure column headers match the expected format
3. **Check Duplicates**: Remove duplicate students to avoid conflicts
4. **Backup Data**: Always backup your current data before importing
5. **Test First**: Import a small sample first to verify everything works correctly

## Troubleshooting

### Common Issues

1. **"Cannot identify name columns"**
   - Solution: Make sure your header row contains clear column names like "First Name" and "Last Name"

2. **"Unsupported file format"**
   - Solution: Ensure your file is either .xls or .xlsx format

3. **Import fails for specific rows** 
   - Solution: Check those rows for missing required fields or invalid data

4. **Performance issues with large files**
   - Solution: Split large files (1000+ students) into smaller chunks