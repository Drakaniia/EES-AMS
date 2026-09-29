use crate::sf2::month::first_school_day::{current_year, report_year_for_school_month};
use crate::sf2::workbook_files::{
    legacy_workbook_file_name, month_workbook_file_name, month_workbook_path,
    month_workbook_sheet_name, school_year_month_files, sf2_legacy_workbook_dir,
    write_month_template_to_dir, BUNDLED_TEMPLATE_BYTES, LEGACY_WORKBOOK_DIR,
};
use std::path::Path;

#[test]
fn a_month_file_is_named_after_its_month_and_calendar_year() {
    assert_eq!(
        month_workbook_file_name("SEPTEMBER", 2026),
        "SF2-SEPTEMBER-2026.xls"
    );
    assert_eq!(
        month_workbook_file_name("AUGUST", 2027),
        "SF2-AUGUST-2027.xls"
    );
    assert_eq!(
        month_workbook_file_name("JANUARY", 2027),
        "SF2-JANUARY-2027.xls"
    );
}

#[test]
fn the_month_in_a_file_name_is_uppercased_and_canonicalised() {
    // The bundled template calls September "SEPT."; a row written by an older
    // build may hold anything. All of them are the same file.
    for spelling in ["SEPTEMBER", "september", "Sept.", "SEPT.", "sept"] {
        assert_eq!(
            month_workbook_file_name(spelling, 2026),
            "SF2-SEPTEMBER-2026.xls",
            "spelling {spelling}"
        );
    }
    assert_eq!(
        month_workbook_file_name("december", 2026),
        "SF2-DECEMBER-2026.xls"
    );
}

#[test]
fn a_month_file_holds_exactly_one_sheet_named_for_its_month() {
    // The sheet name is what the analysis looks for, and the acceptance
    // criterion is one worksheet per file.
    assert_eq!(
        month_workbook_sheet_name("SEPTEMBER", 2026),
        "SEPTEMBER 2026"
    );
    assert_eq!(month_workbook_sheet_name("SEPT.", 2026), "SEPTEMBER 2026");
    assert_eq!(month_workbook_sheet_name("AUGUST", 2027), "AUGUST 2027");
}

#[test]
fn a_month_file_resolves_under_the_workbook_directory() {
    let dir = Path::new("sf2-workbooks");
    assert_eq!(
        month_workbook_path(dir, "SEPTEMBER", 2026),
        dir.join("SF2-SEPTEMBER-2026.xls")
    );
    // A month row can exist before its file does, so this resolves a path
    // whether or not anything is there yet (edge case E4).
    assert!(!month_workbook_path(dir, "APRIL", 2027).exists());
}

#[test]
fn the_legacy_file_stays_resolvable() {
    // The split job and any pre-split install still have to find the original
    // per-class workbook by this name.
    assert_eq!(
        legacy_workbook_file_name("1a2b3c4d-9e8f", "1", "A"),
        "SF2-1-A-1a2b3c4d.xls"
    );
    // Only the first eight characters of the template id are used, and a grade
    // or section that sanitises away falls back rather than producing "SF2--".
    assert_eq!(
        legacy_workbook_file_name("1a2b3c4d-9e8f", "  ", ""),
        "SF2-GRADE-SECTION-1a2b3c4d.xls"
    );
    assert_eq!(
        legacy_workbook_file_name("", "3", "MATAPAT"),
        "SF2-3-MATAPAT-.xls"
    );
}

#[test]
fn the_legacy_file_lives_in_its_own_subfolder() {
    let dir = Path::new("sf2-workbooks");
    assert_eq!(sf2_legacy_workbook_dir(dir), dir.join(LEGACY_WORKBOOK_DIR));
    // Backup snapshots the whole `sf2-workbooks` tree recursively, so the
    // subfolder is picked up with no special case anywhere.
    assert_eq!(LEGACY_WORKBOOK_DIR, "_legacy");
}

#[test]
fn a_school_year_is_twelve_month_files_from_september_to_august() {
    let files = school_year_month_files("2026-2027", 1999);

    assert_eq!(files.len(), 12, "one file per month, twelve months");
    assert_eq!(files[0], ("SEPTEMBER".to_string(), 2026));
    assert_eq!(files[3], ("DECEMBER".to_string(), 2026));
    assert_eq!(files[4], ("JANUARY".to_string(), 2027));
    assert_eq!(files[11], ("AUGUST".to_string(), 2027));

    // The file names are the twelve the spec lists, in order.
    let names = files
        .iter()
        .map(|(month, year)| month_workbook_file_name(month, *year))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "SF2-SEPTEMBER-2026.xls",
            "SF2-OCTOBER-2026.xls",
            "SF2-NOVEMBER-2026.xls",
            "SF2-DECEMBER-2026.xls",
            "SF2-JANUARY-2027.xls",
            "SF2-FEBRUARY-2027.xls",
            "SF2-MARCH-2027.xls",
            "SF2-APRIL-2027.xls",
            "SF2-MAY-2027.xls",
            "SF2-JUNE-2027.xls",
            "SF2-JULY-2027.xls",
            "SF2-AUGUST-2027.xls",
        ]
    );
}

#[test]
fn the_year_assignment_is_pure_and_wraps_at_august() {
    // This is the function the v22 backfill mirrors in SQL, so both the Rust
    // and the SQL side are pinned to the same boundary.
    for month in 9..=12 {
        assert_eq!(
            report_year_for_school_month("2026-2027", month, 1999),
            2026,
            "month {month} belongs to the start year"
        );
    }
    for month in 1..=8 {
        assert_eq!(
            report_year_for_school_month("2026-2027", month, 1999),
            2027,
            "month {month} belongs to the following calendar year"
        );
    }
    // A different school year moves every file with it.
    for month in 9..=12 {
        assert_eq!(report_year_for_school_month("2027-2028", month, 1999), 2027);
    }
    for month in 1..=8 {
        assert_eq!(report_year_for_school_month("2027-2028", month, 1999), 2028);
    }
    // The fallback is only used when the label holds no year, and
    // `current_year` is the value the app passes for that case.
    assert_eq!(
        report_year_for_school_month("unreadable", 9, current_year()),
        current_year()
    );
}

#[test]
fn writing_a_month_file_writes_a_fresh_copy_of_the_bundled_template() {
    let dir = tempfile::tempdir().expect("temp dir for the month workbook");
    let path =
        write_month_template_to_dir(dir.path(), "SEPTEMBER", 2026).expect("write month file");

    assert_eq!(path, dir.path().join("SF2-SEPTEMBER-2026.xls"));
    assert!(path.exists(), "the month file must exist on disk");
    assert_eq!(
        std::fs::metadata(&path).expect("stat").len(),
        BUNDLED_TEMPLATE_BYTES.len() as u64,
        "a month file starts as a copy of the bundled template"
    );

    // Re-running overwrites from scratch, which is what makes a retried split
    // safe (edge case E12).
    std::fs::write(&path, b"clobbered").expect("clobber the file");
    write_month_template_to_dir(dir.path(), "SEPTEMBER", 2026).expect("rewrite month file");
    assert_eq!(
        std::fs::metadata(&path).expect("stat").len(),
        BUNDLED_TEMPLATE_BYTES.len() as u64
    );
}
