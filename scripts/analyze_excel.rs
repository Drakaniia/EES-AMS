use calamine::{open_workbook, DataType, Reader, Xls};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "assets/student-list/SF1_2025_Grade-3-MATAPAT-1.xls";

    if !Path::new(file_path).exists() {
        eprintln!("Error: File not found at {}", file_path);
        return Ok(());
    }

    println!("Analyzing Excel file: {}", file_path);
    println!("==========================================");

    // Open the workbook
    let mut workbook: Xls<_> = open_workbook(file_path)?;

    // Get all worksheet names
    let sheet_names = workbook.sheet_names();
    println!("Sheets found: {:?}", sheet_names);
    println!();

    // Analyze each sheet
    for sheet_name in &sheet_names {
        println!("Analyzing sheet: {}", sheet_name);
        println!("{}", "─".repeat(50));

        if let Some(Ok(range)) = workbook.worksheet_range(sheet_name) {
            let rows = range.rows();
            let cols = range.columns();

            println!("Dimensions: {} rows × {} columns", rows.len(), cols.len());
            println!();

            // Print header row (first 10 rows for analysis)
            println!("First 10 rows of data:");
            println!();

            for (row_idx, row) in rows.iter().take(10).enumerate() {
                print!("Row {:2}: ", row_idx + 1);

                for (col_idx, cell) in row.iter().enumerate() {
                    match cell {
                        DataType::Empty => print!("            "),
                        DataType::String(s) => print!("{:12} ", &s[..s.len().min(12)]),
                        DataType::Float(f) => print!("{:12.2} ", f),
                        DataType::Int(i) => print!("{:12} ", i),
                        DataType::Error(e) => print!("ERROR:{:6} ", e),
                        DataType::Bool(b) => print!("{:12} ", b),
                    }
                }
                println!();
            }

            // Look for potential header patterns
            println!();
            println!("Column Analysis:");
            for (col_idx, col) in cols.iter().take(20).enumerate() {
                let col_letter = (b'A' + col_idx as u8) as char;

                // Find non-empty cells in this column
                let non_empty_cells: Vec<_> = col
                    .iter()
                    .enumerate()
                    .filter_map(|(row_idx, cell)| {
                        if !cell.is_empty() {
                            Some((row_idx, cell))
                        } else {
                            None
                        }
                    })
                    .take(5)
                    .collect();

                if !non_empty_cells.is_empty() {
                    println!("Column {} ({})", col_letter, col_idx + 1);
                    for (row_idx, cell) in non_empty_cells {
                        print!("  Row {:2}: ", row_idx + 1);
                        match cell {
                            DataType::String(s) => println!("{}", s),
                            DataType::Float(f) => println!("{}", f),
                            DataType::Int(i) => println!("{}", i),
                            DataType::Bool(b) => println!("{}", b),
                            _ => println!("{:?}", cell),
                        }
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}
