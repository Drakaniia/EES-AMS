//! Runs the diagnostic against a real install, and proves the run wrote nothing.
//!
//! Ignored by default, because it needs a real database and real workbooks -
//! and because a test that reaches into a user's data has to be something
//! somebody deliberately asks for. Run it with:
//!
//! ```text
//! EES_AMS_DIAG_DB=%APPDATA%\com.ees.ams\attendance.db \
//!   cargo test --lib real_install -- --ignored --nocapture
//! ```
//!
//! ## What it checks
//!
//! The database file's bytes and modification time, and the same for every
//! `.xls` in the workbook directory, before and after a full run. A write to
//! either moves at least one of them, so this is the check the hermetic tests in
//! `diagnose_tests.rs` cannot make: they have no workbook to open.
//!
//! It then prints the whole diagnostic as JSON, because the numbers are the
//! deliverable and reading them out of a log line by eye is how a count gets
//! misread.

use crate::sf2::diagnose::diagnose_sf2_marks;
use std::fs;
use std::path::{Path, PathBuf};

/// Bytes and modification time, the pair a write cannot leave both of.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Fingerprint(u64, u64, std::time::SystemTime);

fn fingerprint(path: &Path) -> Fingerprint {
    let bytes = fs::read(path).expect("the file is readable");
    let length = bytes.len() as u64;
    let modified = fs::metadata(path)
        .expect("the file is stat-able")
        .modified()
        .expect("the file has a modification time");
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in &bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Fingerprint(length, hash, modified)
}

/// Every `.xls` beside the database, and in the workbook folder, with a
/// fingerprint of each.
fn fingerprint_workbooks(db_path: &Path) -> Vec<(PathBuf, Fingerprint)> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for dir in [
        db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
        db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("sf2-workbooks"),
    ] {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(std::result::Result::ok) {
            let path = entry.path();
            if path.is_file() {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .filter(|path| {
            fs::metadata(path)
                .map(|meta| meta.len() > 0)
                .unwrap_or(false)
        })
        .map(|path| {
            let print = fingerprint(&path);
            (path, print)
        })
        .collect()
}

#[test]
#[ignore = "needs a real install; run it deliberately with EES_AMS_DIAG_DB set"]
fn real_install_is_measured_without_being_written_to() {
    let Ok(db_path) = std::env::var("EES_AMS_DIAG_DB") else {
        panic!(
            "set EES_AMS_DIAG_DB to the live attendance.db, e.g. \
             %APPDATA%\\com.ees.ams\\attendance.db"
        );
    };
    let db_path = PathBuf::from(db_path);
    assert!(db_path.is_file(), "{} is not a file", db_path.display());

    let db_before = fingerprint(&db_path);
    let workbooks_before = fingerprint_workbooks(&db_path);

    let diagnostic =
        diagnose_sf2_marks(&db_path).expect("the diagnostic runs against a real install");

    let db_after = fingerprint(&db_path);
    let workbooks_after = fingerprint_workbooks(&db_path);

    assert_eq!(
        db_before,
        db_after,
        "the diagnostic changed {}",
        db_path.display()
    );
    assert_eq!(
        workbooks_before, workbooks_after,
        "the diagnostic changed a workbook"
    );

    println!(
        "\n===== SF2 MARK DIAGNOSTIC (read-only) =====\ndatabase : {}\nschema   : {:?}\nworkbook : {:?}\ndir      : {}\nclass    : {:?}\nyear     : {:?}\nmappings : {}\nverdict  : {}\n{}\nevents   : {:?}\nabsent   : {}\nabsent not in this class: {}\nrecorded fingerprint: {:?}\ndecrease notice: {:?}\n",
        diagnostic.database_path,
        diagnostic.schema_version,
        diagnostic.workbook_path,
        diagnostic.workbook_dir,
        diagnostic.active_class_id,
        diagnostic.school_year,
        diagnostic.mapping_source,
        diagnostic.verdict,
        diagnostic.verdict_reason,
        diagnostic.event_counts,
        diagnostic.total_absent_events,
        diagnostic.absent_events_without_class,
        diagnostic.recorded_fingerprint,
        diagnostic.fingerprint_decrease_notice,
    );

    println!("--- mapping tables ---");
    for table in &diagnostic.tables.legacy {
        println!(
            "  legacy   {:32} exists={} rows={:?}",
            table.table, table.exists, table.rows
        );
    }
    for table in &diagnostic.tables.per_month {
        println!(
            "  per-month{:32} exists={} rows={:?}",
            table.table, table.exists, table.rows
        );
    }
    for grid in &diagnostic.tables.legacy_date_mapping_sheets {
        println!(
            "  legacy grid   sheet={:?} month={} {}..{} columns={}",
            grid.sheet_name, grid.year_month, grid.first_date, grid.last_date, grid.day_columns
        );
    }
    for grid in &diagnostic.tables.month_date_mapping_grids {
        println!(
            "  month grid    month={} {}..{} columns={}",
            grid.year_month, grid.first_date, grid.last_date, grid.day_columns
        );
    }

    println!("\n--- workbooks ---");
    for workbook in &diagnostic.workbooks {
        println!(
            "  {}\n    referenced={} sheets={} month_sheets_measured={} total_x={} error={:?}",
            workbook.path,
            workbook.is_referenced_by_database,
            workbook.sheet_count,
            workbook.month_sheets_measured,
            workbook.total_x_count,
            workbook.read_error
        );
    }

    println!("\n--- months ---");
    for month in &diagnostic.months {
        println!(
            "  {:10} {}  status={:<17} workbook_x={:?} db_absent={:?} db_mapped={:?} scanned={:?} \
             roster={} cols={} sheet={:?} file={:?} mappings={} roster_via={}\n      only_in_workbook={} only_in_database={}\n      {}",
            month.report_month,
            month.report_year,
            month.source_status,
            month.counts.workbook_x_count,
            month.counts.db_absent_count,
            month.counts.db_mapped_absent_count,
            month.counts.cells_scanned,
            month.roster_rows,
            month.day_columns,
            month.sheet_name,
            month.workbook_path,
            month.mapping_source,
            month.roster_resolution,
            month.cells_only_in_workbook.len(),
            month.cells_only_in_database.len(),
            month.reason,
        );
        for cell in &month.cells_only_in_workbook {
            println!(
                "        MISSING FROM DB  {}  {}  {}!{}",
                cell.student_name, cell.date, cell.sheet_name, cell.cell_address
            );
        }
        for cell in &month.cells_only_in_database {
            println!(
                "        MISSING FROM XLS  {}  {}  {}!{}",
                cell.student_name, cell.date, cell.sheet_name, cell.cell_address
            );
        }
    }

    println!("\n--- unplaced worksheets ---");
    for sheet in &diagnostic.unplaced_sheets {
        println!(
            "  {}  {}  visible={} x={}\n      {}",
            sheet.sheet_name,
            sheet.workbook_path,
            sheet.visible,
            sheet.unrowed_x_count,
            sheet.reason
        );
        for cell in &sheet.marks {
            println!(
                "        {}  {}!{}",
                cell.student_name, cell.sheet_name, cell.cell_address
            );
        }
    }

    println!("\n===== END DIAGNOSTIC =====\n");
}
